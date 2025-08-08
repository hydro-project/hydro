/**
 * ELK State Manager
 * 
 * This module provides wrapper functions that ensure all ELK layout interactions
 * are consistent with the VisualState as the single source of truth.
 * 
 * Key principle: ELK should only ever calculate layouts based on the exact
 * visual state requirements, and return results that perfectly match those requirements.
 */

import { 
  getELKConfig, 
  getContainerELKConfig, 
  createFixedPositionOptions, 
  createFreePositionOptions 
} from './elkConfig.js';
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

/**
 * Create an ELK state manager that wraps all ELK layout interactions
 * with VisualState as the single source of truth.
 * 
 * @returns {Object} ELK wrapper functions
 */
export function createELKStateManager() {
  
  /**
   * Calculate full layout for dimension caching (expanded state).
   * This is used to populate the dimension cache with expanded container sizes.
   * 
   * @param {Array} allNodes - Complete source nodes
   * @param {Array} allEdges - Complete source edges  
   * @param {string} layoutType - Layout algorithm type
   * @returns {Promise<Object>} Layout result with expanded dimensions
   */
  async function calculateFullLayout(allNodes, allEdges, layoutType = 'mrtree') {
    console.log(`[ELKStateManager] 🏗️ FULL_LAYOUT: Calculating expanded layout for dimension caching`);
    
    const elk = await loadELK();
    
    if (!elk) {
      console.error('ELK not available for full layout');
      throw new Error('ELK layout engine failed to load');
    }

    const hierarchyNodes = filterNodesByType(allNodes, 'group');
    const regularNodes = filterNodesExcludingType(allNodes, 'group');

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
      id: 'full_layout_root',
      layoutOptions: getContainerELKConfig(layoutType, 'root'),
      children: buildElkHierarchy(null), // Start with no parent (top level)
      edges: allEdges.map(edge => ({
        id: edge.id,
        sources: [edge.source],
        targets: [edge.target],
      })),
    };

    try {
      const layoutResult = await elk.layout(elkGraph);

      // Apply positions back to nodes
      function applyPositions(elkNodes, depth = 0) {
        const layoutedNodes = [];
        elkNodes.forEach(elkNode => {
          const reactFlowNode = allNodes.find(n => n.id === elkNode.id);
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
        edges: allEdges,
      };

    } catch (error) {
      console.error('[ELKStateManager] Full layout failed:', error);
      throw error;
    }
  }
  
  /**
   * Calculate layout based on VisualState.
   * This is the core function that applies visual state filtering and calculates
   * the exact layout that matches the visual state requirements.
   * 
   * @param {Array} allNodes - Complete source nodes (never filtered)
   * @param {Array} allEdges - Complete source edges (never filtered)
   * @param {VisualState} visualState - The current visual state
   * @param {string} layoutType - Layout algorithm type
   * @param {Map} dimensionsCache - Cache of expanded container dimensions
   * @returns {Promise<Object>} Layout result matching visual state
   */
  async function calculateVisualLayout(allNodes, allEdges, visualState, layoutType = 'mrtree', dimensionsCache) {
    console.log(`[ELKStateManager] 🎯 VISUAL_LAYOUT: Calculating layout for visual state`);
    
    const elk = await loadELK();
    
    if (!elk) {
      console.error('ELK not available for visual layout');
      throw new Error('ELK layout engine failed to load');
    }

    // STEP 1: Filter nodes based on visual state
    console.log(`[ELKStateManager] 🔍 FILTERING: Applying visual state filters...`);
    const visibleNodes = allNodes.filter(node => {
      if (node.type === 'group' || node.type === 'collapsedContainer') {
        const containerState = visualState.getContainerState(node.id);
        if (containerState === 'hidden') {
          console.log(`[ELKStateManager] 👻 FILTERING: Container ${node.id} → HIDDEN (filtered out)`);
          return false;
        }
        return true;
      } else {
        const nodeState = visualState.getNodeState(node.id);
        if (nodeState === 'hidden') {
          console.log(`[ELKStateManager] 👻 FILTERING: Node ${node.id} → HIDDEN (filtered out)`);
          return false;
        }
        return true;
      }
    });

    // STEP 2: Filter edges based on visual state AND node visibility
    console.log(`[ELKStateManager] 🔗 FILTERING: Applying edge filters...`);
    const visibleNodeIds = new Set(visibleNodes.map(node => node.id));
    const visibleEdges = allEdges.filter(edge => {
      // First check edge's own visibility state
      const edgeState = visualState.getEdgeState(edge.id);
      if (edgeState === 'hidden') {
        console.log(`[ELKStateManager] 👻 FILTERING: Edge ${edge.id} → HIDDEN (filtered out)`);
        return false;
      }
      
      // Then check if both source and target nodes are visible
      const sourceVisible = visibleNodeIds.has(edge.source);
      const targetVisible = visibleNodeIds.has(edge.target);
      
      if (!sourceVisible || !targetVisible) {
        console.log(`[ELKStateManager] 🔗 FILTERING: Edge ${edge.id} → FILTERED (source:${sourceVisible}, target:${targetVisible})`);
        return false;
      }
      
      return true;
    });

    // Find container nodes for positioning
    const containerNodes = visibleNodes.filter(node => 
      node.type === 'group' || node.type === 'collapsedContainer'
    );
    
    if (containerNodes.length === 0) {
      console.log(`[ELKStateManager] ⚠️ LAYOUT: No containers to layout`);
      return { nodes: visibleNodes, edges: visibleEdges };
    }

    // STEP 3: Validate that all containers have explicit states
    containerNodes.forEach(container => {
      const containerState = visualState.getContainerState(container.id);
      if (!['expanded', 'collapsed'].includes(containerState)) {
        throw new Error(`Container ${container.id} has invalid state '${containerState}'. Must specify 'expanded' or 'collapsed'.`);
      }
    });

    // STEP 4: Set dimensions based on explicit states
    console.log(`[ELKStateManager] 📐 DIMENSIONS: Setting container dimensions based on visual state...`);
    const containersWithExplicitDimensions = containerNodes.map(container => {
      const containerState = visualState.getContainerState(container.id);
      let width, height;
      
      if (containerState === 'collapsed') {
        width = 180;
        height = 60;
        console.log(`[ELKStateManager] ❌ DIMENSIONS: ${container.id} → COLLAPSED (${width}x${height})`);
      } else if (containerState === 'expanded') {
        const cachedDimensions = dimensionsCache.get(container.id);
        if (!cachedDimensions) {
          throw new Error(`Container ${container.id} requested as 'expanded' but no cached dimensions found`);
        }
        width = cachedDimensions.width;
        height = cachedDimensions.height;
        console.log(`[ELKStateManager] ✅ DIMENSIONS: ${container.id} → EXPANDED (${width}x${height}) from cache`);
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
    });

    // STEP 5: Create ELK nodes with explicit dimensions
    console.log(`[ELKStateManager] 🏗️ ELK_GRAPH: Creating ELK nodes with explicit dimensions...`);
    const elkContainers = containersWithExplicitDimensions.map(container => ({
      id: container.id,
      width: container.width,
      height: container.height,
      layoutOptions: createFreePositionOptions() // Let ELK find optimal positions
    }));

    // STEP 6: Calculate rerouted edges for ELK layout
    console.log(`[ELKStateManager] 🔗 ELK_GRAPH: Rerouting edges for container layout...`);
    
    // Create a map of container IDs for quick lookup
    const containerIds = new Set(containerNodes.map(c => c.id));
    
    // Helper function to find the container for a node
    function findNodeContainer(nodeId, visited = new Set()) {
      if (visited.has(nodeId)) return null; // Prevent infinite loops
      visited.add(nodeId);
      
      // If this nodeId is itself a container, return it
      if (containerIds.has(nodeId)) {
        return nodeId;
      }
      
      // Find the node and check its parent
      const node = allNodes.find(n => n.id === nodeId);
      if (!node || !node.parentId) {
        return null; // Node has no parent container
      }
      
      // Check if parent is a container
      if (containerIds.has(node.parentId)) {
        return node.parentId;
      }
      
      // Recursively check parent's container
      return findNodeContainer(node.parentId, visited);
    }
    
    // Reroute edges to connect containers instead of child nodes
    const reroutedEdges = [];
    const edgeSet = new Set(); // Prevent duplicate edges
    
    visibleEdges.forEach(edge => {
      const sourceContainer = findNodeContainer(edge.source);
      const targetContainer = findNodeContainer(edge.target);
      
      // Only include edge if both endpoints have containers and they're different
      if (sourceContainer && targetContainer && sourceContainer !== targetContainer) {
        const edgeKey = `${sourceContainer}->${targetContainer}`;
        if (!edgeSet.has(edgeKey)) {
          edgeSet.add(edgeKey);
          reroutedEdges.push({
            id: `${edge.id}_rerouted_${sourceContainer}_${targetContainer}`,
            source: sourceContainer,
            target: targetContainer,
          });
        }
      }
    });
    
    // Convert rerouted edges to ELK format
    const elkEdges = reroutedEdges.map(edge => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target],
    }));
    
    console.log(`[ELKStateManager] 🔗 ELK_GRAPH: ${elkEdges.length} rerouted edges for ELK layout`);

    // Create ELK graph with edges for proper container positioning
    const elkGraph = {
      id: 'visual_layout_root',
      layoutOptions: getContainerELKConfig(layoutType, 'collapsed'),
      children: elkContainers,
      edges: elkEdges // Include edges so ELK can optimize container positions
    };

    // LOG ELK INPUT
    console.log(`[ELKStateManager] 🎯 ELK_INPUT:`);
    elkContainers.forEach(container => {
      console.log(`[ELKStateManager] 🎯 INPUT: ${container.id}: ${container.width}x${container.height}`);
    });

    try {
      console.log(`[ELKStateManager] 🚀 ELK_LAYOUT: Calling ELK layout engine...`);
      const layoutResult = await elk.layout(elkGraph);
      console.log(`[ELKStateManager] ✅ ELK_LAYOUT: Layout completed successfully`);
      
      // DEBUG: Log complete ELK output
      console.log(`[ELKStateManager] 🔍 ELK_OUTPUT: Complete layout result:`);
      if (layoutResult.children) {
        console.log(`[ELKStateManager] 🔍 ELK_NODES:`);
        layoutResult.children.forEach(node => {
          console.log(`[ELKStateManager] 🔍   Node ${node.id}: x=${node.x}, y=${node.y}, w=${node.width}, h=${node.height}`);
        });
      }
      if (layoutResult.edges) {
        console.log(`[ELKStateManager] 🔍 ELK_EDGES:`);
        layoutResult.edges.forEach(edge => {
          console.log(`[ELKStateManager] 🔍   Edge ${edge.id}: ${edge.sources} -> ${edge.targets}`);
          if (edge.sections) {
            console.log(`[ELKStateManager] 🔍     Sections (${edge.sections.length}):`);
            edge.sections.forEach((section, i) => {
              console.log(`[ELKStateManager] 🔍       Section ${i}: start=(${section.startPoint?.x},${section.startPoint?.y}) end=(${section.endPoint?.x},${section.endPoint?.y})`);
              if (section.bendPoints && section.bendPoints.length > 0) {
                console.log(`[ELKStateManager] 🔍       Bend points:`, section.bendPoints.map(bp => `(${bp.x},${bp.y})`).join(', '));
              }
            });
          }
        });
      } else {
        console.log(`[ELKStateManager] 🔍 ELK_EDGES: No edge routing information provided by ELK`);
      }
      
      // Apply new positions to container nodes
      console.log(`[ELKStateManager] 🎨 POSITIONING: Applying ELK-calculated positions...`);
      const updatedNodes = visibleNodes.map(node => {
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
      
      console.log(`[ELKStateManager] 🏁 COMPLETE: Layout finished. Nodes: ${updatedNodes.length}, Edges: ${visibleEdges.length}`);
      
      return {
        nodes: updatedNodes,
        edges: visibleEdges,
        elkResult: layoutResult // Include raw ELK result for edge routing
      };
    } catch (error) {
      console.error(`[ELKStateManager] ❌ LAYOUT_FAILED:`, error);
      throw error;
    }
  }
  
  /**
   * Calculate simple container repositioning layout.
   * Used for quick adjustments when only container positions need to change.
   * 
   * @param {Array} displayNodes - Current display nodes
   * @param {Array} displayEdges - Current display edges
   * @param {string} layoutType - Layout algorithm type
   * @param {string} changedContainerId - ID of container that changed (optional)
   * @returns {Promise<Object>} Updated layout result
   */
  async function calculateContainerRepositioning(displayNodes, displayEdges, layoutType = 'mrtree', changedContainerId = null) {
    console.log(`[ELKStateManager] 🔄 REPOSITIONING: Calculating container repositioning`);
    
    const elk = await loadELK();
    
    if (!elk) {
      console.error('ELK not available for container repositioning');
      throw new Error('ELK layout engine failed to load');
    }

    // Find only container nodes for repositioning (both group and collapsedContainer types)
    const containerNodes = displayNodes.filter(node => 
      node.type === 'group' || node.type === 'collapsedContainer'
    );
    
    if (containerNodes.length === 0) {
      return { nodes: displayNodes, edges: displayEdges };
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
      id: 'container_repositioning_root',
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
        edges: displayEdges,
      };
    } catch (error) {
      console.error('[ELKStateManager] Container repositioning failed:', error);
      return { nodes: displayNodes, edges: displayEdges }; // Fallback to original
    }
  }
  
  return {
    // Primary layout functions
    calculateFullLayout,
    calculateVisualLayout,
    calculateContainerRepositioning,
    
    // Utility to check if ELK manager is ready
    isReady: async () => {
      const elk = await loadELK();
      return Boolean(elk);
    }
  };
}

