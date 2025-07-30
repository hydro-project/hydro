/**
 * Container Collapse Logic
 * Handles collapsing containers, rerouting edges, and layout adjustments
 */

/**
 * Process nodes to handle collapsed containers
 * Replace expanded containers with collapsed nodes and hide their children
 */
export function processCollapsedContainers(nodes, collapsedContainerIds) {
  const collapsedSet = new Set(collapsedContainerIds);
  
  // Helper function to count leaf nodes recursively within a container
  function countLeafNodes(containerId, nodes) {
    let leafCount = 0;
    
    nodes.forEach(node => {
      if (node.parentId === containerId) {
        if (node.type === 'group') {
          // This is a nested container, count its leaf nodes recursively
          leafCount += countLeafNodes(node.id, nodes);
        } else {
          // This is a leaf node
          leafCount += 1;
        }
      }
    });
    
    return leafCount;
  }
  
  // Helper function to check if a node should be hidden due to any ancestor being collapsed
  function isNodeHiddenByAncestor(node, nodes) {
    if (!node.parentId) return false;
    
    // Check if direct parent is collapsed
    if (collapsedSet.has(node.parentId)) {
      return true;
    }
    
    // Recursively check ancestors
    const parent = nodes.find(n => n.id === node.parentId);
    if (parent) {
      return isNodeHiddenByAncestor(parent, nodes);
    }
    
    return false;
  }
  
  return nodes.map(node => {
    // Handle group container nodes that should be collapsed
    if (node.type === 'group' && collapsedSet.has(node.id)) {
      const leafNodeCount = countLeafNodes(node.id, nodes);
      const collapsed = createCollapsedContainer(node, leafNodeCount);
      return collapsed;
    }
    
    // Handle collapsed containers that should be expanded
    if (node.type === 'collapsedContainer' && !collapsedSet.has(node.id)) {
      const expanded = restoreExpandedContainer(node);
      return expanded;
    }
    
    // Handle child nodes - hide them if ANY ancestor is collapsed (recursive)
    if (isNodeHiddenByAncestor(node, nodes)) {
      return {
        ...node,
        hidden: true,
      };
    }
    
    // Show non-collapsed nodes
    const result = {
      ...node,
      hidden: false,
    };
    if (node.type === 'group') {
      // Keep expanded group node without logging
    }
    return result;
  });
}

/**
 * Create a collapsed container node from a group node
 */
function createCollapsedContainer(groupNode, nodeCount = 0) {
  // Make collapsed container significantly smaller than original
  const originalWidth = groupNode.style?.width || 400;
  const originalHeight = groupNode.style?.height || 300;
  
  // Collapsed size should be much smaller - based on label length but capped
  const labelLength = groupNode.data?.label?.length || 10;
  const collapsedWidth = Math.min(180, originalWidth * 0.3);
  const collapsedHeight = Math.min(60, originalHeight * 0.2);

  return {
    ...groupNode,
    type: 'collapsedContainer', // Use a different type to render differently
    // ReactFlow v12 uses width/height directly on the node, not in style
    width: collapsedWidth,
    height: collapsedHeight,
    style: {
      ...groupNode.style,
      // Also keep in style for our component to use
      width: collapsedWidth,
      height: collapsedHeight,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontSize: '12px',
      fontWeight: '600',
      cursor: 'pointer',
    },
    data: {
      ...groupNode.data,
      isCollapsed: true,
      nodeCount: nodeCount, // Pass the count of hidden nodes
      nodeStyle: groupNode.style, // Store original style for CollapsedContainerNode
      originalDimensions: {
        width: originalWidth,
        height: originalHeight,
      },
    },
  };
}

/**
 * Restore a collapsed container back to its expanded group form
 */
function restoreExpandedContainer(collapsedNode) {
  const originalDimensions = collapsedNode.data?.originalDimensions;
  const originalStyle = collapsedNode.data?.nodeStyle;
  
  return {
    ...collapsedNode,
    type: 'group', // Change back to group type
    width: originalDimensions?.width || 400,
    height: originalDimensions?.height || 300,
    style: {
      ...originalStyle,
      width: originalDimensions?.width || 400,
      height: originalDimensions?.height || 300,
    },
    data: {
      ...collapsedNode.data,
      isCollapsed: false,
      // Remove collapsed-specific data
      nodeCount: undefined,
    },
  };
}

