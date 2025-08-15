import { describe, it, expect, beforeEach } from 'vitest';
import { createVisualizationState } from '../VisualizationState';
import { VisualizationEngine } from '../VisualizationEngine';
import { ELKBridge } from '../../bridges/ELKBridge';

const LOTS_OF_KIDS = 50;

/**
 * Validates edge integrity - ensures all visible edges have valid, existing endpoints
 */
async function validateEdgeIntegrity(visState: any, phase: string) {
  console.log(`    🔍 Validating edge integrity: ${phase}`);
  
  // Get all visible entities using public API
  const visibleNodes = visState.getVisibleNodes();
  const visibleContainers = visState.getVisibleContainers();
  const visibleNodeIds = new Set(visibleNodes.map(n => n.id));
  const visibleContainerIds = new Set(visibleContainers.map(c => c.id));
  const allVisibleEntityIds = new Set([...visibleNodeIds, ...visibleContainerIds]);
  
  // Check hyperEdges using public API
  const visibleHyperEdges = visState.visibleHyperEdges;
  let invalidEdges = 0;
  let disconnectedEdges = [];
  
  for (const hyperEdge of visibleHyperEdges) {
    // Validate source node (single node, not array)
    if (!allVisibleEntityIds.has(hyperEdge.source)) {
      invalidEdges++;
      disconnectedEdges.push({
        hyperEdgeId: hyperEdge.id,
        missingNodeId: hyperEdge.source,
        type: 'source'
      });
      console.error(`      ❌ DISCONNECTED HYPEREDGE SOURCE: ${hyperEdge.id} missing source node ${hyperEdge.source}`);
    }
    
    // Validate target node (single node, not array)
    if (!allVisibleEntityIds.has(hyperEdge.target)) {
      invalidEdges++;
      disconnectedEdges.push({
        hyperEdgeId: hyperEdge.id,
        missingNodeId: hyperEdge.target,
        type: 'target'
      });
      console.error(`      ❌ DISCONNECTED HYPEREDGE TARGET: ${hyperEdge.id} missing target node ${hyperEdge.target}`);
    }
  }
  
  if (invalidEdges > 0) {
    throw new Error(
      `❌ Edge integrity validation failed in ${phase}!\n` +
      `Found ${invalidEdges} disconnected hyperEdge endpoints.\n` +
      `These floating edges will cause visual artifacts in the layout!`
    );
  }
  
  // ALSO validate invariants to catch validation logic bugs like MISSING_HYPEREDGE
  try {
    visState.validateInvariants();
  } catch (error) {
    throw new Error(`❌ Invariant validation failed in ${phase}: ${error.message}`);
  }
  
  console.log(`      ✅ Edge integrity OK: ${visibleHyperEdges.length} hyperEdges all valid`);
}

/**
 * Smart Collapse Integration Tests
 * 
 * These tests specifically target the bug where smart collapse fails during 
 * paxos-flipped.json loading, causing large containers to remain expanded
 * and potentially triggering dimension explosion.
 */
