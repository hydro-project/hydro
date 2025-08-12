/**
 * Unit test for collapsed containers with hyperEdges rendering
 * Tests the specific case where both containers are collapsed and should render with hyperEdges
 */

import { VisualizationState } from '../core/VisState';
import { ReactFlowBridge } from '../bridges/ReactFlowBridge';

describe('Collapsed Containers Rendering', () => {
  test('should render two collapsed containers with bidirectional hyperEdges', () => {
    // Create a VisualizationState that matches the state after both collapses
    const visState = new VisualizationState();
    
    // Add containers (both collapsed, not hidden)
    visState.setContainer('loc_0', {
      name: 'Container A',
      style: 'default',
      collapsed: true,
      hidden: false,
      children: ['0', '1', '7', '8']
    });
    
    visState.setContainer('loc_1', {
      name: 'Container B',
      style: 'default',
      collapsed: true,
      hidden: false,
      children: ['2', '3', '4', '5', '6']
    });
    
    // Add nodes (all hidden since they're in collapsed containers)
    const nodeIds = ['0', '1', '2', '3', '4', '5', '6', '7', '8'];
    nodeIds.forEach(id => {
      visState.setGraphNode(id, {
        label: `Node ${id}`,
        style: 'default',
        hidden: true
      });
    });
    
    // Add hyperEdges (the key elements that should be rendered)
    const hyperEdge1 = {
      id: 'hyper_loc_0_to_loc_1',
      source: 'loc_0',
      target: 'loc_1',
      style: 'default',
      hidden: false
    };
    
    const hyperEdge2 = {
      id: 'hyper_loc_1_to_loc_0',
      source: 'loc_1', 
      target: 'loc_0',
      style: 'default',
      hidden: false
    };
    
    // Add hyperEdges to internal collections
    (visState as any)._collections.hyperEdges.set(hyperEdge1.id, hyperEdge1);
    (visState as any)._collections.hyperEdges.set(hyperEdge2.id, hyperEdge2);
    
    console.log('=== UNIT TEST DEBUG ===');
    console.log('VisibleContainers:', visState.visibleContainers.map(c => ({ id: c.id, collapsed: c.collapsed, hidden: c.hidden })));
    console.log('VisibleNodes:', visState.visibleNodes.map(n => ({ id: n.id, hidden: n.hidden })));
    console.log('VisibleEdges:', visState.visibleEdges.map(e => ({ id: e.id, source: e.source, target: e.target, hidden: e.hidden })));
    
    // Test the ReactFlow conversion
    const bridge = new ReactFlowBridge();
    const reactFlowData = bridge.visStateToReactFlow(visState);
    
    console.log('=== REACTFLOW CONVERSION RESULT ===');
    console.log('Nodes:', reactFlowData.nodes.map(n => ({ id: n.id, type: n.type, position: n.position })));
    console.log('Edges:', reactFlowData.edges.map(e => ({ id: e.id, source: e.source, target: e.target, type: e.type })));
    
    // Assertions
    expect(reactFlowData.nodes).toHaveLength(2); // Two containers
    expect(reactFlowData.edges).toHaveLength(2); // Two hyperEdges
    
    // Check containers are present
    const containerNodes = reactFlowData.nodes.filter(n => n.type === 'container');
    expect(containerNodes).toHaveLength(2);
    expect(containerNodes.map(n => n.id).sort()).toEqual(['loc_0', 'loc_1']);
    
    // Check hyperEdges are present
    const edges = reactFlowData.edges;
    expect(edges.map(e => e.id).sort()).toEqual(['hyper_loc_0_to_loc_1', 'hyper_loc_1_to_loc_0']);
    expect(edges.find(e => e.id === 'hyper_loc_0_to_loc_1')).toMatchObject({
      source: 'loc_0',
      target: 'loc_1'
    });
    expect(edges.find(e => e.id === 'hyper_loc_1_to_loc_0')).toMatchObject({
      source: 'loc_1', 
      target: 'loc_0'
    });
    
    console.log('✅ Test passed: Collapsed containers with hyperEdges render correctly');
  });
});
