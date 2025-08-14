import { ReactFlowBridge } from '../bridges/ReactFlowBridge';
import { VisualizationState } from '../core/VisualizationState';

describe('ReactFlowBridge Container Positioning', () => {
  let bridge: ReactFlowBridge;
  let visState: VisualizationState;

  beforeEach(() => {
    bridge = new ReactFlowBridge();
    visState = new VisualizationState();
  });

  test('should convert ELK absolute coordinates to ReactFlow relative coordinates for child containers', () => {
    // Set up a parent container at (100, 100) with size 400x300
    visState.setContainer({
      id: 'parent',
      x: 100,
      y: 100,
      width: 400,
      height: 300,
      collapsed: false,
      children: ['child']  // Establish hierarchy properly
    });

    // Set up a child container at absolute position (150, 150) 
    // Should become relative position (50, 50) within parent
    visState.setContainer({
      id: 'child',
      x: 150,
      y: 150, 
      width: 100,
      height: 100,
      collapsed: false
    });

    // Mock ELK layout data  
    visState.setContainerLayout('parent', {
      position: { x: 100, y: 100 },
      dimensions: { width: 400, height: 300 }
    });

    visState.setContainerLayout('child', {
      position: { x: 150, y: 150 },
      dimensions: { width: 100, height: 100 }
    });

    // Convert to ReactFlow format
    const reactFlowData = bridge.visStateToReactFlow(visState);
    
    // Find containers in the result
    const parentContainer = reactFlowData.nodes.find(n => n.id === 'parent');
    const childContainer = reactFlowData.nodes.find(n => n.id === 'child');

    expect(parentContainer).toBeDefined();
    expect(childContainer).toBeDefined();

    // Parent should keep absolute coordinates
    expect(parentContainer?.position).toEqual({ x: 100, y: 100 });
    expect(parentContainer?.parentId).toBeUndefined();

    // Child should have relative coordinates within parent
    expect(childContainer?.position).toEqual({ x: 50, y: 50 }); // 150-100, 150-100
    expect(childContainer?.parentId).toBe('parent');
    expect(childContainer?.extent).toBe('parent');
  });

  test('should use fallback grid positioning when ELK data is missing', () => {
    // Set up containers without ELK layout data
    visState.setContainer({
      id: 'parent',
      x: 0,
      y: 0,
      width: 400,
      height: 300,
      collapsed: false,
      children: ['child1', 'child2']  // Establish hierarchy properly
    });

    visState.setContainer({
      id: 'child1',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      collapsed: false
    });

    visState.setContainer({
      id: 'child2',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      collapsed: false
    });

    // Convert without ELK data
    const reactFlowData = bridge.visStateToReactFlow(visState);
    
    const child1 = reactFlowData.nodes.find(n => n.id === 'child1');
    const child2 = reactFlowData.nodes.find(n => n.id === 'child2');

    expect(child1).toBeDefined();
    expect(child2).toBeDefined();

    // Should use grid fallback positioning
    expect(child1?.position.x).toBeGreaterThan(0); // Should have padding
    expect(child2?.position.x).toBeGreaterThan(child1?.position.x); // Should be in next grid column

    expect(child1?.parentId).toBe('parent');
    expect(child2?.parentId).toBe('parent');
  });
});