/**
 * Reroute edges for collapsed containers
 * Edges that go into/out of nodes inside collapsed containers should be rerouted to the container
 */
export function rerouteEdgesForCollapsedContainers(edges, nodes, childNodesByParent, collapsedContainerIds) {
  const collapsedSet = new Set(collapsedContainerIds);
  
  // Create a set of hidden node IDs for quick lookup
  const hiddenNodeIds = new Set();
  nodes.forEach(node => {
    if (node.hidden) {
      hiddenNodeIds.add(node.id);
    }
  });
  
  
  return edges.map(edge => {
    // If we're expanding (no collapsed containers), restore any hidden edges
    if (collapsedSet.size === 0 && edge.hidden) {
      return {
        ...edge,
        hidden: false,
        // Restore original handles if they exist, otherwise remove them
        sourceHandle: edge.data?.originalSourceHandle !== undefined ? edge.data.originalSourceHandle : undefined,
        targetHandle: edge.data?.originalTargetHandle !== undefined ? edge.data.originalTargetHandle : undefined,
        style: {
          ...edge.style,
          strokeWidth: edge.style?.strokeWidth || 2,
          stroke: edge.style?.stroke || '#999',
        },
      };
    }
    
    // If this edge has been previously rerouted and we're expanding, restore original
    if (collapsedSet.size === 0 && edge.data?.originalSource && edge.data?.originalTarget) {
      return restoreOriginalEdge(edge);
    }
    
    // Hide edges that connect ONLY to hidden nodes (both endpoints hidden = internal edge)
    // Don't remove them - just hide them so they can be restored later
    if (hiddenNodeIds.has(edge.source) && hiddenNodeIds.has(edge.target)) {
      return {
        ...edge,
        hidden: true,
        // Store original handles for restoration
        data: {
          ...edge.data,
          originalSourceHandle: edge.data?.originalSourceHandle !== undefined ? edge.data.originalSourceHandle : edge.sourceHandle,
          originalTargetHandle: edge.data?.originalTargetHandle !== undefined ? edge.data.originalTargetHandle : edge.targetHandle,
        },
        // Clean up handle properties to avoid ReactFlow errors
        sourceHandle: undefined,
        targetHandle: undefined,
      };
    }
    
    // Now process edges for rerouting (these are edges that don't connect to hidden nodes)
    let newSource = edge.source;
    let newTarget = edge.target;
    let shouldHide = false;
    
    // Find which containers the source and target belong to (with recursive lookup)
    const sourceContainer = findNodeContainer(edge.source, childNodesByParent, collapsedContainerIds);
    const targetContainer = findNodeContainer(edge.target, childNodesByParent, collapsedContainerIds);
    
    // Check if source is in a collapsed container
    if (sourceContainer && collapsedSet.has(sourceContainer)) {
      newSource = sourceContainer;
    }
    
    // Check if target is in a collapsed container
    if (targetContainer && collapsedSet.has(targetContainer)) {
      newTarget = targetContainer;
    }
    
    // Hide edges that are internal to collapsed containers
    if (sourceContainer && targetContainer && 
        sourceContainer === targetContainer && 
        collapsedSet.has(sourceContainer)) {
      shouldHide = true;
    }
    
    // Hide self-loops on collapsed containers (but only if both source and target were rerouted)
    if (newSource === newTarget && collapsedSet.has(newSource) && 
        (sourceContainer || targetContainer)) {
      shouldHide = true;
    }
    
    // Ensure we don't create invalid edges
    if (!newSource || !newTarget) {
      console.warn(`Invalid edge created: ${edge.id} with source=${newSource}, target=${newTarget}. Keeping original.`);
      newSource = edge.source;
      newTarget = edge.target;
    }
    
    // If no change needed, return original edge (don't modify handles)
    if (newSource === edge.source && newTarget === edge.target && !shouldHide) {
      return edge;
    }
    
    // Create the rerouted edge - completely omit handle properties to avoid null issues
    const result = {
      ...edge,
      source: newSource,
      target: newTarget,
      hidden: shouldHide,
      // Improve edge styling for collapsed containers
      style: {
        ...edge.style,
        strokeWidth: shouldHide ? 0 : (edge.style?.strokeWidth || 2),
        stroke: shouldHide ? 'transparent' : (edge.style?.stroke || '#999'),
      },
      data: {
        ...edge.data,
        originalSource: edge.data?.originalSource || edge.source,
        originalTarget: edge.data?.originalTarget || edge.target,
        originalSourceHandle: edge.data?.originalSourceHandle || edge.sourceHandle,
        originalTargetHandle: edge.data?.originalTargetHandle || edge.targetHandle,
        isRerouted: newSource !== edge.source || newTarget !== edge.target,
      },
    };
    
    // Completely remove handle properties when rerouting to containers
    // Don't set them to undefined - delete them entirely
    delete result.sourceHandle;
    delete result.targetHandle;
    
    if (result.data.isRerouted || shouldHide) {
    }
    
    return result;
  });
  // REMOVED: Don't filter out edges - keep them all but hidden
}

