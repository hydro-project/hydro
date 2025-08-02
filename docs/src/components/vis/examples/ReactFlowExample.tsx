/**
 * @fileoverview Example ReactFlow Integration
 * 
 * Demonstrates how to use the HydroFlow component with the VisualizationState.
 */

import React, { useState, useCallback } from 'react';
import { 
  HydroFlow,
  createVisualizationState,
  createVisualizationStateAdapter,
  NODE_STYLES,
  EDGE_STYLES,
  DEFAULT_LAYOUT_CONFIG,
  DEFAULT_RENDER_CONFIG
} from '../index.js';
import type { 
  VisualizationState as IVisualizationState,
  HydroFlowEventHandlers,
  LayoutConfig,
  RenderConfig
} from '../index.js';

// Example component showing ReactFlow integration
export const HydroFlowExample: React.FC = () => {
  const [visualizationState] = useState<IVisualizationState>(() => {
    const coreState = createVisualizationState();
    
    // Create example nodes
    coreState.setGraphNode('node1', { 
      label: 'Input Node', 
      style: NODE_STYLES.DEFAULT 
    });
    coreState.setGraphNode('node2', { 
      label: 'Processing Node', 
      style: NODE_STYLES.HIGHLIGHTED 
    });
    coreState.setGraphNode('node3', { 
      label: 'Output Node', 
      style: NODE_STYLES.DEFAULT 
    });
    coreState.setGraphNode('node4', { 
      label: 'Error Node', 
      style: NODE_STYLES.ERROR 
    });

    // Create example edges
    coreState.setGraphEdge('edge1', { 
      source: 'node1', 
      target: 'node2', 
      style: EDGE_STYLES.DEFAULT 
    });
    coreState.setGraphEdge('edge2', { 
      source: 'node2', 
      target: 'node3', 
      style: EDGE_STYLES.THICK 
    });
    coreState.setGraphEdge('edge3', { 
      source: 'node2', 
      target: 'node4', 
      style: EDGE_STYLES.DASHED 
    });

    // Create example container
    coreState.setContainer('container1', { 
      expandedDimensions: { width: 300, height: 200 },
      children: ['node2', 'node4']
    });

    return createVisualizationStateAdapter(coreState);
  });

  const [selectedElements, setSelectedElements] = useState<{
    nodes: string[];
    edges: string[];
  }>({ nodes: [], edges: [] });

  // Event handlers
  const eventHandlers: Partial<HydroFlowEventHandlers> = {
    onNodeClick: useCallback((event, node) => {
      console.log('Node clicked:', node.id);
      setSelectedElements(prev => ({
        ...prev,
        nodes: [node.id]
      }));
    }, []),

    onNodeDoubleClick: useCallback((event, node) => {
      console.log('Node double-clicked:', node.id);
      // Toggle container collapse if it's a container
      if (node.type === 'hydro-container') {
        const container = visualizationState.getContainer(node.id);
        if (container) {
          visualizationState.updateContainer(node.id, { collapsed: !container.collapsed });
        }
      }
    }, [visualizationState]),

    onEdgeClick: useCallback((event, edge) => {
      console.log('Edge clicked:', edge.id);
      setSelectedElements(prev => ({
        ...prev,
        edges: [edge.id]
      }));
    }, []),

    onSelectionChange: useCallback((selection) => {
      setSelectedElements({
        nodes: selection.nodes.map(n => n.id),
        edges: selection.edges.map(e => e.id)
      });
    }, []),

    onPaneClick: useCallback(() => {
      setSelectedElements({ nodes: [], edges: [] });
    }, [])
  };

  // Custom layout configuration
  const layoutConfig: Partial<LayoutConfig> = {
    ...DEFAULT_LAYOUT_CONFIG,
    algorithm: 'layered',
    direction: 'DOWN',
    spacing: {
      nodeNode: 80,
      edgeEdge: 10,
      edgeNode: 20,
      componentComponent: 100
    }
  };

  // Custom render configuration
  const renderConfig: Partial<RenderConfig> = {
    ...DEFAULT_RENDER_CONFIG,
    enableMiniMap: true,
    enableControls: true,
    showBackground: true,
    backgroundVariant: 'dots',
    fitView: true
  };

  return (
    <div style={{ width: '100%', height: '600px', border: '1px solid #ddd' }}>
      <div style={{ 
        padding: '10px', 
        background: '#f8f9fa', 
        borderBottom: '1px solid #ddd',
        fontSize: '14px'
      }}>
        <strong>Hydro ReactFlow Example</strong>
        {selectedElements.nodes.length > 0 && (
          <span style={{ marginLeft: '20px', color: '#007bff' }}>
            Selected nodes: {selectedElements.nodes.join(', ')}
          </span>
        )}
        {selectedElements.edges.length > 0 && (
          <span style={{ marginLeft: '20px', color: '#28a745' }}>
            Selected edges: {selectedElements.edges.join(', ')}
          </span>
        )}
      </div>
      
      <div style={{ height: 'calc(100% - 50px)' }}>
        <HydroFlow
          visualizationState={visualizationState}
          layoutConfig={layoutConfig}
          renderConfig={renderConfig}
          eventHandlers={eventHandlers}
          onLayoutComplete={() => console.log('Layout completed!')}
          onError={(error) => console.error('Visualization error:', error)}
        />
      </div>
    </div>
  );
};

export default HydroFlowExample;
