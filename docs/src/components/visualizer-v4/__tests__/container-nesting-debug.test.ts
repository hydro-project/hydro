import { ReactFlowBridge } from '../bridges/ReactFlowBridge';
import { VisualizationState } from '../core/VisualizationState';

describe('Container Nesting Debug', () => {
  let bridge: ReactFlowBridge;
  let visState: VisualizationState;

  beforeEach(() => {
    bridge = new ReactFlowBridge();
    visState = new VisualizationState();
  });

  test('should correctly convert hierarchical containers with ELK coordinates', () => {
    // Set up containers matching the chat.json structure
    // Root container bt_5 at absolute position (100, 100)
    visState.addContainer('bt_5', {
      id: 'bt_5',
      x: 100,
      y: 100,
      width: 400,
      height: 300,
      collapsed: false,
      children: ['bt_3']
    });

    // Child container bt_3 at absolute position (150, 150) 
    // Should become relative (50, 50) within bt_5
    visState.addContainer('bt_3', {
      id: 'bt_3',
      x: 150,
      y: 150,
      width: 200,
      height: 150,
      collapsed: false,
      children: ['bt_4']
    });

    // Grandchild container bt_4 at absolute position (170, 170)
    // Should become relative (20, 20) within bt_3
    visState.addContainer('bt_4', {
      id: 'bt_4',
      x: 170,
      y: 170,
      width: 100,
      height: 80,
      collapsed: false
    });

    // Root container bt_6 at absolute position (600, 100)
    visState.addContainer('bt_6', {
      id: 'bt_6',
      x: 600,
      y: 100,
      width: 400,
      height: 300,
      collapsed: false,
      children: ['bt_1']
    });

    // Child container bt_1 at absolute position (650, 150)
    // Should become relative (50, 50) within bt_6
    visState.addContainer('bt_1', {
      id: 'bt_1',
      x: 650,
      y: 150,
      width: 200,
      height: 150,
      collapsed: false,
      children: ['bt_2']
    });

    // Grandchild container bt_2 at absolute position (670, 170)
    // Should become relative (20, 20) within bt_1
    visState.addContainer('bt_2', {
      id: 'bt_2',
      x: 670,
      y: 170,
      width: 100,
      height: 80,
      collapsed: false
    });

    // Mock ELK layout data with absolute coordinates
    visState.setContainerLayout('bt_5', {
      position: { x: 100, y: 100 },
      dimensions: { width: 400, height: 300 }
    });

    visState.setContainerLayout('bt_3', {
      position: { x: 150, y: 150 },
      dimensions: { width: 200, height: 150 }
    });

    visState.setContainerLayout('bt_4', {
      position: { x: 170, y: 170 },
      dimensions: { width: 100, height: 80 }
    });

    visState.setContainerLayout('bt_6', {
      position: { x: 600, y: 100 },
      dimensions: { width: 400, height: 300 }
    });

    visState.setContainerLayout('bt_1', {
      position: { x: 650, y: 150 },
      dimensions: { width: 200, height: 150 }
    });

    visState.setContainerLayout('bt_2', {
      position: { x: 670, y: 170 },
      dimensions: { width: 100, height: 80 }
    });

    // Convert to ReactFlow format
    const reactFlowData = bridge.visStateToReactFlow(visState);
    
    // Find containers in the result
    const containers = reactFlowData.nodes.filter(n => n.type === 'container');
    const bt5 = containers.find(n => n.id === 'bt_5');
    const bt3 = containers.find(n => n.id === 'bt_3');
    const bt4 = containers.find(n => n.id === 'bt_4');
    const bt6 = containers.find(n => n.id === 'bt_6');
    const bt1 = containers.find(n => n.id === 'bt_1');
    const bt2 = containers.find(n => n.id === 'bt_2');

    // Debug: Log all container data
    console.log('=== CONTAINER NESTING DEBUG ===');
    containers.forEach(container => {
      console.log(`Container ${container.id}:`, {
        position: container.position,
        parentId: container.parentId,
        extent: container.extent,
        style: container.style
      });
    });

    // Test root containers (no parent)
    expect(bt5).toBeDefined();
    expect(bt5?.parentId).toBeUndefined();
    expect(bt5?.position).toEqual({ x: 100, y: 100 }); // Keep absolute position

    expect(bt6).toBeDefined();
    expect(bt6?.parentId).toBeUndefined();
    expect(bt6?.position).toEqual({ x: 600, y: 100 }); // Keep absolute position

    // Test first level children
    expect(bt3).toBeDefined();
    expect(bt3?.parentId).toBe('bt_5');
    expect(bt3?.extent).toBe('parent');
    expect(bt3?.position).toEqual({ x: 50, y: 50 }); // 150-100, 150-100

    expect(bt1).toBeDefined();
    expect(bt1?.parentId).toBe('bt_6');
    expect(bt1?.extent).toBe('parent');
    expect(bt1?.position).toEqual({ x: 50, y: 50 }); // 650-600, 150-100

    // Test second level children (grandchildren)
    expect(bt4).toBeDefined();
    expect(bt4?.parentId).toBe('bt_3');
    expect(bt4?.extent).toBe('parent');
    expect(bt4?.position).toEqual({ x: 20, y: 20 }); // 170-150, 170-150

    expect(bt2).toBeDefined();
    expect(bt2?.parentId).toBe('bt_1');
    expect(bt2?.extent).toBe('parent');
    expect(bt2?.position).toEqual({ x: 20, y: 20 }); // 670-650, 170-150

    // Verify container ordering (parents should come before children)
    const containerOrder = containers.map(c => c.id);
    const bt5Index = containerOrder.indexOf('bt_5');
    const bt3Index = containerOrder.indexOf('bt_3');
    const bt4Index = containerOrder.indexOf('bt_4');
    const bt6Index = containerOrder.indexOf('bt_6');
    const bt1Index = containerOrder.indexOf('bt_1');
    const bt2Index = containerOrder.indexOf('bt_2');

    expect(bt5Index).toBeLessThan(bt3Index); // bt_5 before bt_3
    expect(bt3Index).toBeLessThan(bt4Index); // bt_3 before bt_4
    expect(bt6Index).toBeLessThan(bt1Index); // bt_6 before bt_1
    expect(bt1Index).toBeLessThan(bt2Index); // bt_1 before bt_2

    // Verify dimensions are preserved
    expect(bt5?.style?.width).toBe(400);
    expect(bt5?.style?.height).toBe(300);
    expect(bt3?.style?.width).toBe(200);
    expect(bt3?.style?.height).toBe(150);
  });

  test('should handle missing ELK data with fallback grid positioning', () => {
    // Set up containers without ELK layout data
    visState.addContainer('parent', {
      id: 'parent',
      x: 0,
      y: 0,
      width: 400,
      height: 300,
      collapsed: false,
      children: ['child1', 'child2']
    });

    visState.addContainer('child1', {
      id: 'child1',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      collapsed: false
    });

    visState.addContainer('child2', {
      id: 'child2',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      collapsed: false
    });

    // Convert without ELK data (should use fallback positioning)
    const reactFlowData = bridge.visStateToReactFlow(visState);
    
    const containers = reactFlowData.nodes.filter(n => n.type === 'container');
    const parent = containers.find(n => n.id === 'parent');
    const child1 = containers.find(n => n.id === 'child1');
    const child2 = containers.find(n => n.id === 'child2');

    // Debug: Log fallback positioning
    console.log('=== FALLBACK POSITIONING DEBUG ===');
    containers.forEach(container => {
      console.log(`Container ${container.id}:`, {
        position: container.position,
        parentId: container.parentId,
        extent: container.extent
      });
    });

    expect(parent?.parentId).toBeUndefined();
    
    expect(child1?.parentId).toBe('parent');
    expect(child1?.extent).toBe('parent');
    expect(child1?.position.x).toBeGreaterThan(0); // Should have grid padding
    
    expect(child2?.parentId).toBe('parent');
    expect(child2?.extent).toBe('parent');
    expect(child2?.position.x).toBeGreaterThan(child1?.position.x); // Should be in next grid position
  });
});
