/**
 * Shared configuration and utilities for ReactFlow components
 */

import { generateNodeColors } from './utils.js';

// ELK layout configurations
export const ELK_LAYOUT_CONFIGS = {
  mrtree: {
    'elk.algorithm': 'mrtree',
    'elk.direction': 'DOWN',
    'elk.spacing.nodeNode': 50,
    'elk.spacing.edgeNode': 20,
  },
  layered: {
    'elk.algorithm': 'layered',
    'elk.direction': 'DOWN',
    'elk.spacing.nodeNode': 30,
    'elk.layered.spacing.nodeNodeBetweenLayers': 50,
  },
  force: {
    'elk.algorithm': 'force',
    'elk.spacing.nodeNode': 100,
  },
  stress: {
    'elk.algorithm': 'stress',
    'elk.spacing.nodeNode': 100,
  },
  radial: {
    'elk.algorithm': 'radial',
    'elk.spacing.nodeNode': 100,
  },
};

// Common ReactFlow configuration
export const REACTFLOW_CONFIG = {
  fitView: true,
  nodesDraggable: true,
  nodesConnectable: true,
  elementsSelectable: true,
  maxZoom: 2,
  minZoom: 0.1,
  nodeOrigin: [0.5, 0.5],
  elevateEdgesOnSelect: true,
  disableKeyboardA11y: false,
  // CRITICAL: These settings are needed for proper group node behavior
  defaultEdgeOptions: {
    style: { strokeWidth: 2 },
  },
  // Allow nodes to be positioned outside parent bounds during layout
  translateExtent: [[-Infinity, -Infinity], [Infinity, Infinity]],
};

// Common MiniMap configuration
export const MINIMAP_CONFIG = {
  nodeStrokeWidth: 2,
  nodeStrokeColor: "#666",
  maskColor: "rgba(240, 240, 240, 0.6)",
};

// Common Background configuration
export const BACKGROUND_CONFIG = {
  color: "#f5f5f5",
  gap: 20,
};

// Default edge options
export const DEFAULT_EDGE_OPTIONS = {
  type: 'smoothstep',
  animated: false,
  style: {
    strokeWidth: 2,
    stroke: '#666666',
  },
  markerEnd: {
    type: 'arrowclosed',
    width: 20,
    height: 20,
    color: '#666666',
  },
};

// Default node style configuration
export const DEFAULT_NODE_STYLE = {
  borderRadius: '8px',
  padding: '10px',
  color: '#333',
  fontSize: '12px',
  fontWeight: '500',
  width: 200,
  height: 60,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  textAlign: 'center',
};

/**
 * Create styled node from raw node data
 */
export function createStyledNode(node, colorPalette = 'Set3', hierarchyData = null) {
  
  // For group nodes (hierarchy containers), preserve their existing style
  if (node.type === 'group') {
    const width = node.style?.width ?? 300;
    const height = node.style?.height ?? 200;
    
    return {
      ...node,
      // CRITICAL: Set both top-level width/height AND style width/height
      // ReactFlow needs both for proper internal processing
      width,
      height,
      style: {
        width,
        height,
        ...(node.style || {}),
      },
    };
  }
  
  // For regular nodes, apply standard styling
  const nodeColors = generateNodeColors(node.data?.nodeType || 'Transform', colorPalette);
  
  // Generate display label WITHOUT hierarchy information - just the node name
  let displayLabel = node.data?.label || node.id;
  // Remove hierarchy path to keep labels clean and simple
  
  // Debug: log the colors being generated
  console.log(`[createStyledNode] Node ${node.id}: nodeType=${node.data?.nodeType || 'Transform'}, gradient=${nodeColors.gradient}`);
  
  return {
    ...node,
    type: node.type || 'default', // Ensure regular nodes have a type
    data: {
      ...node.data,
      label: displayLabel,
    },
    position: { x: 0, y: 0 }, // Will be set by layout
    style: {
      ...DEFAULT_NODE_STYLE,
      color: '#fff',                       // White text for good contrast on gradients
      background: nodeColors.gradient,     // Use the gradient from generateNodeColors
      border: 'none',                      // No border for clean gradient look
      borderRadius: '6px',                 // Slightly rounded corners
      boxShadow: 'none',                   // Remove shadow - these should be text, not nodes
      fontWeight: '500',                   // Medium font weight for readability
    },
  };
}

/**
 * Create styled edge from raw edge data
 */
export function createStyledEdge(edge) {
  const result = {
    ...DEFAULT_EDGE_OPTIONS,
    ...edge,
  };
  
  // Clean up any "null" string values that come from the backend
  // This is the root cause - the backend is sending sourceHandle/targetHandle as "null" strings
  if (result.sourceHandle === "null") {
    delete result.sourceHandle;
  }
  if (result.targetHandle === "null") {
    delete result.targetHandle;
  }
  
  // Debug: Check for problematic handle values from backend
  if (edge.id === 'e7') {
    console.log('DEBUG - createStyledEdge e7 cleaned:', {
      id: result.id,
      sourceHandle: result.sourceHandle,
      targetHandle: result.targetHandle,
      sourceHandleType: typeof result.sourceHandle,
      targetHandleType: typeof result.targetHandle,
      hasSourceHandle: 'sourceHandle' in result,
      hasTargetHandle: 'targetHandle' in result
    });
  }
  
  return result;
}

