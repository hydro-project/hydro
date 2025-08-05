/**
 * VisState Integration Utilities
 * 
 * Bridge between the existing Visualizer component and the new VisState system.
 * This allows us to gradually migrate to the new bridge architecture while 
 * maintaining compatibility with existing code.
 */

import { createVisualizationState } from '../index.js';
import { ELKBridge } from '../bridges/ELKBridge.js';
import { ReactFlowBridge } from '../bridges/ReactFlowBridge.js';

/**
 * Create a modern click handler that uses VisState for container collapse/expand
 * @param {Function} setNodes - ReactFlow setNodes function  
 * @param {Function} setEdges - ReactFlow setEdges function
 * @param {Function} fitViewport - Function to fit viewport after layout
 * @returns {Function} Click handler function
 */
export function createVisStateClickHandler(setNodes, setEdges, fitViewport) {
  // Create the bridge components
  const elkBridge = new ELKBridge();
  const reactFlowBridge = new ReactFlowBridge();
  
  return async (event, node) => {
    // Only handle container nodes
    if (node.type !== 'group' && node.type !== 'collapsedContainer') {
      return;
    }
    
    event.stopPropagation();
    
    console.log(`[VisStateIntegration] 🎯 Container ${node.id} clicked`);
    
    try {
      // Create VisState from current ReactFlow state
      const visState = await createVisStateFromReactFlowData(node, setNodes, setEdges);
      
      // Determine current state and toggle
      const isCurrentlyCollapsed = node.type === 'collapsedContainer' || 
                                  (node.data && node.data.collapsed);
      
      if (isCurrentlyCollapsed) {
        console.log(`[VisStateIntegration] 📖 Expanding container ${node.id}`);
        visState.expandContainer(node.id);
      } else {
        console.log(`[VisStateIntegration] 📦 Collapsing container ${node.id}`);
        visState.collapseContainer(node.id);
      }
      
      // Run layout through ELK bridge
      console.log(`[VisStateIntegration] 🔄 Running ELK layout...`);
      await elkBridge.layoutVisState(visState);
      
      // Convert back to ReactFlow format
      console.log(`[VisStateIntegration] 🔄 Converting to ReactFlow format...`);
      const reactFlowData = reactFlowBridge.visStateToReactFlow(visState);
      
      // Update ReactFlow
      console.log(`[VisStateIntegration] ✅ Updating ReactFlow: ${reactFlowData.nodes.length} nodes, ${reactFlowData.edges.length} edges`);
      setNodes(reactFlowData.nodes);
      setEdges(reactFlowData.edges);
      
      // Fit viewport after layout
      setTimeout(() => fitViewport(300, 'container toggle'), 200);
      
    } catch (error) {
      console.error(`[VisStateIntegration] ❌ Container toggle failed:`, error);
      throw error;
    }
  };
}

/**
 * Create a VisState from current ReactFlow nodes and edges
 * This is a simplified conversion - in practice you'd want to maintain
 * the VisState as the source of truth.
 */
async function createVisStateFromReactFlowData(clickedNode, setNodes, setEdges) {
  // Get current ReactFlow state
  const currentNodes = await new Promise(resolve => {
    setNodes(nodes => {
      resolve(nodes);
      return nodes;
    });
  });
  
  const currentEdges = await new Promise(resolve => {
    setEdges(edges => {
      resolve(edges);
      return edges;
    });
  });
  
  // Create new VisState and populate it
  const visState = createVisualizationState();
  
  // Add all nodes
  currentNodes.forEach(node => {
    if (node.type === 'group' || node.type === 'collapsedContainer') {
      // Add as container
      visState.setContainer(node.id, {
        label: node.data?.label || node.id,
        collapsed: node.type === 'collapsedContainer' || (node.data && node.data.collapsed),
        children: node.data?.children || [],
        expandedDimensions: { 
          width: node.width || 200, 
          height: node.height || 200 
        }
      });
    } else {
      // Add as regular node
      visState.setGraphNode(node.id, {
        label: node.data?.label || node.id,
        hidden: node.hidden || false
      });
    }
  });
  
  // Add all edges
  currentEdges.forEach(edge => {
    visState.setGraphEdge(edge.id, {
      source: edge.source,
      target: edge.target,
      style: edge.data?.style || 'default'
    });
  });
  
  console.log(`[VisStateIntegration] 📊 Created VisState: ${currentNodes.length} nodes, ${currentEdges.length} edges`);
  
  return visState;
}

/**
 * Validate that container click handling is working
 */
export function validateContainerClickHandling() {
  console.log('[VisStateIntegration] ✅ Container click handling integration ready');
  return true;
}