/**
 * Restore an edge to its original state before rerouting
 */
function restoreOriginalEdge(edge) {
  const originalSource = edge.data?.originalSource || edge.source;
  const originalTarget = edge.data?.originalTarget || edge.target;
  
  const restored = {
    ...edge,
    source: originalSource,
    target: originalTarget,
    hidden: false,
    style: {
      ...edge.style,
      strokeWidth: edge.style?.strokeWidth || 2,
      stroke: edge.style?.stroke || '#999',
    },
    data: {
      ...edge.data,
      // Clean up rerouting data
      originalSource: undefined,
      originalTarget: undefined,
      originalSourceHandle: undefined,
      originalTargetHandle: undefined,
      isRerouted: false,
    },
  };
  
  // Restore original handle properties if they were cached
  if (edge.data?.originalSourceHandle !== undefined) {
    restored.sourceHandle = edge.data.originalSourceHandle;
  } else {
    // If no cached handle, remove the property entirely
    delete restored.sourceHandle;
  }
  
  if (edge.data?.originalTargetHandle !== undefined) {
    restored.targetHandle = edge.data.originalTargetHandle;
  } else {
    // If no cached handle, remove the property entirely
    delete restored.targetHandle;
  }
  
  return restored;
}

/**
 * Find which container a node belongs to
 * For hierarchical containers, returns the outermost collapsed container
 */
function findNodeContainer(nodeId, childNodesByParent, collapsedContainerIds) {
  const collapsedSet = new Set(collapsedContainerIds);
  
  // Recursively find the container hierarchy for this node
  function findContainerHierarchy(nodeId) {
    // Handle both Map and object formats
    if (childNodesByParent instanceof Map) {
      for (const [parentId, childIds] of childNodesByParent) {
        if (childIds.has(nodeId)) {
          return [parentId, ...findContainerHierarchy(parentId)];
        }
      }
    } else {
      // Handle object format
      for (const [parentId, childIds] of Object.entries(childNodesByParent)) {
        if (childIds.has && childIds.has(nodeId)) {
          return [parentId, ...findContainerHierarchy(parentId)];
        }
      }
    }
    return [];
  }
  
  const containerHierarchy = findContainerHierarchy(nodeId);
  
  // Find the outermost collapsed container in the hierarchy
  // Start from the immediate parent and work outward
  for (const containerId of containerHierarchy) {
    if (collapsedSet.has(containerId)) {
      return containerId;
    }
  }
  
  return null;
}

/**
 * Update layout to maintain positions while adjusting for collapsed containers
 * This preserves the layout of non-changed containers while allowing the collapsed
 * container to find a new optimal position
 */
export function adjustLayoutForCollapsedContainers(nodes, collapsedContainerIds, lastChangedContainer) {
  // For now, we'll let the layout algorithm handle repositioning
  // In a future iteration, we could implement more sophisticated position preservation
  return nodes.map(node => {
    // If this is a collapsed container and it wasn't the one that just changed,
    // we could preserve its position here
    if (node.type === 'group' && collapsedContainerIds.has(node.id) && 
        node.id !== lastChangedContainer) {
      // Could preserve position for unchanged containers
      // For now, let ELK handle all positioning
    }
    
    return node;
  });
}
