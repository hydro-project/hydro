/**
 * @fileoverview Simple ReactFlow Integration Example
 * 
 * Basic demonstration without complex state management.
 */

import React from 'react';
import { 
  HydroFlow,
  createVisualizationState,
  createVisualizationStateAdapter,
  NODE_STYLES,
  EDGE_STYLES
} from '../index.js';

// Simple example component
export const SimpleHydroFlowExample: React.FC = () => {
  // Create visualization state
  const coreState = createVisualizationState();
  
  // Add example nodes
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

  // Add example edges
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

  // Create adapter
  const visualizationState = createVisualizationStateAdapter(coreState);

  // Simple event handlers
  const eventHandlers = {
    onNodeClick: (event: React.MouseEvent, node: any) => {
      console.log('Node clicked:', node.id);
    },
    onEdgeClick: (event: React.MouseEvent, edge: any) => {
      console.log('Edge clicked:', edge.id);
    }
  };

  return (
    <div style={{ width: '100%', height: '600px', border: '1px solid #ddd' }}>
      <div style={{ 
        padding: '10px', 
        background: '#f8f9fa', 
        borderBottom: '1px solid #ddd',
        fontSize: '14px'
      }}>
        <strong>Simple Hydro ReactFlow Example</strong>
      </div>
      
      <div style={{ height: 'calc(100% - 50px)' }}>
        <HydroFlow
          visualizationState={visualizationState}
          eventHandlers={eventHandlers}
          onLayoutComplete={() => console.log('Layout completed!')}
          onError={(error) => console.error('Visualization error:', error)}
        />
      </div>
    </div>
  );
};

export default SimpleHydroFlowExample;