/**
 * Get node color for MiniMap
 */
export function getMiniMapNodeColor(node, colorPalette = 'Set3') {
  const nodeColors = generateNodeColors(
    node.data?.nodeType || node.data?.type || 'Transform', 
    colorPalette
  );
  return nodeColors.primary;
}

/**
 * Validate hierarchy data for correctness
 */
function validateHierarchy(hierarchy, nodeAssignments, nodes) {
  const errors = [];
  const warnings = [];
  
  if (!hierarchy || !Array.isArray(hierarchy)) {
    errors.push('Hierarchy must be an array');
    return { isValid: false, errors, warnings };
  }
  
  if (!nodeAssignments || typeof nodeAssignments !== 'object') {
    errors.push('NodeAssignments must be an object');
    return { isValid: false, errors, warnings };
  }
  
  // Collect all hierarchy container IDs
  const allContainerIds = new Set();
  const collectContainerIds = (containers) => {
    containers.forEach(container => {
      if (!container.id || !container.name) {
        errors.push(`Container missing id or name: ${JSON.stringify(container)}`);
        return;
      }
      
      // Additional validation: ensure name is not empty or just whitespace
      if (typeof container.name !== 'string' || container.name.trim().length === 0) {
        errors.push(`Container ${container.id} has invalid name - cannot render container label: "${container.name}"`);
        return;
      }
      
      if (allContainerIds.has(container.id)) {
        errors.push(`Duplicate container ID: ${container.id}`);
        return;
      }
      
      allContainerIds.add(container.id);
      
      if (container.children) {
        collectContainerIds(container.children);
      }
    });
  };
  
  collectContainerIds(hierarchy);
  
  // Validate node assignments
  const nodeIds = new Set(nodes.map(n => n.id));
  const assignedContainers = new Set();
  
  for (const [nodeId, containerId] of Object.entries(nodeAssignments)) {
    if (!nodeIds.has(nodeId)) {
      errors.push(`Node assignment references unknown node: ${nodeId}`);
    }
    
    if (!allContainerIds.has(containerId)) {
      errors.push(`Node assignment references unknown container: ${containerId}`);
    }
    
    assignedContainers.add(containerId);
  }
  
  // Report empty containers as warnings (not errors)
  const emptyContainers = Array.from(allContainerIds).filter(id => !assignedContainers.has(id));
  if (emptyContainers.length > 0) {
    warnings.push(`Empty containers (this is OK for organizational hierarchy): ${emptyContainers.join(', ')}`);
  }
  
  return {
    isValid: errors.length === 0,
    errors,
    warnings,
    stats: {
      totalContainers: allContainerIds.size,
      emptyContainers: emptyContainers.length,
      assignedNodes: Object.keys(nodeAssignments).length
    }
  };
}

/**
 * Process backtrace data into hierarchy structure
/**
 * Process hierarchy data and assign hierarchy paths to nodes
 */
