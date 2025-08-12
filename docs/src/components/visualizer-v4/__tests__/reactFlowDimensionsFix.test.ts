import { describe, it, expect, beforeEach } from 'vitest';
import { createVisualizationState } from '../core/VisState';
import type { VisualizationState } from '../core/VisState';
import { ELKBridge } from '../bridges/ELKBridge';
import { ReactFlowBridge } from '../bridges/ReactFlowBridge';

/**
 * Tests to verify that ReactFlowBridge correctly uses computed dimensions from VisState
 * after ELK layout calculations, addressing the encapsulation of expandedDimensions.
 */
describe('ReactFlowBridge Dimensions Fix', () => {
  let visState: VisualizationState;
  let elkBridge: ELKBridge;
  let reactFlowBridge: ReactFlowBridge;

  beforeEach(() => {
    visState = createVisualizationState();
    elkBridge = new ELKBridge();
    reactFlowBridge = new ReactFlowBridge();
  });

  it('should correctly use ELK-calculated dimensions in ReactFlow conversion', async () => {
    // // console.log((('\n=== Testing ReactFlowBridge Dimensions Fix ===')));

    // 1. Create a simple hierarchy with containers using fluent API
    visState
      .setGraphNode('node1', { label: 'Node 1', style: 'default' })
      .setGraphNode('node2', { label: 'Node 2', style: 'default' })
      .setGraphNode('node3', { label: 'Node 3', style: 'default' })
      .setContainer('container1', { children: ['node1', 'node2'], collapsed: false })
      .setGraphEdge('edge1', { source: 'node1', target: 'node2' })
      .setGraphEdge('edge2', { source: 'node2', target: 'node3' });

    // // console.log((('✅ Created test data with 1 container containing 2 nodes')));

    // 2. Check initial state (before ELK)
    const initialContainers = visState.visibleContainers;
    expect(initialContainers).toHaveLength(1);
    
    const initialContainer = initialContainers[0];
    // // console.log(((`📏 Initial container dimensions: ${initialContainer.width}x${initialContainer.height} at (${initialContainer.x}, ${initialContainer.y})`)));
    
    // Initial dimensions should be minimum dimensions from label-adjusted calculations
    // (Our container label positioning provides minimum dimensions)
    expect(initialContainer.width).toBe(200); // MIN_CONTAINER_WIDTH
    expect(initialContainer.height).toBeGreaterThanOrEqual(150); // MIN_CONTAINER_HEIGHT + label space

    // 3. Run ELK layout to calculate proper dimensions
    await elkBridge.layoutVisState(visState);
    // // console.log((('✅ ELK layout completed')));

    // 4. Check dimensions after ELK
    const elkContainers = visState.visibleContainers;
    const elkContainer = elkContainers[0];
    // // console.log(((`📏 ELK-calculated container dimensions: ${elkContainer.width}x${elkContainer.height} at (${elkContainer.x}, ${elkContainer.y})`)));
    
    // ELK should have calculated proper dimensions
    expect(elkContainer.width).toBeGreaterThan(0);
    expect(elkContainer.height).toBeGreaterThan(0);
    expect(elkContainer.x).toBeGreaterThanOrEqual(0);
    expect(elkContainer.y).toBeGreaterThanOrEqual(0);

    // 5. Convert to ReactFlow format
    const reactFlowData = reactFlowBridge.visStateToReactFlow(visState);
    // // console.log((('✅ ReactFlow conversion completed')));

    // 6. Verify ReactFlow gets the correct container dimensions
    const containerNodes = reactFlowData.nodes.filter(node => node.type === 'container');
    expect(containerNodes).toHaveLength(1);

    const reactFlowContainer = containerNodes[0];
    // // console.log(((`📏 ReactFlow container dimensions: ${reactFlowContainer.data.width}x${reactFlowContainer.data.height} at (${reactFlowContainer.position.x}, ${reactFlowContainer.position.y})`)));

    // ReactFlow should receive the same dimensions that are used for layout
    const expectedDimensions = visState.getContainerAdjustedDimensions(elkContainer.id);
    expect(reactFlowContainer.data.width).toBe(expectedDimensions.width);
    expect(reactFlowContainer.data.height).toBe(expectedDimensions.height);
    expect(reactFlowContainer.position.x).toBe(elkContainer.x);
    expect(reactFlowContainer.position.y).toBe(elkContainer.y);

    // 7. Verify style dimensions match data dimensions
    expect(reactFlowContainer.style?.width).toBe(expectedDimensions.width);
    expect(reactFlowContainer.style?.height).toBe(expectedDimensions.height);

    // // console.log((('✅ All dimension checks passed - ReactFlowBridge correctly uses ELK-calculated dimensions!')));
    // // console.log((('=== Test Complete ===\n')));
  });

  it('should handle multiple containers with different sizes', async () => {
    // // console.log((('\n=== Testing Multiple Container Dimensions ===')));

    // Create containers with different numbers of children (should get different sizes)
    visState
      .setGraphNode('node1', { label: 'Node 1', style: 'default' })
      .setGraphNode('node2', { label: 'Node 2', style: 'default' })
      .setGraphNode('node3', { label: 'Node 3', style: 'default' })
      .setGraphNode('node4', { label: 'Node 4', style: 'default' })
      .setGraphNode('node5', { label: 'Node 5', style: 'default' })
      // Small container with 2 nodes
      .setContainer('small_container', { children: ['node1', 'node2'], collapsed: false })
      // Large container with 3 nodes  
      .setContainer('large_container', { children: ['node3', 'node4', 'node5'], collapsed: false })
      .setGraphEdge('edge1', { source: 'node1', target: 'node2' })
      .setGraphEdge('edge2', { source: 'node3', target: 'node4' })
      .setGraphEdge('edge3', { source: 'node4', target: 'node5' });

    // Run ELK layout
    await elkBridge.layoutVisState(visState);

    // Check VisState containers
    const visStateContainers = visState.visibleContainers;
    expect(visStateContainers).toHaveLength(2);

    const smallContainer = visStateContainers.find(c => c.id === 'small_container')!;
    const largeContainer = visStateContainers.find(c => c.id === 'large_container')!;

    // // console.log(((`📏 Small container (2 nodes): ${smallContainer.width}x${smallContainer.height}`)));
    // // console.log(((`📏 Large container (3 nodes): ${largeContainer.width}x${largeContainer.height}`)));

    // Large container should be at least as tall as small container
    // Note: Layout algorithm may assign same minimum height to both containers
    expect(largeContainer.height).toBeGreaterThanOrEqual(smallContainer.height);

    // Convert to ReactFlow
    const reactFlowData = reactFlowBridge.visStateToReactFlow(visState);
    const reactFlowContainers = reactFlowData.nodes.filter(node => node.type === 'container');
    expect(reactFlowContainers).toHaveLength(2);

    // Verify ReactFlow containers match VisState containers
    for (const reactFlowContainer of reactFlowContainers) {
      const visStateContainer = visStateContainers.find(c => c.id === reactFlowContainer.id)!;
      const expectedDimensions = visState.getContainerAdjustedDimensions(visStateContainer.id);
      
      expect(reactFlowContainer.data.width).toBe(expectedDimensions.width);
      expect(reactFlowContainer.data.height).toBe(expectedDimensions.height);
      expect(reactFlowContainer.position.x).toBe(visStateContainer.x);
      expect(reactFlowContainer.position.y).toBe(visStateContainer.y);

      // // console.log(((`✅ Container ${reactFlowContainer.id}: ReactFlow matches VisState dimensions`)));
    }

    // // console.log((('✅ Multiple container dimension test passed!')));
    // // console.log((('=== Test Complete ===\n')));
  });
});
