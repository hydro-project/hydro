/**
 * Edge Styling Integration Test
 * 
 * Tests the complete pipeline from JSON with edgeStyleConfig to rendered ReactFlow edges
 */

import {
  parseGraphJSON,
  createRenderConfig,
  FlowGraph,
  VisualizationEngine
} from '../index';
import { convertEdgesToReactFlow } from '../bridges/EdgeBridge';

describe('Edge Styling Integration', () => {
  const testJSON = {
    nodes: [
      { id: 'n1', label: 'Node 1', type: 'standard' },
      { id: 'n2', label: 'Node 2', type: 'standard' }
    ],
    edges: [
      {
        id: 'e1',
        source: 'n1',
        target: 'n2',
        edgeProperties: ['Network', 'Bounded'],
        label: 'test edge'
      }
    ],
    edgeStyleConfig: {
      propertyMappings: {
        'Network': {
          reactFlowType: 'floating',
          style: {
            stroke: '#2563eb',
            strokeWidth: 3
          },
          animated: true,
          label: 'NET'
        },
        'Bounded': {
          reactFlowType: 'floating',
          style: {
            stroke: '#16a34a',
            strokeWidth: 2
          },
          animated: false,
          label: 'B'
        }
      },
      defaultStyle: {
        reactFlowType: 'floating',
        style: {
          stroke: '#999999',
          strokeWidth: 2
        },
        animated: false
      },
      combinationRules: {
        priority: ['Network', 'Bounded'],
        description: 'Network has priority over Bounded'
      }
    }
  };

  test('parseGraphJSON extracts edgeStyleConfig correctly', () => {
    const parseResult = parseGraphJSON(testJSON);
    
    expect(parseResult.metadata.edgeStyleConfig).toBeDefined();
    expect(parseResult.metadata.edgeStyleConfig.propertyMappings).toBeDefined();
    expect(parseResult.metadata.edgeStyleConfig.propertyMappings['Network']).toBeDefined();
    expect(parseResult.metadata.edgeStyleConfig.propertyMappings['Network'].style.stroke).toBe('#2563eb');
  });

  test('createRenderConfig includes edgeStyleConfig', () => {
    const parseResult = parseGraphJSON(testJSON);
    const renderConfig = createRenderConfig(parseResult, { fitView: true });
    
    expect(renderConfig.edgeStyleConfig).toBeDefined();
    expect(renderConfig.edgeStyleConfig).toBe(parseResult.metadata.edgeStyleConfig);
    expect(renderConfig.fitView).toBe(true);
  });

  test('EdgeBridge processes edge properties correctly', () => {
    const parseResult = parseGraphJSON(testJSON);
    const edges = Array.from(parseResult.state.visibleEdges);
    
    const reactFlowEdges = convertEdgesToReactFlow(edges, {
      edgeStyleConfig: parseResult.metadata.edgeStyleConfig,
      showPropertyLabels: true,
      enableAnimations: true
    });
    
    expect(reactFlowEdges).toHaveLength(1);
    
    const edge = reactFlowEdges[0];
    expect(edge.id).toBe('e1');
    expect(edge.type).toBe('floating');
    expect(edge.animated).toBe(true); // Network property should enable animation
    expect(edge.style?.stroke).toBe('#2563eb'); // Network should take priority
    expect(edge.data?.edgeProperties).toContain('Network');
    expect(edge.data?.edgeProperties).toContain('Bounded');
  });

  test('Edge priority system works correctly', () => {
    const parseResult = parseGraphJSON(testJSON);
    const edges = Array.from(parseResult.state.visibleEdges);
    
    // Network should have priority over Bounded according to the combinationRules
    const reactFlowEdges = convertEdgesToReactFlow(edges, {
      edgeStyleConfig: parseResult.metadata.edgeStyleConfig
    });
    
    const edge = reactFlowEdges[0];
    expect(edge.style?.stroke).toBe('#2563eb'); // Network color, not Bounded color
    expect(edge.style?.strokeWidth).toBe(3); // Network width, not Bounded width
    expect(edge.animated).toBe(true); // Network animation
  });

  test('Floating edges do not have handle properties', () => {
    const parseResult = parseGraphJSON(testJSON);
    const edges = Array.from(parseResult.state.visibleEdges);
    
    const reactFlowEdges = convertEdgesToReactFlow(edges, {
      edgeStyleConfig: parseResult.metadata.edgeStyleConfig
    });
    
    const edge = reactFlowEdges[0];
    expect(edge.sourceHandle).toBeUndefined();
    expect(edge.targetHandle).toBeUndefined();
    expect(edge.type).toBe('floating');
  });

  test('Edge labels are created correctly', () => {
    const parseResult = parseGraphJSON(testJSON);
    const edges = Array.from(parseResult.state.visibleEdges);
    
    const reactFlowEdges = convertEdgesToReactFlow(edges, {
      edgeStyleConfig: parseResult.metadata.edgeStyleConfig,
      showPropertyLabels: true
    });
    
    const edge = reactFlowEdges[0];
    // Should combine original label with property abbreviations
    expect(edge.label).toContain('test edge');
    expect(edge.label).toMatch(/\[.*\]/); // Should contain property abbreviations in brackets
  });

  test('ReactFlowBridge integration with EdgeBridge', async () => {
    const parseResult = parseGraphJSON(testJSON);
    const renderConfig = createRenderConfig(parseResult);
    
    // Run layout first
    const engine = new VisualizationEngine(parseResult.state);
    await engine.runLayout();
    
    // Test ReactFlowBridge with edge style config
    const { ReactFlowBridge } = await import('../bridges/ReactFlowBridge');
    const bridge = new ReactFlowBridge();
    bridge.setEdgeStyleConfig(renderConfig.edgeStyleConfig);
    
    const reactFlowData = bridge.visStateToReactFlow(parseResult.state);
    
    expect(reactFlowData.edges).toHaveLength(1);
    const edge = reactFlowData.edges[0];
    expect(edge.type).toBe('floating');
    expect(edge.style?.stroke).toBe('#2563eb');
    expect(edge.animated).toBe(true);
  });
});

// Export for use in other tests if needed
export const EdgeStylingIntegrationTestSuite = 'Edge Styling Integration';
