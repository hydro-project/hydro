/**
 * Simple ELK Layout Integration
 * 
 * Provides graph layout using ELK algorithms with centralized configuration from elkConfig.js
 */

import { 
  getELKConfig, 
  getContainerELKConfig, 
  createFixedPositionOptions, 
  createFreePositionOptions 
} from './elkConfig.js';
import { filterNodesByType, filterNodesByParent, filterNodesExcludingType } from './constants.js';

let ELK = null;

// Cache for storing original expanded container dimensions
// This ensures we always use the correct expanded dimensions for layout calculations
const containerDimensionsCache = new Map();

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
        // Let ELK calculate container size - use centralized container config
        layoutOptions: getContainerELKConfig(layoutType, 'hierarchy'),
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

  // Build the ELK graph with hierarchy - use centralized root config
  const elkGraph = {
    id: 'root',
    layoutOptions: getContainerELKConfig(layoutType, 'root'),
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

    // CRITICAL: Cache the expanded dimensions of all group nodes for later use
    // This ensures we always have the correct expanded dimensions for layout calculations
    layoutedNodes.forEach(node => {
      if (node.type === 'group') {
        console.log(`[Layout] 💾 CACHING: ${node.id} → ${node.width}x${node.height}`);
        containerDimensionsCache.set(node.id, {
          width: node.width,
          height: node.height
        });
      }
    });

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
      elkContainer.layoutOptions = createFixedPositionOptions(
        container.position.x, 
        container.position.y
      );
    } else {
      // Allow the changed container to find a new position
      elkContainer.layoutOptions = createFreePositionOptions();
    }
    
    return elkContainer;
  });

  // Create simple ELK graph for container layout
  const elkGraph = {
    id: 'container_root',
    layoutOptions: getContainerELKConfig(layoutType, 'collapsed'),
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

/**
 * Layout wrapper with explicit container state specification
 * This function forces callers to be explicit about the intended state of each container
 */
export async function applyLayoutWithExplicitContainerStates(
  displayNodes, 
  edges, 
  containerStates, // { containerId: 'expanded' | 'collapsed' | 'hidden' }
  layoutType = 'mrtree'
) {
  console.log(`[Layout] 🔧 EXPLICIT LAYOUT: Container states:`, containerStates);
  
  const elk = await loadELK();
  
  if (!elk) {
    console.error('ELK not available for explicit layout');
    throw new Error('ELK layout engine failed to load');
  }

  // Find only container nodes for repositioning
  const containerNodes = displayNodes.filter(node => 
    node.type === 'group' || node.type === 'collapsedContainer'
  );
  
  if (containerNodes.length === 0) {
    return { nodes: displayNodes, edges };
  }

  // STEP 1: Validate that all containers have explicit states
  containerNodes.forEach(container => {
    if (!containerStates.hasOwnProperty(container.id)) {
      throw new Error(`Container ${container.id} missing explicit state. Must specify 'expanded', 'collapsed', or 'hidden'.`);
    }
  });

  // STEP 2: Set dimensions based on explicit states
  console.log(`[Layout] 📐 EXPLICIT: Setting dimensions based on explicit states...`);
  const containersWithExplicitDimensions = containerNodes.map(container => {
    const state = containerStates[container.id];
    let width, height;
    
    if (state === 'collapsed') {
      width = 180;
      height = 60;
      console.log(`[Layout] ❌ EXPLICIT: ${container.id} → COLLAPSED (${width}x${height})`);
    } else if (state === 'expanded') {
      const cachedDimensions = containerDimensionsCache.get(container.id);
      if (!cachedDimensions) {
        throw new Error(`Container ${container.id} requested as 'expanded' but no cached dimensions found`);
      }
      width = cachedDimensions.width;
      height = cachedDimensions.height;
      console.log(`[Layout] ✅ EXPLICIT: ${container.id} → EXPANDED (${width}x${height}) from cache`);
    } else if (state === 'hidden') {
      // Hidden containers are filtered out below
      console.log(`[Layout] 👻 EXPLICIT: ${container.id} → HIDDEN (will be filtered out)`);
      return null;
    } else {
      throw new Error(`Invalid container state '${state}' for ${container.id}. Must be 'expanded', 'collapsed', or 'hidden'.`);
    }
    
    return {
      ...container,
      width: width,
      height: height,
      style: {
        ...container.style,
        width: width,
        height: height,
      }
    };
  }).filter(container => container !== null); // Remove hidden containers

  // STEP 3: Create ELK nodes - all free positioning since we're being explicit about dimensions
  console.log(`[Layout] 🏗️ EXPLICIT: Creating ELK nodes with explicit dimensions...`);
  const elkContainers = containersWithExplicitDimensions.map(container => ({
    id: container.id,
    width: container.width,
    height: container.height,
    layoutOptions: createFreePositionOptions() // Let ELK find optimal positions
  }));

  // Create ELK graph
  const elkGraph = {
    id: 'explicit_container_root',
    layoutOptions: getContainerELKConfig(layoutType, 'collapsed'),
    children: elkContainers,
    edges: []
  };

  // LOG ELK INPUT
  console.log(`[Layout] 🎯 EXPLICIT LAYOUT - ELK INPUT:`);
  elkContainers.forEach(container => {
    console.log(`[Layout] 🎯 EXPLICIT: ${container.id}: ${container.width}x${container.height}`);
  });

  try {
    console.log(`[Layout] 🚀 EXPLICIT: Calling ELK layout...`);
    const layoutResult = await elk.layout(elkGraph);
    console.log(`[Layout] ✅ EXPLICIT: ELK layout completed successfully`);
    
    // Apply new positions to all container nodes
    console.log(`[Layout] 🎨 EXPLICIT: Applying new positions...`);
    const updatedDisplayNodes = displayNodes.map(node => {
      if (node.type === 'group' || node.type === 'collapsedContainer') {
        const elkContainer = layoutResult.children?.find(c => c.id === node.id);
        if (elkContainer) {
          return {
            ...node,
            position: {
              x: elkContainer.x || node.position.x,
              y: elkContainer.y || node.position.y
            }
          };
        }
      }
      return node;
    });
    
    console.log(`[Layout] 🏁 EXPLICIT: Layout complete`);
    return {
      nodes: updatedDisplayNodes,
      edges: edges,
    };
  } catch (error) {
    console.error(`[Layout] ❌ EXPLICIT LAYOUT FAILED:`, error);
    return { nodes: displayNodes, edges };
  }
}

/**
 * Clear the container dimensions cache when graph data changes
 * This should be called whenever new graph data is loaded
 */
export function clearContainerDimensionsCache() {
  containerDimensionsCache.clear();
}