export function processHierarchy(graphData, selectedGrouping = '') {
  // Handle new hierarchy choices format
  let hierarchy, nodeAssignments;
  
  if (graphData.hierarchyChoices && graphData.nodeAssignments) {
    // New format with multiple hierarchy choices
    if (selectedGrouping && graphData.nodeAssignments[selectedGrouping]) {
      nodeAssignments = graphData.nodeAssignments[selectedGrouping];
      const choice = graphData.hierarchyChoices.find(c => c.id === selectedGrouping);
      hierarchy = choice ? choice.hierarchy : [];
    } else if (graphData.hierarchyChoices.length > 0) {
      // Fall back to first available choice
      const firstChoice = graphData.hierarchyChoices[0];
      hierarchy = firstChoice.hierarchy;
      nodeAssignments = graphData.nodeAssignments[firstChoice.id] || {};
    } else {
      return graphData;
    }
  } else if (graphData.hierarchy && graphData.nodeAssignments) {
    // Legacy format - backward compatibility
    hierarchy = graphData.hierarchy;
    nodeAssignments = graphData.nodeAssignments;
  } else {
    // No hierarchy data
    return graphData;
  }

  // Validate hierarchy data
  const validation = validateHierarchy(hierarchy, nodeAssignments, graphData.nodes);
  // All console logs, errors, and warnings removed for focused debugging
  if (!validation.isValid) {
    throw new Error(`Invalid hierarchy data: ${validation.errors.join('; ')}`);
  }
  
  // Build a path lookup for hierarchy nodes
  const hierarchyPaths = {};
  const hierarchyNodes = [];
  
  function buildPaths(node, parentPath = '', parentId = null, depth = 0) {
    const currentPath = parentPath ? `${parentPath} / ${node.name}` : node.name;
    hierarchyPaths[node.id] = currentPath;
    
    // All console logs removed for focused debugging
    
    // Create a hierarchy-level-based color scheme with higher opacity
    const colors = [
      'rgba(59, 130, 246, 0.25)',   // Blue - Level 0 (datacenter)
      'rgba(16, 185, 129, 0.25)',   // Green - Level 1 (building) 
      'rgba(245, 158, 11, 0.25)',   // Orange - Level 2 (floor)
      'rgba(139, 92, 246, 0.25)',   // Purple - Level 3 (room)
      'rgba(239, 68, 68, 0.25)',    // Red - Level 4 (additional nesting)
    ];
    
    const borderColors = [
      'rgb(59, 130, 246)',   // Blue
      'rgb(16, 185, 129)',   // Green 
      'rgb(245, 158, 11)',   // Orange
      'rgb(139, 92, 246)',   // Purple
      'rgb(239, 68, 68)',    // Red
    ];
    
    const backgroundColor = colors[depth % colors.length];
    const borderColor = borderColors[depth % borderColors.length];
    
    // Create a ReactFlow parent node for this hierarchy level
    const hierarchyNode = {
      id: node.id,
      type: 'group', // ReactFlow's built-in group node type
      data: {
        label: node.name,
      },
      position: { x: 0, y: 0 }, // Will be set by ELK
      parentId: parentId, // FIXED: ReactFlow v12 uses parentId instead of parentNode
      // TEMPORARILY REMOVED: extent: 'parent', // This might be causing layout issues
      style: {
        // Use background instead of backgroundColor to override ReactFlow defaults
        background: backgroundColor,
        border: `3px solid ${borderColor}`,
        borderRadius: '8px',
        padding: '12px',
        // Ensure labels are visible with stronger contrast
        fontSize: '14px',
        fontWeight: 'bold',
        color: borderColor,
        // Override any default ReactFlow group styling
        zIndex: 1,
        boxSizing: 'border-box',
      },
    };
    
    hierarchyNodes.push(hierarchyNode);
    
    if (node.children) {
      node.children.forEach(child => buildPaths(child, currentPath, node.id, depth + 1));
    }
  }
  
  // Build paths and create hierarchy nodes for all hierarchy levels
  hierarchy.forEach(rootNode => {
    buildPaths(rootNode, '', null, 0); // Start with depth 0
  });
  
  // Assign hierarchy paths and parent relationships to graph nodes
  const processedNodes = graphData.nodes.map(node => {
    const assignment = nodeAssignments[node.id];
    
    if (assignment && hierarchyPaths[assignment]) {
      const processedNode = {
        ...node,
        data: {
          ...node.data,
          hierarchyPath: hierarchyPaths[assignment],
          hierarchyAssignment: assignment,
        },
        parentId: assignment, // FIXED: ReactFlow v12 uses parentId instead of parentNode
        extent: 'parent', // Constrain within parent bounds
      };
      
      return processedNode;
    }
    
    return node;
  });
  
  const result = {
    ...graphData,
    nodes: [...hierarchyNodes, ...processedNodes], // Hierarchy nodes first, then graph nodes
  };
  
  // Debug: Log what we're returning from processHierarchy
  console.log('[processHierarchy] Returning nodes:', result.nodes.map(n => ({ id: n.id, type: n.type, label: n.data?.label })));
  
  return result;
}

/**
 * Process graph data into styled nodes and edges
 */
export async function processGraphData(graphData, colorPalette, currentLayout, applyLayout, currentGrouping = '') {
  if (!graphData?.nodes?.length) {
    console.warn('No nodes found, returning empty result');
    return { nodes: [], edges: [] };
  }

  // Use the selected grouping hierarchy, or fall back to first available one
  let selectedGrouping = currentGrouping;
  if (!selectedGrouping && graphData.hierarchyChoices && graphData.hierarchyChoices.length > 0) {
    selectedGrouping = graphData.hierarchyChoices[0].id;
  }

  // The hierarchy and nodeAssignments are now generated by the Rust code
  // We just need to process the existing hierarchy structure
  const processedGraphData = processHierarchy(graphData, selectedGrouping);
  
  // CRITICAL: Only apply createStyledNode to non-group nodes
  // Group nodes (hierarchy containers) are already properly styled by processHierarchy
  const processedNodes = processedGraphData.nodes.map(node => {
    if (node.type === 'group') {
      // Group nodes are already styled - don't re-process them
      console.log(`[processGraphData] Keeping group node: ${node.id} (${node.data?.label})`);
      return node;
    }
    // Only regular nodes need styling
    console.log(`[processGraphData] Styling regular node: ${node.id} (${node.data?.label})`);
    return createStyledNode(node, colorPalette, processedGraphData.hierarchy);
  });
  
  // Debug: Log all final nodes to check for duplicates
  console.log('[processGraphData] Final node list:');
  
  const processedEdges = (processedGraphData.edges || []).map(edge => createStyledEdge(edge));

  // Apply layout
  const layoutResult = await applyLayout(processedNodes, processedEdges, currentLayout);

  return {
    nodes: layoutResult.nodes,
    edges: layoutResult.edges,
  };
}
