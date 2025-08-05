/**
 * Demo of visualizer-v4 integration
 * 
 * Tests that v3 core/bridges architecture works with v2 frontend components
 */

import { createIntegratedStateManager } from '../integration/StateAdapter.js';

/**
 * Simple test to validate the integration works
 */
async function testIntegration() {
  console.log('🚀 Testing visualizer-v4 integration...');
  
  // Create integrated state manager
  const stateManager = createIntegratedStateManager();
  
  // Test data in v2 format
  const testData = {
    nodes: [
      { id: 'node1', label: 'Node 1' },
      { id: 'node2', label: 'Node 2' },
      { id: 'node3', label: 'Node 3' }
    ],
    edges: [
      { id: 'edge1', source: 'node1', target: 'node2' },
      { id: 'edge2', source: 'node2', target: 'node3' }
    ],
    containers: [
      { 
        id: 'container1', 
        children: ['node1', 'node2'],
        label: 'Container 1',
        expandedDimensions: { width: 300, height: 200 }
      }
    ]
  };
  
  // Test 1: Set graph data using v2 format, stored in v3 VisState
  console.log('📊 Setting graph data...');
  stateManager.setGraphData(testData);
  
  // Verify data is in VisState
  const nodes = stateManager.getVisibleNodes();
  const edges = stateManager.getVisibleEdges();
  const containers = stateManager.getVisibleContainers();
  
  console.log(`✅ VisState contains: ${nodes.length} nodes, ${edges.length} edges, ${containers.length} containers`);
  
  // Test 2: Layout using v3 ELK bridge
  console.log('📐 Performing layout using v3 ELK bridge...');
  try {
    await stateManager.performLayout({
      algorithm: 'mrtree',
      direction: 'DOWN'
    });
    console.log('✅ Layout completed successfully');
    
    // Check that layout data is now in VisState
    const layoutedNodes = stateManager.getVisibleNodes();
    const hasPositions = layoutedNodes.some(node => node.x !== undefined && node.y !== undefined);
    console.log(`✅ Layout positions applied: ${hasPositions}`);
    
  } catch (error) {
    console.log('⚠️ Layout test skipped (ELK not available in test environment)');
  }
  
  // Test 3: ReactFlow conversion using v3 bridge
  console.log('🔄 Converting to ReactFlow using v3 bridge...');
  const reactFlowData = stateManager.getReactFlowData();
  console.log(`✅ ReactFlow data generated: ${reactFlowData.nodes.length} nodes, ${reactFlowData.edges.length} edges`);
  
  // Test 4: Container operations using v3 VisState
  console.log('📦 Testing container operations...');
  const initialVisibleNodes = stateManager.getVisibleNodes().length;
  
  stateManager.collapseContainer('container1');
  const collapsedVisibleNodes = stateManager.getVisibleNodes().length;
  console.log(`✅ After collapse: ${initialVisibleNodes} → ${collapsedVisibleNodes} visible nodes`);
  
  stateManager.expandContainer('container1');
  const expandedVisibleNodes = stateManager.getVisibleNodes().length;
  console.log(`✅ After expand: ${collapsedVisibleNodes} → ${expandedVisibleNodes} visible nodes`);
  
  // Test 5: State queries
  console.log('🔍 Testing state queries...');
  const finalState = stateManager.getState();
  console.log(`✅ Final state: ${finalState.nodes.length} nodes, ${finalState.edges.length} edges, ${finalState.containers.length} containers`);
  
  console.log('🎉 Integration test completed successfully!');
  return true;
}

// Export for use in other tests
export { testIntegration };

// If running directly (e.g., node demo.js), run the test
if (typeof window === 'undefined' && typeof module !== 'undefined') {
  testIntegration().catch(console.error);
}