/**
 * Validation helper to ensure ELK input matches expected visual state.
 * This can be used in development to catch input preparation issues.
 * 
 * @param {Array} elkNodes - ELK input nodes
 * @param {Array} elkEdges - ELK input edges
 * @param {VisualState} visualState - Expected visual state
 * @returns {Object} Validation result
 */
export function validateELKInput(elkNodes, elkEdges, visualState) {
  const issues = [];
  
  // Check that ELK nodes match visual state expectations
  elkNodes.forEach(elkNode => {
    const containerState = visualState.getContainerState(elkNode.id);
    
    // Validate dimensions match expected state
    if (containerState === 'collapsed') {
      if (elkNode.width !== 180 || elkNode.height !== 60) {
        issues.push(`ELK Node ${elkNode.id}: Expected collapsed dimensions (180x60), got (${elkNode.width}x${elkNode.height})`);
      }
    }
  });
  
  // Check for edges with missing nodes
  const elkNodeIds = new Set(elkNodes.map(n => n.id));
  elkEdges.forEach(edge => {
    edge.sources.forEach(sourceId => {
      if (!elkNodeIds.has(sourceId)) {
        issues.push(`ELK Edge ${edge.id}: Source node ${sourceId} not found in ELK nodes`);
      }
    });
    edge.targets.forEach(targetId => {
      if (!elkNodeIds.has(targetId)) {
        issues.push(`ELK Edge ${edge.id}: Target node ${targetId} not found in ELK nodes`);
      }
    });
  });
  
  return {
    isValid: issues.length === 0,
    issues,
    summary: issues.length === 0 
      ? 'ELK input matches visual state' 
      : `${issues.length} input validation issues found`
  };
}
