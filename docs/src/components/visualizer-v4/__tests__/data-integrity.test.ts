/**
 * Test to detect data integrity issues where containers appear in graphNodes collection
 * This prevents bugs like the spacing issue where ELKBridge couldn't distinguish containers from nodes
 */

import { VisualizationState } from '../core/VisualizationState';
import { parseGraphJSON } from '../core/JSONParser';

describe('Data Integrity: Container vs Node Collections', () => {
  // Use a simple test data structure with correct type definitions
  // The issue was: hierarchy children should reference node IDs, not create new containers
  const testData = {
    nodes: [
      { id: 'node1', data: { label: 'Node 1' } },
      { id: 'node2', data: { label: 'Node 2' } },
      { id: 'node3', data: { label: 'Node 3' } },
      { id: 'node4', data: { label: 'Node 4' } }
    ],
    edges: [
      { id: 'edge1', source: 'node1', target: 'node2' },
      { id: 'edge2', source: 'node3', target: 'node4' }
    ],
    hierarchyChoices: [
      {
        id: 'test-hierarchy',
        name: 'Test Hierarchy',
        children: [
          {
            id: 'container1',
            name: 'Container 1',
            // This is the correct format - no children means it's a leaf container
          },
          {
            id: 'container2', 
            name: 'Container 2',
            // This is the correct format - no children means it's a leaf container
          }
        ]
      }
    ]
  };

  test('containers should not exist in graphNodes collection', () => {
    // Parse the test data
    console.log('🔄 Parsing test data...');
    const parseResult = parseGraphJSON(testData);
    const visState = parseResult.state;
    
    console.log(`📊 Data loaded: ${parseResult.metadata.nodeCount} nodes, ${parseResult.metadata.containerCount} containers`);
    
    // Get all container IDs
    const containerIds = new Set<string>();
    
    // Collect container IDs from the state using internal API access
    const internalState = visState as any;
    const containers = internalState._collections.containers;
    
    for (const [containerId] of containers) {
      containerIds.add(containerId as string);
    }
    
    console.log(`🏗️ Found ${containerIds.size} containers in containers collection`);
    
    // Check if any container IDs also exist in graphNodes collection
    const graphNodes = internalState._collections.graphNodes;
    const duplicateIds: string[] = [];
    
    for (const containerId of containerIds) {
      if (graphNodes.has(containerId)) {
        duplicateIds.push(containerId);
      }
    }
    
    // Log results
    if (duplicateIds.length > 0) {
      console.error(`❌ Data integrity violation: ${duplicateIds.length} containers found in graphNodes collection:`);
      duplicateIds.forEach(id => {
        console.error(`  - ${id}`);
      });
      
      // Also log what type of data is in graphNodes for these IDs
      duplicateIds.slice(0, 3).forEach(id => {
        const nodeData = graphNodes.get(id);
        console.error(`  ${id} in graphNodes:`, {
          hasWidth: nodeData?.width !== undefined,
          hasHeight: nodeData?.height !== undefined
        });
      });
    } else {
      console.log('✅ Data integrity check passed: No containers found in graphNodes collection');
    }
    
    // Test assertion
    expect(duplicateIds).toHaveLength(0);
    
    // Additional check: Verify containers have proper structure
    const containerArray = Array.from(containers.values());
    const sampleContainer = containerArray[0] as any;
    
    if (sampleContainer) {
      console.log('📦 Sample container structure:', {
        id: sampleContainer.id,
        hasChildren: sampleContainer.children !== undefined,
        hasCollapsed: sampleContainer.collapsed !== undefined,
        hasLabel: sampleContainer.label !== undefined
      });
      
      // Containers should have children set and collapsed property
      expect(sampleContainer.children).toBeDefined();
      expect(typeof sampleContainer.collapsed).toBe('boolean');
    }
  });

  test('nodes should not exist in containers collection', () => {
    // Parse the test data
    const parseResult = parseGraphJSON(testData);
    const visState = parseResult.state;
    
    // Get all node IDs
    const nodeIds = new Set<string>();
    const internalState = visState as any;
    const graphNodes = internalState._collections.graphNodes;
    
    for (const [nodeId] of graphNodes) {
      nodeIds.add(nodeId as string);
    }
    
    console.log(`🔍 Found ${nodeIds.size} nodes in graphNodes collection`);
    
    // Check if any node IDs also exist in containers collection
    const containers = internalState._collections.containers;
    const duplicateIds: string[] = [];
    
    for (const nodeId of nodeIds) {
      if (containers.has(nodeId)) {
        duplicateIds.push(nodeId);
      }
    }
    
    // Log results
    if (duplicateIds.length > 0) {
      console.error(`❌ Data integrity violation: ${duplicateIds.length} nodes found in containers collection:`);
      duplicateIds.forEach(id => {
        console.error(`  - ${id}`);
      });
    } else {
      console.log('✅ Data integrity check passed: No nodes found in containers collection');
    }
    
    // Test assertion
    expect(duplicateIds).toHaveLength(0);
  });

  test('all entity IDs should be unique across collections', () => {
    // Parse the test data
    const parseResult = parseGraphJSON(testData);
    const visState = parseResult.state;
    const internalState = visState as any;
    
    // Get all IDs from each collection
    const nodeIds = new Set<string>();
    const containerIds = new Set<string>();
    const edgeIds = new Set<string>();
    
    for (const [id] of internalState._collections.graphNodes) {
      nodeIds.add(id as string);
    }
    
    for (const [id] of internalState._collections.containers) {
      containerIds.add(id as string);
    }
    
    for (const [id] of internalState._collections.graphEdges) {
      edgeIds.add(id as string);
    }
    
    console.log(`🔍 Entity counts: ${nodeIds.size} nodes, ${containerIds.size} containers, ${edgeIds.size} edges`);
    
    // Check for overlaps
    const allIds = new Set([...nodeIds, ...containerIds, ...edgeIds]);
    const totalExpected = nodeIds.size + containerIds.size + edgeIds.size;
    
    if (allIds.size !== totalExpected) {
      const overlap = totalExpected - allIds.size;
      console.error(`❌ Data integrity violation: ${overlap} overlapping IDs detected across collections`);
    } else {
      console.log('✅ Data integrity check passed: All entity IDs are unique across collections');
    }
    
    // Test assertion
    expect(allIds.size).toBe(totalExpected);
  });
});
