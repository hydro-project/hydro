/**
 * Container Collapse Logic
 * Handles collapsing containers, rerouting edges, and layout adjustments
 */

import { REQUIRED_HANDLE_IDS } from '../utils/handleValidation.js';
import { COMPONENT_COLORS } from '../utils/constants.js';

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
 * Find the lowest visible ancestor of a node
 * Walks up the parent chain until it finds a visible node
 */
function findLowestVisibleAncestor(nodeId, nodes, visibleNodes) {
  // Find the node itself
  const node = nodes.find(n => n.id === nodeId);
  if (!node || !node.parentId) {
    return null; // No parent, can't find ancestor
  }
  
  // Check if the direct parent is visible
  if (visibleNodes.has(node.parentId)) {
    return node.parentId;
  }
  
  // Recursively check parent's ancestors
  return findLowestVisibleAncestor(node.parentId, nodes, visibleNodes);
}

/**
 * Get all descendant nodes of a container (recursively)
 */
function getAllDescendants(containerId, nodes) {
  const descendants = new Set();
  
  function addDescendants(parentId) {
    nodes.forEach(node => {
      if (node.parentId === parentId) {
        descendants.add(node.id);
        // Recursively add descendants of this node if it's also a container
        addDescendants(node.id);
      }
    });
  }
  
  addDescendants(containerId);
  return descendants;
}

/**
 * Reroute edges for collapsed containers
 * When a container is collapsed, edges that connect to its descendants should be rerouted to the container
 */