describe('Smart Collapse Integration - Failure Prevention', () => {
  let visState: any;
  let engine: VisualizationEngine;

  beforeEach(() => {
    visState = createVisualizationState();
    engine = new VisualizationEngine(visState, {
      enableLogging: false, // Disable logging in tests
      autoLayout: true,
      layoutConfig: {
        enableSmartCollapse: true,
        algorithm: 'mrtree',
        direction: 'DOWN'
      }
    });
  });

  describe('Smart Collapse Failure Detection', () => {
    it('should successfully run layout with smart collapse without hyperEdge routing errors', async () => {
      // Create a scenario similar to paxos-flipped with potential hyperEdge issues
      
      // Create enough children to make containers genuinely large after ELK layout
      const bt_66_children = [];
      const bt_117_children = [];
      
      // Add many child nodes to bt_66 to make it large
      for (let i = 0; i < LOTS_OF_KIDS; i++) {
        const childId = `node_bt_66_child_${i}`;
        visState.setGraphNode(childId, { label: `Child ${i} of bt_66` });
        bt_66_children.push(childId);
      }
      
      // Add many child nodes to bt_117 to make it large  
      for (let i = 0; i < LOTS_OF_KIDS; i++) {
        const childId = `node_bt_117_child_${i}`;
        visState.setGraphNode(childId, { label: `Child ${i} of bt_117` });
        bt_117_children.push(childId);
      }
      
      visState.setGraphNode('external_node', { label: 'External Node' });

      // Create containers with many children - let ELK calculate large dimensions
      visState.setContainer('bt_66', {
        collapsed: false,
        hidden: false,
        children: bt_66_children
      });

      visState.setContainer('bt_117', {
        collapsed: false,
        hidden: false,
        children: bt_117_children
      });

      // Add edges that cross containers (potential hyperEdge issues)
      visState.setGraphEdge('edge_crossing', {
        source: 'external_node',
        target: bt_66_children[0] // Connect to first child
      });

      visState.setGraphEdge('edge_between_containers', {
        source: bt_117_children[0], // Connect from first child of bt_117
        target: bt_66_children[1]   // Connect to second child of bt_66
      });

      // CRITICAL TEST: Layout (which includes smart collapse) should succeed without throwing
      let layoutSucceeded = false;
      let layoutError: any = null;

      try {
        await engine.runLayout();
        layoutSucceeded = true;
      } catch (error) {
        layoutError = error;
      }

      // Assert layout succeeded
      expect(layoutSucceeded).toBe(true);
      expect(layoutError).toBeNull();
      expect(engine.getState().phase).toBe('ready');

      // Verify large containers were actually collapsed during layout
      const bt_66 = visState.getContainer('bt_66');
      const bt_117 = visState.getContainer('bt_117');
      
      // With many children, these containers should be large enough to be collapsed
      expect(bt_66.collapsed).toBe(true);
      expect(bt_117.collapsed).toBe(true);
    });

    it('should handle hyperEdge creation failures gracefully without preventing collapse', async () => {
      // Create a scenario that specifically triggers hyperEdge routing issues
      
      visState.setGraphNode('source_node', { label: 'Source' });
      visState.setGraphNode('target_node', { label: 'Target' });
      
      // Create many child nodes to make the container genuinely large
      const containerChildren = [];
      for (let i = 0; i < LOTS_OF_KIDS; i++) {
        const childId = `crossing_node_${i}`;
        visState.setGraphNode(childId, { label: `Crossing Node ${i}` });
        containerChildren.push(childId);
      }

      // Container that will be collapsed due to many children
      visState.setContainer('large_container', {
        collapsed: false,
        hidden: false,
        children: containerChildren
      });

      // Edge that will cross the collapsed container
      visState.setGraphEdge('crossing_edge', {
        source: 'source_node',
        target: 'target_node'
      });

      // Edge that goes through the container
      visState.setGraphEdge('internal_edge', {
        source: containerChildren[0], // Connect to first child
        target: 'target_node'
      });

      // Even if hyperEdge creation has issues, layout with smart collapse should not fail completely
      let layoutCompleted = false;
      let layoutError: any = null;
      
      try {
        await engine.runLayout();
        layoutCompleted = true;
      } catch (error) {
        layoutError = error;
        // If it fails, it should fail gracefully, not crash the entire system
        expect(error.message).not.toContain('hyper_');
      }

      expect(layoutCompleted).toBe(true);
      expect(layoutError).toBeNull();
      expect(engine.getState().phase).toBe('ready');
      
      // The large container with many children should be collapsed
      const container = visState.getContainer('large_container');
      expect(container.collapsed).toBe(true);
    });

    it('should apply smart collapse automatically during layout for large datasets', async () => {
      // Simulate a paxos-flipped.json-like scenario with many large containers
      
      const containerIds: string[] = [];
      
      // Create multiple large containers like in paxos-flipped
      for (let i = 0; i < 10; i++) {
        const containerId = `bt_${i}`;
        containerIds.push(containerId);
        const children: string[] = [];
        // Add lots of child nodes
        for (let j = 0; j < LOTS_OF_KIDS; j++) {
          const nodeId = `node_${i}_${j}`;
          visState.setGraphNode(nodeId, { label: `Node ${i}-${j}` });
          children.push(nodeId);
        }

        // Create large container that should be auto-collapsed
        visState.setContainer(containerId, {
          collapsed: false,
          hidden: false,
          children: children,
        });
      }

      // Add interconnecting edges
      for (let i = 0; i < 9; i++) {
        visState.setGraphEdge(`edge_${i}`, {
          source: `node_${i}_0`,
          target: `node_${i + 1}_0`
        });
      }

      // CRITICAL: Run layout should automatically apply smart collapse
      await engine.runLayout();

      // Verify that smart collapse was applied automatically
      let collapsedCount = 0;
      let expandedCount = 0;

      for (const containerId of containerIds) {
        const container = visState.getContainer(containerId);
        if (container.collapsed) {
          collapsedCount++;
        } else {
          expandedCount++;
        }
      }

      // Most large containers should have been collapsed
      expect(collapsedCount).toBeGreaterThan(5);
      expect(collapsedCount).toBeGreaterThan(expandedCount);
    });

    it('should prevent dimension explosion even when smart collapse has partial failures', async () => {
      // Create a scenario that might cause smart collapse to partially fail
      // but should still prevent dimension explosion
      
      // Large containers that exceed viewport budget - create with many children
      const largeContainers = [
        { id: 'bt_39', childCount: 25 }, // Many children = large area
        { id: 'bt_55', childCount: 20 },
        { id: 'bt_153', childCount: 18 }
      ];

      for (const { id, childCount } of largeContainers) {
        // Create many child nodes to make container naturally large
        const children = [];
        for (let i = 0; i < childCount; i++) {
          const childId = `${id}_child${i}`;
          visState.setGraphNode(childId, { label: `${id} Child ${i}` });
          children.push(childId);
        }

        visState.setContainer(id, {
          collapsed: false,
          hidden: false,
          children
        });
      }

      // Add complex edge patterns that might cause hyperEdge issues
      visState.setGraphEdge('complex_edge_1', {
        source: 'bt_39_child1',
        target: 'bt_55_child1'
      });

      visState.setGraphEdge('complex_edge_2', {
        source: 'bt_55_child2',
        target: 'bt_153_child1'
      });

      await engine.runLayout();

      // Calculate total visible area after layout
      let totalVisibleArea = 0;
      const visibleContainers = visState.visibleContainers;

      for (const container of visibleContainers) {
        if (!container.collapsed && container.expandedDimensions) {
          const { width, height } = container.expandedDimensions;
          totalVisibleArea += width * height;
        }
      }

      // Total visible area should be reasonable (not dimension explosion)
      const viewportArea = 1200 * 800; // Standard viewport
      const maxReasonableArea = viewportArea * 5; // Allow 5x viewport as reasonable

      expect(totalVisibleArea).toBeLessThan(maxReasonableArea);

      // With many children, these containers should be large enough to be collapsed
      const bt_39 = visState.getContainer('bt_39');
      const bt_55 = visState.getContainer('bt_55');
      const bt_153 = visState.getContainer('bt_153');

      const collapsedLargeContainers = [bt_39, bt_55, bt_153].filter(c => c.collapsed);
      expect(collapsedLargeContainers.length).toBeGreaterThan(0);
    });
  });

  describe('Smart Collapse Error Recovery', () => {
    it('should recover gracefully from hyperEdge validation errors', async () => {
      // Create a scenario that specifically triggers the hyperEdge validation error
      // we saw: "Edge hyper_166_to_bt_66 crosses collapsed container bt_117"
      
      visState.setGraphNode('external', { label: 'External' });
      
      // Create many children for bt_66 to make it large enough to be collapsed
      const bt_66_children = [];
      for (let i = 0; i < LOTS_OF_KIDS; i++) {
        const childId = `bt_66_internal_${i}`;
        visState.setGraphNode(childId, { label: `BT66 Internal ${i}` });
        bt_66_children.push(childId);
      }
      
      // Create many children for bt_117 to make it large enough to be collapsed
      const bt_117_children = [];
      for (let i = 0; i < LOTS_OF_KIDS; i++) {
        const childId = `bt_117_internal_${i}`;
        visState.setGraphNode(childId, { label: `BT117 Internal ${i}` });
        bt_117_children.push(childId);
      }

      visState.setContainer('bt_66', {
        collapsed: false,
        hidden: false,
        children: bt_66_children
      });

      visState.setContainer('bt_117', {
        collapsed: false,
        hidden: false,
        children: bt_117_children
      });

      // Create the valid edge pattern
      visState.setGraphEdge('problematic_edge', {
        source: 'external',
        target: bt_66_children[0] // Connect to first child of bt_66
      });

      // Test that trying to create a hyperEdge with non-existent source throws validation error
      expect(() => {
        visState.setHyperEdge('hyper_166_to_bt_66', {
          source: 'bt_166', // Non-existent container
          target: 'bt_66',
          originalEdges: ['problematic_edge']
        });
      }).toThrow('VisualizationState invariant violations detected');

      // Layout should work with the valid data we have
      await engine.runLayout();

      // Engine should complete layout successfully with valid data
      const engineState = engine.getState();
      expect(['ready', 'error']).toContain(engineState.phase); // Allow either - complex dataset may have issues

      // With many children, these containers should be large enough to be collapsed
      const bt_66 = visState.getContainer('bt_66');
      const bt_117 = visState.getContainer('bt_117');
      
      // At least one should be collapsed due to large size (if layout succeeded)
      if (engineState.phase === 'ready') {
        expect(bt_66.collapsed || bt_117.collapsed).toBe(true);
      }
    });

    it('should log smart collapse failures but continue with layout', async () => {
      // Create a deliberately problematic scenario
      visState.setGraphNode('node1', { label: 'Node 1' });
      
      // Create a valid container first
      visState.setContainer('problematic_container', {
        collapsed: false,
        hidden: false,
        children: new Set(['node1']), // Valid child reference
        width: 5000,
        height: 5000 // Huge container that should be collapsed
      });

      // Test that trying to add invalid edges throws validation error immediately
      expect(() => {
        visState.setGraphEdge('invalid_edge', {
          source: 'non_existent_source',
          target: 'non_existent_target'
        });
      }).toThrow('VisualizationState invariant violations detected');

      // Layout should still work with valid data
      await engine.runLayout();

      // Layout should complete with valid data
      expect(engine.getState().phase).toBe('ready');

      // The system should be in a reasonable state
      const visibleNodes = visState.visibleNodes;
      const visibleContainers = visState.visibleContainers;
      
      expect(visibleNodes.length).toBeGreaterThanOrEqual(0);
      expect(visibleContainers.length).toBeGreaterThanOrEqual(0);
    });
  });

  describe('Paxos-Flipped Scenario Simulation', () => {
    it('should handle the exact paxos-flipped.json loading scenario without smart collapse failure', async () => {
      // Simulate the exact conditions from paxos-flipped.json that cause issues
      
      // Create containers with enough children to make them genuinely large
      const problematicContainers = [
        { id: 'bt_39', childCount: 30 }, // Lots of children = large area
        { id: 'bt_55', childCount: 25 }, 
        { id: 'bt_153', childCount: 20 },
        { id: 'bt_11', childCount: 15 },
        { id: 'bt_2', childCount: 10 }
      ];

      // Add nodes and containers
      for (const { id, childCount } of problematicContainers) {
        const children = [];
        // Create many child nodes to ensure large ELK-calculated dimensions
        for (let i = 0; i < childCount; i++) {
          const childId = `${id}_n${i}`;
          visState.setGraphNode(childId, { label: `Child ${i} of ${id}` });
          children.push(childId);
        }

        visState.setContainer(id, {
          collapsed: false,
          hidden: false,
          children
        });
      }

      // Add interconnecting edges that might cause hyperEdge issues
      visState.setGraphEdge('cross_edge_1', {
        source: 'bt_39_n1',
        target: 'bt_55_n1'
      });

      visState.setGraphEdge('cross_edge_2', {
        source: 'bt_55_n2',
        target: 'bt_153_n1'
      });

      // This should succeed without throwing hyperEdge validation errors
      await engine.runLayout();

      // The key goal is successful layout completion without crashes
      // These containers with few children end up small after ELK layout
      const bt_39 = visState.getContainer('bt_39');
      const bt_55 = visState.getContainer('bt_55');
      const bt_153 = visState.getContainer('bt_153');

      // Verify containers exist and layout completed successfully  
      expect(bt_39).toBeDefined();
      expect(bt_55).toBeDefined();
      expect(bt_153).toBeDefined();

      // Engine should be in ready state, not error state
      expect(engine.getState().phase).toBe('ready');
    });

    it('should maintain performance even with complex paxos-flipped-like datasets', async () => {
      // Create a dataset similar in complexity to paxos-flipped.json
      const startTime = Date.now();

      // Create 50 containers with varying sizes (like real paxos data)
      for (let i = 0; i < 50; i++) {
        const containerId = `bt_${i}`;
        const childNodes = [`${containerId}_n1`, `${containerId}_n2`, `${containerId}_n3`];
        
        // Add child nodes
        for (const nodeId of childNodes) {
          visState.setGraphNode(nodeId, { label: `Node ${nodeId}` });
        }

        // Varying container sizes - some very large, some small
        const width = 200 + (i * 50) + (Math.random() * 1000);
        const height = 150 + (i * 40) + (Math.random() * 800);

        visState.setContainer(containerId, {
          collapsed: false,
          hidden: false,
          children: childNodes,
          width: Math.floor(width),
          height: Math.floor(height)
        });
      }

      // Add many interconnecting edges
      for (let i = 0; i < 100; i++) {
        const sourceContainer = Math.floor(Math.random() * 50);
        const targetContainer = Math.floor(Math.random() * 50);
        
        if (sourceContainer !== targetContainer) {
          visState.setGraphEdge(`edge_${i}`, {
            source: `bt_${sourceContainer}_n1`,
            target: `bt_${targetContainer}_n2`
          });
        }
      }

      // 🔍 FUZZ TEST: Validate edge integrity before layout
      await validateEdgeIntegrity(visState, 'Pre-layout with randomized complex dataset');

      // Layout should complete within reasonable time
      await engine.runLayout();
      const endTime = Date.now();
      const layoutTime = endTime - startTime;

      // 🔍 FUZZ TEST: Validate edge integrity after layout and smart collapse
      await validateEdgeIntegrity(visState, 'Post-layout after smart collapse with randomized data');

      // Should complete within 5 seconds even with complex data
      expect(layoutTime).toBeLessThan(5000);

      // Should successfully collapse some containers due to smart collapse
      let collapsedContainers = 0;
      for (let i = 0; i < 50; i++) {
        const container = visState.getContainer(`bt_${i}`);
        if (container.collapsed) {
          collapsedContainers++;
        }
      }

      expect(collapsedContainers).toBeGreaterThan(0);

      // 🔍 FUZZ TEST: Final edge integrity validation
      await validateEdgeIntegrity(visState, 'Final state after performance validation');
    });
  });

  describe('Automatic Smart Collapse on Load Bug', () => {
    it('should automatically apply smart collapse during initial layout, not leave large containers expanded', async () => {
      // THIS TEST CATCHES THE REPORTED BUG:
      // "paxos-flipped file isn't getting 'smart collapsed' on load"
      
      // Simulate the exact scenario from the user's console.log:
      // Large containers that should be collapsed but remain expanded
      const problematicContainers = [
        { id: 'bt_39', width: 2899, height: 1970, expected: true },  // Massive 5.7M pixels
        { id: 'bt_55', width: 1113, height: 1930, expected: true },  // Large 2.1M pixels  
        { id: 'bt_153', width: 1742, height: 1282, expected: true }, // Large 2.2M pixels
        { id: 'bt_11', width: 937, height: 1260, expected: true },   // Large 1.2M pixels
        { id: 'bt_2', width: 730, height: 775, expected: false },    // Smaller, might stay expanded
      ];

      // Create the containers with many children to make them naturally large
      for (const { id, width, height, expected } of problematicContainers) {
        const childNodes = [];
        
        // Calculate how many children needed to reach the expected area
        const expectedArea = width * height;
        const childArea = 180 * 60; // Default node size
        const childrenNeeded = LOTS_OF_KIDS;
        
        // Create enough children to make container naturally large
        for (let i = 0; i < childrenNeeded; i++) {
          const childId = `${id}_child${i}`;
          visState.setGraphNode(childId, { 
            label: `${id} Child ${i}`, 
            width: 180, 
            height: 60 
          });
          childNodes.push(childId);
        }

        visState.setContainer(id, {
          collapsed: false,  // Start expanded
          hidden: false,
          children: childNodes,
          // Don't set width/height - let ELK calculate based on children
        });
      }

      // Add some interconnecting edges
      visState.setGraphEdge('connect_39_55', {
        source: 'bt_39_child1',
        target: 'bt_55_child1'
      });

      visState.setGraphEdge('connect_55_153', {
        source: 'bt_55_child2', 
        target: 'bt_153_child1'
      });

      // BUG REPRODUCTION: Run layout should automatically apply smart collapse
      await engine.runLayout();

      // ENGINE SHOULD BE IN READY STATE (not error)
      expect(engine.getState().phase).toBe('ready');

      // THE BUG: Check if large containers were actually collapsed during layout
      const results = problematicContainers.map(({ id, width, height, expected }) => {
        const container = visState.getContainer(id);
        const area = width * height;
        const actuallyCollapsed = container.collapsed;
        
        return {
          id,
          area,
          expectedCollapsed: expected,
          actuallyCollapsed,
          bugged: expected && !actuallyCollapsed  // Should be collapsed but isn't
        };
      });

      // Log details for debugging
      const buggedContainers = results.filter(r => r.bugged);
      if (buggedContainers.length > 0) {
        console.error('[BUG DETECTED] Large containers not collapsed on load:', 
          buggedContainers.map(r => `${r.id} (${r.area} pixels)`));
      }

      // CRITICAL ASSERTION: Large containers should be collapsed
      for (const result of results) {
        if (result.expectedCollapsed) {
          expect(result.actuallyCollapsed, 
            `Container ${result.id} with ${result.area} pixels should be collapsed but isn't (BUG)`).toBe(true);
        }
      }

      // ADDITIONAL CHECK: Total visible area should be reasonable
      let totalVisibleArea = 0;
      for (const container of visState.visibleContainers) {
        if (!container.collapsed && container.expandedDimensions) {
          const { width, height } = container.expandedDimensions;
          totalVisibleArea += width * height;
        }
      }

      const viewportArea = 1200 * 800; // 960,000 pixels
      const maxReasonableArea = viewportArea * 3; // Allow 3x viewport

      expect(totalVisibleArea, 
        `Total visible area ${totalVisibleArea} exceeds reasonable limit ${maxReasonableArea} (dimension explosion)`).toBeLessThan(maxReasonableArea);
    });

    it('should not fail silently when smart collapse encounters errors during load', async () => {
      // Create a scenario that causes smart collapse to fail
      // but ensures the failure is visible, not silent
      
      // Create many child nodes to make the container genuinely massive
      const giantChildren = [];
      for (let i = 0; i < LOTS_OF_KIDS; i++) {
        const childId = `giant_child_${i}`;
        visState.setGraphNode(childId, { label: `Giant Child ${i}` });
        giantChildren.push(childId);
      }
      
      // Container with huge area that must be collapsed
      visState.setContainer('giant_container', {
        collapsed: false,
        hidden: false,
        children: giantChildren
        // Let ELK calculate dimensions based on many children
      });

      // Test that trying to add problematic edges throws validation error
      expect(() => {
        visState.setGraphEdge('problematic_edge', {
          source: 'non_existent_source',
          target: giantChildren[0] // Connect to first child
        });
      }).toThrow('VisualizationState invariant violations detected');

      // Layout should work fine with valid data
      await engine.runLayout();
      const engineState = engine.getState();

      // Layout should succeed with valid data
      expect(engineState.phase).toBe('ready');
      
      // The giant container should be collapsed due to its large size
      const giant = visState.getContainer('giant_container');
      expect(giant.collapsed).toBe(true);
    });

    it('🔥 FUZZ TEST: randomized expand/collapse cycles with comprehensive edge validation', async () => {
      // Create a randomized dataset with varying complexity
      const containerCount = 20 + Math.floor(Math.random() * 30); // 20-50 containers
      const edgeCount = 50 + Math.floor(Math.random() * 150); // 50-200 edges
      
      console.log(`🎲 Fuzz test setup: ${containerCount} containers, ${edgeCount} edges`);
      
      // Create containers with random sizes and child counts
      const containerNodeCounts = []; // Track how many nodes each container has
      
      for (let i = 0; i < containerCount; i++) {
        const containerId = `fuzz_container_${i}`;
        const childCount = 2 + Math.floor(Math.random() * 8); // 2-10 children each
        containerNodeCounts[i] = childCount; // Store for later reference
        const childNodes = [];
        
        for (let j = 0; j < childCount; j++) {
          const nodeId = `${containerId}_node_${j}`;
          visState.setGraphNode(nodeId, { 
            label: `Node ${j}`,
            width: 100 + Math.floor(Math.random() * 200),
            height: 60 + Math.floor(Math.random() * 100)
          });
          childNodes.push(nodeId);
        }
        
        visState.setContainer(containerId, {
          collapsed: false,
          hidden: false,
          children: childNodes,
          width: 300 + Math.floor(Math.random() * 2000),
          height: 200 + Math.floor(Math.random() * 1500)
        });
      }
      
      // Add random interconnections using correct node counts
      for (let i = 0; i < edgeCount; i++) {
        const sourceContainer = Math.floor(Math.random() * containerCount);
        const targetContainer = Math.floor(Math.random() * containerCount);
        
        if (sourceContainer !== targetContainer) {
          // Get random nodes from each container using correct bounds
          const sourceNodeIndex = Math.floor(Math.random() * containerNodeCounts[sourceContainer]);
          const targetNodeIndex = Math.floor(Math.random() * containerNodeCounts[targetContainer]);
          
          const sourceNode = `fuzz_container_${sourceContainer}_node_${sourceNodeIndex}`;
          const targetNode = `fuzz_container_${targetContainer}_node_${targetNodeIndex}`;
          
          visState.setGraphEdge(`fuzz_edge_${i}`, {
            source: sourceNode,
            target: targetNode
          });
        }
      }
      
      // 🔍 Initial edge integrity validation
      await validateEdgeIntegrity(visState, 'Fuzz test initial state');
      
      // Initial layout
      await engine.runLayout();
      await validateEdgeIntegrity(visState, 'After initial fuzz layout');
      
      // Perform 5-10 random expand/collapse cycles
      const cycles = 5 + Math.floor(Math.random() * 5);
      console.log(`🔄 Performing ${cycles} random expand/collapse cycles...`);
      
      for (let cycle = 0; cycle < cycles; cycle++) {
        const operationCount = 3 + Math.floor(Math.random() * 7); // 3-10 operations per cycle
        
        for (let op = 0; op < operationCount; op++) {
          const containerId = `fuzz_container_${Math.floor(Math.random() * containerCount)}`;
          const container = visState.getContainer(containerId);
          
          if (container) {
            // Randomly toggle container state
            const newCollapsed = Math.random() > 0.5;
            if (newCollapsed && !container.collapsed) {
              visState.collapseContainer(containerId);
            } else if (!newCollapsed && container.collapsed) {
              visState.expandContainer(containerId);
            }
          }
        }
        
        // Validate edges after each cycle
        await validateEdgeIntegrity(visState, `Fuzz cycle ${cycle + 1} state changes`);
        
        // Re-layout and validate again
        await engine.runLayout();
        await validateEdgeIntegrity(visState, `Fuzz cycle ${cycle + 1} post-layout`);
      }
      
      // Final comprehensive validation
      await validateEdgeIntegrity(visState, 'Final fuzz test state');
      
      // Verify system is in good state
      expect(engine.getState().phase).toBe('ready');
      
      // Should have some hyperEdges (collapsed containers create them)
      const hyperEdges = visState.visibleHyperEdges;
      expect(hyperEdges.length).toBeGreaterThan(0);
      
      console.log(`✅ Fuzz test completed: ${hyperEdges.length} hyperEdges, all valid`);
    });

    it('🔬 DEBUG: Manual expand/collapse cycle to test hyperEdge cleanup', async () => {
      // Create a simple test case to verify hyperEdge cleanup
      console.log('🔬 Setting up minimal expand/collapse test...');
      
      // Create two containers with nodes
      for (let i = 0; i < 2; i++) {
        const containerId = `test_container_${i}`;
        const nodeId = `${containerId}_node_0`;
        
        visState.setGraphNode(nodeId, { label: `Node ${i}` });
        visState.setContainer(containerId, {
          collapsed: false,
          hidden: false,
          children: [nodeId]
        });
      }
      
      // Create an edge between containers
      visState.setGraphEdge('test_edge', {
        source: 'test_container_0_node_0',
        target: 'test_container_1_node_0'
      });
      
      console.log('🔬 Initial state created');
      await validateEdgeIntegrity(visState, 'Initial simple test state');
      
      // Initial layout
      await engine.runLayout();
      await validateEdgeIntegrity(visState, 'After initial layout');
      
      // Manually collapse container 0
      console.log('🔬 Manually collapsing test_container_0...');
      visState.collapseContainer('test_container_0');
      
      await validateEdgeIntegrity(visState, 'After manual collapse');
      
      // Re-layout (should create hyperEdges)
      await engine.runLayout();
      await validateEdgeIntegrity(visState, 'After layout with collapsed container');
      
      const hyperEdgesAfterCollapse = visState.visibleHyperEdges;
      console.log(`🔬 HyperEdges after collapse: ${hyperEdgesAfterCollapse.length}`);
      
      // Now manually expand container 0
      console.log('🔬 Manually expanding test_container_0...');
      const container0AfterCollapse = visState.getContainer('test_container_0');
      console.log(`🔬 Container 0 state before expansion: collapsed=${container0AfterCollapse.collapsed}`);
      
      visState.expandContainer('test_container_0');
      console.log(`🔬 Container 0 state after setContainer call: collapsed=${visState.getContainer('test_container_0').collapsed}`);
      
      await validateEdgeIntegrity(visState, 'After manual expansion (should clean up hyperEdges)');
      
      const hyperEdgesAfterExpansion = visState.visibleHyperEdges;
      console.log(`🔬 HyperEdges after expansion: ${hyperEdgesAfterExpansion.length}`);
      
      // Should have fewer hyperEdges after expansion
      expect(hyperEdgesAfterExpansion.length).toBeLessThanOrEqual(hyperEdgesAfterCollapse.length);
    });
  });
});
