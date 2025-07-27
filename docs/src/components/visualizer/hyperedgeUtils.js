/**
 * Hyperedge Utilities
 * 
 * Functions for generating and managing hyperedges between containers
 * when they are collapsed. This enables visual connections between
 * collapsed containers that represent aggregated data flow.
 */

/**
 * Generates hyperedges between containers based on cross-container edges
 * @param {Array} nodes - All nodes in the graph
 * @param {Array} edges - All edges in the graph
 * @returns {Array} Array of hyperedges for ELK layout
 */
export const generateHyperedges = (nodes, edges) => {
  const hyperedges = [];
  const containerPairs = new Set();
  
  // Find all edges that cross container boundaries
  edges.forEach(edge => {
    const sourceNode = nodes.find(n => n.id === edge.source);
    const targetNode = nodes.find(n => n.id === edge.target);
    
    if (sourceNode && targetNode) {
      const sourceLocationId = sourceNode.data?.locationId;
      const targetLocationId = targetNode.data?.locationId;
      
      // Only create hyperedges between different locations (containers)
      if (sourceLocationId !== undefined && targetLocationId !== undefined && 
          sourceLocationId !== targetLocationId) {
        
        const sourceContainerId = `container_${sourceLocationId}`;
        const targetContainerId = `container_${targetLocationId}`;
        const pairKey = `${sourceContainerId}->${targetContainerId}`;
        
        // Avoid duplicate hyperedges between the same container pair
        if (!containerPairs.has(pairKey)) {
          containerPairs.add(pairKey);
          hyperedges.push({
            id: `hyperedge_${sourceLocationId}_to_${targetLocationId}`,
            sources: [sourceContainerId],
            targets: [targetContainerId],
          });
        }
      }
    }
  });
  
  return hyperedges;
};

/**
 * Routes edges appropriately when containers are collapsed
 * @param {Array} edges - Current edges
 * @param {Object} collapsedLocations - Map of locationId -> boolean (collapsed state)
 * @param {Object} childNodeIdsByParent - Map of parentId -> Set of child node IDs
 * @param {Array} hyperedges - Precomputed hyperedges to include when containers are collapsed
 * @returns {Array} Updated edges with proper routing
 */
export const routeEdgesForCollapsedContainers = (edges, collapsedLocations, childNodeIdsByParent, collapsedContainerIds = null) => {
  const routedEdges = edges.map(edge => {
    // Skip hyperedges - they're handled separately
    if (edge.data?.isHyperedge) {
      return edge;
    }
    
    let newEdge = { ...edge };
    
    // Reset any previous modifications
    if (newEdge.data?.originalSource) {
      newEdge.source = newEdge.data.originalSource;
      newEdge.data = { ...newEdge.data };
      delete newEdge.data.originalSource;
    }
    if (newEdge.data?.originalTarget) {
      newEdge.target = newEdge.data.originalTarget;
      newEdge.data = { ...newEdge.data };
      delete newEdge.data.originalTarget;
    }
    newEdge.hidden = false;

    // Find collapsed containers containing source/target
    let sourceInCollapsedContainer = null;
    let targetInCollapsedContainer = null;
    
    // If we have specific collapsed container IDs, use them; otherwise fall back to location-based logic
    if (collapsedContainerIds && collapsedContainerIds.size > 0) {
      // Simple approach: if ANY container is collapsed, check ALL containers with children
      for (const containerId in childNodeIdsByParent) {
        const childIds = childNodeIdsByParent[containerId] || new Set();
        
        // If this container has children and ANY container is collapsed, treat this as potentially collapsed
        if (childIds.size > 0 && collapsedContainerIds.size > 0) {
          if (childIds.has(newEdge.source)) {
            sourceInCollapsedContainer = containerId;
          }
          if (childIds.has(newEdge.target)) {
            targetInCollapsedContainer = containerId;
          }
        }
      }
    } else {
      // Fallback to original logic
      for (const containerId in childNodeIdsByParent) {
        const childIds = childNodeIdsByParent[containerId] || new Set();
        
        // Extract locationId from containerId (e.g., "container_1" -> "1")
        const locationId = containerId.replace('container_', '');
        const isContainerCollapsed = collapsedLocations[locationId];
        
        if (isContainerCollapsed && childIds.size > 0) {
          if (childIds.has(newEdge.source)) {
            sourceInCollapsedContainer = containerId;
          }
          if (childIds.has(newEdge.target)) {
            targetInCollapsedContainer = containerId;
          }
        }
      }
    }
    
    // Apply routing based on container states
    if (sourceInCollapsedContainer && targetInCollapsedContainer) {
      if (sourceInCollapsedContainer === targetInCollapsedContainer) {
        newEdge.hidden = true; // Hide internal edges
      } else {
        // Route container to container (this creates the hyperedge visual)
        newEdge.data = { ...newEdge.data, originalSource: newEdge.source, originalTarget: newEdge.target };
        newEdge.source = sourceInCollapsedContainer;
        newEdge.target = targetInCollapsedContainer;
        // Style it as a hyperedge
        newEdge.style = {
          ...newEdge.style,
          strokeWidth: 3,
          stroke: '#880088',
          strokeDasharray: '8,4'
        };
        newEdge.animated = true;
        newEdge.markerEnd = { ...newEdge.markerEnd, color: '#880088' };
      }
    } else if (sourceInCollapsedContainer) {
      newEdge.data = { ...newEdge.data, originalSource: newEdge.source };
      newEdge.source = sourceInCollapsedContainer;
    } else if (targetInCollapsedContainer) {
      newEdge.data = { ...newEdge.data, originalTarget: newEdge.target };
      newEdge.target = targetInCollapsedContainer;
    }
    
    return newEdge;
  });
  
  return routedEdges;
};

/**
 * Creates a mapping of child node IDs by their parent container
 * @param {Array} nodes - All nodes in the graph
 * @returns {Object} Map of parentId -> Set of child node IDs
 */
export const createChildNodeMapping = (nodes) => {
  const childNodeIdsByParent = {};
  
  nodes.forEach(node => {
    if (node.parentNode) {
      if (!childNodeIdsByParent[node.parentNode]) {
        childNodeIdsByParent[node.parentNode] = new Set();
      }
      childNodeIdsByParent[node.parentNode].add(node.id);
    }
  });
  
  return childNodeIdsByParent;
};