export function rerouteEdgesForCollapsedContainers(edges, nodes, childNodesByParent, collapsedContainerIds) {
  const collapsedSet = new Set(collapsedContainerIds);

  // If no containers are collapsed, restore any previously rerouted edges and ensure handles are clean
  if (collapsedSet.size === 0) {
    return edges.map(edge => {
      // CRITICAL: Restore any edge that was modified during collapse (rerouted, hidden, or marked as internal)
      if (edge.data?.isRerouted || edge.data?.isInternalEdge || edge.hidden) {
        // Use stored original values if available, otherwise current values
        const restoredSource = edge.data?.originalSource || edge.source;
        const restoredTarget = edge.data?.originalTarget || edge.target;
        const restoredSourceHandle = edge.data?.originalSourceHandle || edge.sourceHandle || REQUIRED_HANDLE_IDS.source;
        const restoredTargetHandle = edge.data?.originalTargetHandle || edge.targetHandle || REQUIRED_HANDLE_IDS.target;
        
        return {
          ...edge,
          source: restoredSource,
          target: restoredTarget,
          sourceHandle: restoredSourceHandle,
          targetHandle: restoredTargetHandle,
          hidden: false,
          data: {
            ...edge.data,
            // Clean up ALL modification markers
            originalSource: undefined,
            originalTarget: undefined,
            originalSourceHandle: undefined,
            originalTargetHandle: undefined,
            isRerouted: undefined,
            isInternalEdge: undefined,
          },
        };
      }
      
      // For non-modified edges, just ensure handle IDs are set to valid defaults
      return {
        ...edge,
        sourceHandle: edge.sourceHandle || REQUIRED_HANDLE_IDS.source,
        targetHandle: edge.targetHandle || REQUIRED_HANDLE_IDS.target,
      };
    });
  }

  // Step 1: Figure out which nodes are visible and which are hidden
  const visibleNodes = new Set();
  const hiddenNodes = new Set();
  
  nodes.forEach(node => {
    if (node.hidden) {
      hiddenNodes.add(node.id);
    } else {
      visibleNodes.add(node.id);
    }
  });

  // Step 2: For each hidden node, find its lowest visible ancestor
  const hiddenNodeToVisibleAncestor = new Map();
  
  nodes.forEach(node => {
    if (hiddenNodes.has(node.id)) {
      // Find the lowest visible ancestor for this hidden node
      let ancestor = findLowestVisibleAncestor(node.id, nodes, visibleNodes);
      if (ancestor) {
        hiddenNodeToVisibleAncestor.set(node.id, ancestor);
      }
    }
  });

  const resultEdges = edges.filter(edge => {
    // Filter out edges with null/invalid handles that can't be fixed
    if (edge.sourceHandle === "null" || edge.targetHandle === "null" || 
        edge.sourceHandle === null || edge.targetHandle === null ||
        edge.sourceHandle === undefined || edge.targetHandle === undefined) {
      return false; // Remove these problematic edges
    }
    
    return true; // Keep all other edges for processing
  }).map(edge => {
    let newSource = edge.source;
    let newTarget = edge.target;
    let needsRerouting = false;

    // Check if source node is hidden - if so, reroute to its visible ancestor
    if (hiddenNodes.has(edge.source)) {
      const visibleAncestor = hiddenNodeToVisibleAncestor.get(edge.source);
      if (visibleAncestor) {
        newSource = visibleAncestor;
        needsRerouting = true;
      }
    }

    // Check if target node is hidden - if so, reroute to its visible ancestor  
    if (hiddenNodes.has(edge.target)) {
      const visibleAncestor = hiddenNodeToVisibleAncestor.get(edge.target);
      if (visibleAncestor) {
        newTarget = visibleAncestor;
        needsRerouting = true;
      }
    }

    // Additional safety check: make sure newSource and newTarget actually exist in visible nodes
    if (!visibleNodes.has(newSource) || !visibleNodes.has(newTarget)) {
      // CRITICAL: Don't remove internal edges entirely - preserve them as hidden for restoration
      // If both source and target are hidden (internal edge), mark as hidden but preserve
      if (hiddenNodes.has(edge.source) && hiddenNodes.has(edge.target)) {
        return {
          ...edge,
          hidden: true,
          sourceHandle: edge.sourceHandle || REQUIRED_HANDLE_IDS.source,
          targetHandle: edge.targetHandle || REQUIRED_HANDLE_IDS.target,
          data: {
            ...edge.data,
            // Mark as internal edge for restoration
            isInternalEdge: true,
            originalSource: edge.data?.originalSource || edge.source,
            originalTarget: edge.data?.originalTarget || edge.target,
            originalSourceHandle: edge.data?.originalSourceHandle || edge.sourceHandle,
            originalTargetHandle: edge.data?.originalTargetHandle || edge.targetHandle,
          },
        };
      }
      
      // For other cases where edges would connect to non-existent nodes, log and skip
      console.warn(`Edge ${edge.id} would connect to non-existent nodes:`, {
        edge,
        newSource,
        newTarget,
        sourceExists: visibleNodes.has(newSource),
        targetExists: visibleNodes.has(newTarget),
        visibleNodes: Array.from(visibleNodes)
      });
      return null;
    }

    // If no rerouting needed, return edge with clean handles
    // CRITICAL: Always ensure sourceHandle and targetHandle are set to valid values
    // that exist on both GroupNode and CollapsedContainerNode components
    if (!needsRerouting) {
      return {
        ...edge,
        sourceHandle: edge.sourceHandle || REQUIRED_HANDLE_IDS.source, // Must match Handle IDs in GroupNode.js and CollapsedContainerNode.js
        targetHandle: edge.targetHandle || REQUIRED_HANDLE_IDS.target, // Must match Handle IDs in GroupNode.js and CollapsedContainerNode.js
      };
    }

    // Create rerouted edge and cache original for restoration
    // CRITICAL: Handle IDs must be consistent across all node types
    return {
      ...edge,
      source: newSource,
      target: newTarget,
      sourceHandle: edge.sourceHandle || REQUIRED_HANDLE_IDS.source, // Must match Handle IDs in node components
      targetHandle: edge.targetHandle || REQUIRED_HANDLE_IDS.target, // Must match Handle IDs in node components
      data: {
        ...edge.data,
        // Cache original edge information for restoration when expanding
        originalSource: edge.data?.originalSource || edge.source,
        originalTarget: edge.data?.originalTarget || edge.target,
        originalSourceHandle: edge.data?.originalSourceHandle || edge.sourceHandle,
        originalTargetHandle: edge.data?.originalTargetHandle || edge.targetHandle,
        isRerouted: true,
      },
    };
  }).filter(edge => edge !== null); // Remove any null edges from safety check
  
  return resultEdges;
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
      stroke: edge.style?.stroke || COMPONENT_COLORS.EDGE_DEFAULT,
    },
    data: {
      ...edge.data,
      // Clean up ALL rerouting and internal edge data
      originalSource: undefined,
      originalTarget: undefined,
      originalSourceHandle: undefined,
      originalTargetHandle: undefined,
      isRerouted: false,
      isInternalEdge: undefined,
    },
  };
  
  // Restore original handle properties if they were cached and valid
  if (edge.data?.originalSourceHandle && edge.data.originalSourceHandle !== "null") {
    restored.sourceHandle = edge.data.originalSourceHandle;
  } else {
    // Use default handle if no valid cached handle
    restored.sourceHandle = REQUIRED_HANDLE_IDS.source;
  }
  
  if (edge.data?.originalTargetHandle && edge.data.originalTargetHandle !== "null") {
    restored.targetHandle = edge.data.originalTargetHandle;
  } else {
    // Use default handle if no valid cached handle
    restored.targetHandle = REQUIRED_HANDLE_IDS.target;
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
