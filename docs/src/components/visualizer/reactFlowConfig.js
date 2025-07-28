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
  console.log(`Creating styled node for ${node.id}, type: ${node.type}`);
  
  // For group nodes (hierarchy containers), preserve their existing style
  if (node.type === 'group') {
    console.log(`Group node ${node.id} - preserving existing style:`, node.style);
    return {
      ...node,
      // Don't override the style for group nodes - they already have color styling
    };
  }
  
  // For regular nodes, apply standard styling
  const nodeColors = generateNodeColors(node.data?.nodeType || 'Transform', colorPalette);
  
  // Generate display label with hierarchy information
  let displayLabel = node.data?.label || node.id;
  if (hierarchyData && node.data?.hierarchyPath) {
    displayLabel = `${node.data.hierarchyPath} > ${displayLabel}`;
  }
  
  console.log(`Regular node ${node.id} - applying standard styling`);
  return {
    ...node,
    data: {
      ...node.data,
      label: displayLabel,
    },
    position: { x: 0, y: 0 }, // Will be set by layout
    style: {
      ...DEFAULT_NODE_STYLE,
      background: nodeColors.gradient,
      border: `2px solid ${nodeColors.border}`,
    },
  };
}

/**
 * Create styled edge from raw edge data
 */
export function createStyledEdge(edge) {
  return {
    ...edge,
    ...DEFAULT_EDGE_OPTIONS,
  };
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
 * Process hierarchy data and assign hierarchy paths to nodes
 */
export function processHierarchy(graphData) {
  console.log('=== HIERARCHY PROCESSING DEBUG START ===');
  console.log('Input graphData:', {
    nodes: graphData.nodes?.map(n => ({ id: n.id, label: n.data?.label })),
    hierarchy: graphData.hierarchy,
    nodeAssignments: graphData.nodeAssignments
  });

  if (!graphData.hierarchy || !graphData.nodeAssignments) {
    console.log('No hierarchy or nodeAssignments found, returning original data');
    console.log('=== HIERARCHY PROCESSING DEBUG END ===');
    return graphData;
  }

  // Validate hierarchy data
  const validation = validateHierarchy(graphData.hierarchy, graphData.nodeAssignments, graphData.nodes);
  console.log('Hierarchy validation result:', validation);
  
  if (!validation.isValid) {
    console.error('❌ Hierarchy validation failed:', validation.errors);
    throw new Error(`Invalid hierarchy data: ${validation.errors.join('; ')}`);
  }
  
  if (validation.warnings.length > 0) {
    console.warn('⚠️ Hierarchy warnings:', validation.warnings);
  }

  const { hierarchy, nodeAssignments } = graphData;
  
  // Build a path lookup for hierarchy nodes
  const hierarchyPaths = {};
  const hierarchyNodes = [];
  
  function buildPaths(node, parentPath = '', parentId = null, depth = 0) {
    const currentPath = parentPath ? `${parentPath} / ${node.name}` : node.name;
    hierarchyPaths[node.id] = currentPath;
    
    console.log(`Building hierarchy node: id=${node.id}, name=${node.name}, parentId=${parentId}, path=${currentPath}, depth=${depth}`);
    
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
    
    console.log('Created hierarchy node:', hierarchyNode);
    hierarchyNodes.push(hierarchyNode);
    
    if (node.children) {
      console.log(`Processing ${node.children.length} children of ${node.id}`);
      node.children.forEach(child => buildPaths(child, currentPath, node.id, depth + 1));
    }
  }
  
  // Build paths and create hierarchy nodes for all hierarchy levels
  hierarchy.forEach(rootNode => {
    console.log('Processing root hierarchy node:', rootNode.id);
    buildPaths(rootNode, '', null, 0); // Start with depth 0
  });
  
  console.log('All hierarchy paths:', hierarchyPaths);
  console.log('All hierarchy nodes:', hierarchyNodes.map(n => ({
    id: n.id,
    type: n.type,
    parentId: n.parentId, // FIXED: ReactFlow v12 uses parentId
    label: n.data.label
  })));
  
  // Assign hierarchy paths and parent relationships to graph nodes
  const processedNodes = graphData.nodes.map(node => {
    const assignment = nodeAssignments[node.id];
    console.log(`Processing graph node ${node.id}, assignment: ${assignment}`);
    
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
      
      console.log(`Assigned node ${node.id} to parent ${assignment}:`, {
        id: processedNode.id,
        parentId: processedNode.parentId,
        hierarchyPath: processedNode.data.hierarchyPath
      });
      
      return processedNode;
    }
    
    console.log(`Node ${node.id} has no hierarchy assignment`);
    return node;
  });
  
  const result = {
    ...graphData,
    nodes: [...hierarchyNodes, ...processedNodes], // Hierarchy nodes first, then graph nodes
  };
  
  console.log('Final processed nodes:', result.nodes.map(n => ({
    id: n.id,
    type: n.type,
    parentId: n.parentId, // FIXED: ReactFlow v12 uses parentId
    label: n.data?.label,
    hierarchyPath: n.data?.hierarchyPath
  })));
  
  console.log('=== HIERARCHY PROCESSING DEBUG END ===');
  return result;
}

/**
 * Process graph data into styled nodes and edges
 */
export async function processGraphData(graphData, colorPalette, currentLayout, applyLayout) {
  console.log('=== PROCESS GRAPH DATA DEBUG START ===');
  console.log('Input graphData:', {
    nodeCount: graphData?.nodes?.length,
    edgeCount: graphData?.edges?.length,
    hasHierarchy: !!graphData?.hierarchy,
    hasNodeAssignments: !!graphData?.nodeAssignments
  });

  if (!graphData?.nodes?.length) {
    console.log('No nodes found, returning empty result');
    console.log('=== PROCESS GRAPH DATA DEBUG END ===');
    return { nodes: [], edges: [] };
  }

  // Process hierarchy data first
  const processedGraphData = processHierarchy(graphData);
  
  console.log('After hierarchy processing:', {
    nodeCount: processedGraphData.nodes.length,
    hierarchyNodes: processedGraphData.nodes.filter(n => n.type === 'group').map(n => n.id),
    regularNodes: processedGraphData.nodes.filter(n => n.type !== 'group').map(n => n.id)
  });
  
  const processedNodes = processedGraphData.nodes.map(node => 
    createStyledNode(node, colorPalette, processedGraphData.hierarchy)
  );
  const processedEdges = (processedGraphData.edges || []).map(edge => createStyledEdge(edge));

  console.log('After styling, before layout:', {
    nodeCount: processedNodes.length,
    edgeCount: processedEdges.length,
    nodes: processedNodes.map(n => ({
      id: n.id,
      type: n.type,
      parentId: n.parentId, // FIXED: ReactFlow v12 uses parentId
      position: n.position
    }))
  });

  // Apply layout
  const layoutResult = await applyLayout(processedNodes, processedEdges, currentLayout);
  
  console.log('Final result after layout:', {
    nodeCount: layoutResult.nodes.length,
    edgeCount: layoutResult.edges.length,
    nodes: layoutResult.nodes.map(n => ({
      id: n.id,
      type: n.type,
      parentId: n.parentId, // FIXED: ReactFlow v12 uses parentId
      position: n.position
    }))
  });

  console.log('=== PROCESS GRAPH DATA DEBUG END ===');
  
  return {
    nodes: layoutResult.nodes,
    edges: layoutResult.edges,
  };
}
