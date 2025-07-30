/**
 * Simple ELK Layout Integration
 * 
 * Provides        layoutOptions: {
          'elk.spacing.nodeNode': 30,
          'elk.algorithm': 'layered',
          'elk.layered.spacing.nodeNodeBetweenLayers': 40,
          'elk.layered.spacing.borderToNode': 20,
        },
        children: buildElkHierarchy(container.id),
      };aph layout using ELK algorithms with shared configuration
 */

import { ELK_LAYOUT_CONFIGS } from './reactFlowConfig.js';
import { filterNodesByType, filterNodesByParent, filterNodesExcludingType } from './constants.js';

let ELK = null;

// Load ELK dynamically
async function loadELK() {
  if (ELK) return ELK;
  
  try {
    const elkModule = await import('elkjs');
    ELK = new elkModule.default();
    return ELK;
  } catch (error) {
    console.error('Failed to load ELK:', error);
    return null;
  }
}

export async function applyLayout(nodes, edges, layoutType = 'mrtree') {
  // Remove the detailed logging for cleaner output
  
  const elk = await loadELK();
  
  if (!elk) {
    console.error('ELK not available - this is a critical error');
    throw new Error('ELK layout engine failed to load');
  }

  const hierarchyNodes = filterNodesByType(nodes, 'group');
  const regularNodes = filterNodesExcludingType(nodes, 'group');

  // Build ELK hierarchy structure
  function buildElkHierarchy(parentId = null) {
    const children = [];
    // Add hierarchy containers at this level
    const containers = filterNodesByParent(hierarchyNodes, parentId);
    containers.forEach(container => {
      // Recursively build children for this container
      const childElkNodes = buildElkHierarchy(container.id);
      const elkContainer = {
        ...container,  // PRESERVE all original properties (style, data, type, etc.)
        id: container.id,
        // Let ELK calculate container size - temporarily remove padding to test
        layoutOptions: {
          ...ELK_LAYOUT_CONFIGS[layoutType], // Use the selected layout algorithm!
          'elk.spacing.nodeNode': 20,
          'elk.spacing.edgeNode': 15,
          'elk.spacing.edgeEdge': 10,
        },
        children: childElkNodes,
      };
      children.push(elkContainer);
    });
    // Add regular nodes at this level - let ELK handle sizing
    const levelNodes = filterNodesByParent(regularNodes, parentId);
    levelNodes.forEach(node => {
      const elkNode = {
        ...node,  // PRESERVE all original properties (style, data, type, etc.)
        id: node.id,
        // Use existing dimensions if available, otherwise let ELK decide
        width: node.style?.width || node.width || 200,
        height: node.style?.height || node.height || 60,
      };
      children.push(elkNode);
    });
    return children;
  }

  // Build the ELK graph with hierarchy - let ELK handle all sizing and positioning
  const elkGraph = {
    id: 'root',
    layoutOptions: {
      ...ELK_LAYOUT_CONFIGS[layoutType],
      'elk.padding': '[top=20,left=20,bottom=20,right=20]', // Root level padding for canvas margins
      'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
      'elk.spacing.nodeNode': 40, // Spacing between top-level containers
      'elk.spacing.edgeNode': 20,
      'elk.spacing.edgeEdge': 15,
    },  
    children: buildElkHierarchy(null), // Start with no parent (top level)
    edges: edges.map(edge => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target],
    })),
  };
 
  // Helper function to get all node IDs from hierarchy
  function getAllNodeIds(nodes) {
    const ids = [];
    nodes.forEach(node => {
      ids.push(node.id);
      if (node.children) {
        ids.push(...getAllNodeIds(node.children));
      }
    });
    return ids;
  }
  
  // Only log missing nodes, not the full list
  elkGraph.edges.forEach(edge => {
    const allNodeIds = getAllNodeIds(elkGraph.children);
    const sourceExists = allNodeIds.includes(edge.sources[0]);
    const targetExists = allNodeIds.includes(edge.targets[0]);
    if (!sourceExists || !targetExists) {
      console.error(`[ELK] Missing node for edge ${edge.sources[0]} -> ${edge.targets[0]}. Source exists: ${sourceExists}, Target exists: ${targetExists}`);
    }
  });

  try {
    const layoutResult = await elk.layout(elkGraph);

    // Apply positions back to nodes
    function applyPositions(elkNodes, depth = 0) {
      const layoutedNodes = [];
      elkNodes.forEach(elkNode => {
        const reactFlowNode = nodes.find(n => n.id === elkNode.id);
        if (reactFlowNode) {
          // Use ELK's results exactly as calculated
          const processedNode = {
            ...reactFlowNode,
            width: elkNode.width,
            height: elkNode.height,
            position: {
              x: elkNode.x || 0,
              y: elkNode.y || 0,
            },
            data: {
              ...reactFlowNode.data,
              nodeStyle: {
                ...reactFlowNode.style,
                width: elkNode.width,
                height: elkNode.height,
              },
            },
            style: {
              width: elkNode.width,
              height: elkNode.height,
            },
            extent: reactFlowNode.parentId ? 'parent' : undefined,
          };
          layoutedNodes.push(processedNode);
        }
        
        // Recursively apply positions to children
        if (elkNode.children) {
          layoutedNodes.push(...applyPositions(elkNode.children, depth + 1));
        }
      });
      return layoutedNodes;
    }
    const layoutedNodes = applyPositions(layoutResult.children || []);

    // CRITICAL: Sort nodes so parents come before children (ReactFlow v12 requirement)
    const sortedNodes = [];
    const nodeMap = new Map(layoutedNodes.map(node => [node.id, node]));
    const visited = new Set();
    
    function addNodeAndParents(nodeId) {
      if (visited.has(nodeId)) return;
      
      const node = nodeMap.get(nodeId);
      if (!node) return;
      
      if (node.parentId && !visited.has(node.parentId)) {
        addNodeAndParents(node.parentId);
      }
      
      visited.add(nodeId);
      sortedNodes.push(node);
    }
    
    layoutedNodes.forEach(node => addNodeAndParents(node.id));
    
    return {
      nodes: sortedNodes,
      edges: edges,
    };

  } catch (error) {
    console.error('ELK layout failed:', error);
    throw error; // Let the error bubble up instead of silently falling back
  }
}

