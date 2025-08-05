/**
 * Integration adapter for v2 frontend components with v3 core/bridges
 * 
 * This adapter provides a compatibility layer that allows v2 frontend components
 * to work with the v3 VisState architecture while maintaining clean separation.
 */

import { createVisualizationState } from '../index.js';
import { ELKBridge } from '../bridges/ELKBridge.js';
import { ReactFlowBridge } from '../bridges/ReactFlowBridge.js';

/**
 * Create an integrated state manager that bridges v2 and v3 architectures
 */
export function createIntegratedStateManager() {
  // Create the v3 VisState as single source of truth
  const visState = createVisualizationState();
  
  // Create stateless bridges
  const elkBridge = new ELKBridge();
  const reactFlowBridge = new ReactFlowBridge();
  
  return {
    // V3 core state (single source of truth)
    visState,
    
    // V3 stateless bridges
    elkBridge,
    reactFlowBridge,
    
    // Compatibility methods for v2 frontend
    
    /**
     * Add graph data in v2 format but store in v3 VisState
     */
    setGraphData(graphData) {
      if (!graphData || !graphData.nodes) return;
      
      // Clear existing data
      visState.clear();
      
      // Add nodes to VisState
      graphData.nodes.forEach(node => {
        visState.setGraphNode(node.id, {
          label: node.label || node.id,
          style: node.style || 'default',
          hidden: false,
          ...node
        });
      });
      
      // Add edges to VisState  
      if (graphData.edges) {
        graphData.edges.forEach(edge => {
          visState.setGraphEdge(edge.id, {
            source: edge.source,
            target: edge.target,
            style: edge.style || 'default',
            hidden: false,
            ...edge
          });
        });
      }
      
      // Add containers to VisState
      if (graphData.containers) {
        graphData.containers.forEach(container => {
          visState.setContainer(container.id, {
            children: container.children || [],
            expandedDimensions: container.expandedDimensions || { width: 200, height: 100 },
            collapsed: container.collapsed || false,
            hidden: false,
            label: container.label,
            ...container
          });
        });
      }
    },
    
    /**
     * Get layout data using v3 bridges  
     */
    async performLayout(layoutConfig = {}) {
      // Use ELK bridge for layout
      const elkResult = await elkBridge.applyLayout(visState, {
        algorithm: layoutConfig.algorithm || 'mrtree',
        direction: layoutConfig.direction || 'DOWN',
        ...layoutConfig
      });
      
      // Layout is now stored in VisState via bridge
      return elkResult;
    },
    
    /**
     * Get ReactFlow data using v3 bridge
     */
    getReactFlowData() {
      return reactFlowBridge.visStateToReactFlow(visState);
    },
    
    /**
     * Get current state for v2 frontend compatibility
     */
    getState() {
      return {
        nodes: visState.visibleNodes,
        edges: visState.visibleEdges,
        containers: visState.visibleContainers
      };
    },
    
    /**
     * Container operations using v3 VisState
     */
    collapseContainer(containerId) {
      visState.collapseContainer(containerId);
    },
    
    expandContainer(containerId) {
      visState.expandContainer(containerId);
    },
    
    /**
     * State queries using v3 VisState
     */
    getVisibleNodes() {
      return visState.visibleNodes;
    },
    
    getVisibleEdges() {
      return visState.visibleEdges;
    },
    
    getVisibleContainers() {
      return visState.visibleContainers;
    }
  };
}

/**
 * Hook for React components to use the integrated state manager
 */
export function useIntegratedStateManager() {
  // In a real implementation, this would use React state/context
  // For now, we'll create a new instance each time
  return createIntegratedStateManager();
}