/**
 * Apply layout readjustment for collapsed containers only
 * This function will only reposition containers while keeping other nodes fixed
 */
export async function applyLayoutForCollapsedContainers(displayNodes, edges, layoutType = 'mrtree', changedContainerId = null) {
  const elk = await loadELK();
  
  if (!elk) {
    console.error('ELK not available for container layout');
    throw new Error('ELK layout engine failed to load');
  }

  // Find only container nodes for repositioning (both group and collapsedContainer types)
  const containerNodes = displayNodes.filter(node => 
    node.type === 'group' || node.type === 'collapsedContainer'
  );
  
  if (containerNodes.length === 0) {
    return { nodes: displayNodes, edges };
  }

  // Create ELK nodes for containers only
  const elkContainers = containerNodes.map(container => {
    let width, height;
    
    // Check if this is a collapsed container and use appropriate dimensions
    if (container.type === 'collapsedContainer') {
      // Use the small collapsed dimensions
      width = container.width || 180;
      height = container.height || 60;
    } else {
      // Use the original large dimensions for expanded containers
      width = parseFloat(container.style?.width?.toString().replace('px', '')) || 400;
      height = parseFloat(container.style?.height?.toString().replace('px', '')) || 300;
    }
    
    const elkContainer = {
      id: container.id,
      width: width,
      height: height,
    };
    
    // If this container wasn't the one that changed, try to keep its position fixed
    if (changedContainerId && container.id !== changedContainerId) {
      elkContainer.x = container.position.x;
      elkContainer.y = container.position.y;
      elkContainer.layoutOptions = {
        'elk.position.x': container.position.x.toString(),
        'elk.position.y': container.position.y.toString(),
        'elk.nodeSize.constraints': 'FIXED_POS',
        'elk.nodeSize.options': 'FIXED_POS'
      };
    } else {
      // Allow the changed container to find a new position
      elkContainer.layoutOptions = {
        'elk.nodeSize.constraints': '',
        'elk.nodeSize.options': ''
      };
    }
    
    return elkContainer;
  });

  // Create simple ELK graph for container layout
  const elkGraph = {
    id: 'container_root',
    layoutOptions: {
      ...ELK_LAYOUT_CONFIGS[layoutType],
      'elk.spacing.nodeNode': 80, // Reduced spacing for better collapsed container layout
      'elk.spacing.componentComponent': 60, // Reduced spacing
      'elk.partitioning.activate': 'false'
    },
    children: elkContainers,
    edges: [] // No edges needed for simple container repositioning
  };

  try {
    const layoutResult = await elk.layout(elkGraph);
    
    // Apply new positions only to container nodes in displayNodes
    const updatedDisplayNodes = displayNodes.map(node => {
      if (node.type === 'group' || node.type === 'collapsedContainer') {
        const elkContainer = layoutResult.children?.find(c => c.id === node.id);
        if (elkContainer) {
          // Only update position if this was the changed container OR if no specific container was changed
          if (!changedContainerId || node.id === changedContainerId) {
            return {
              ...node,
              position: {
                x: elkContainer.x || node.position.x,
                y: elkContainer.y || node.position.y
              }
            };
          }
        }
      }
      return node;
    });
    
    return {
      nodes: updatedDisplayNodes,
      edges: edges,
    };
  } catch (error) {
    console.error('Container layout with ELK failed:', error);
    return { nodes: displayNodes, edges }; // Fallback to original
  }
}
