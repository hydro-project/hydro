/**
 * JSON Parser for Hydro Graph Data
 * 
 * Converts the old visualizer's JSON format into the new VisualizationState format.
 * Handles nodes, edges, hierarchies, and grouping assignments.
 */

import { createVisualizationState } from './VisState.js';
import { NODE_STYLES, EDGE_STYLES } from './constants.js';

/**
 * Parse Hydro graph JSON and populate a VisualizationState
 * 
 * @param {Object|string} jsonData - The JSON data (object or JSON string)
 * @param {string} [selectedGrouping] - Which hierarchy grouping to use (defaults to first available)
 * @returns {Object} - { state: VisualizationState, metadata: Object }
 */
export function parseHydroGraphJSON(jsonData, selectedGrouping = null) {
  // Parse JSON if it's a string
  const data = typeof jsonData === 'string' ? JSON.parse(jsonData) : jsonData;
  
  // Validate basic structure
  if (!isValidGraphData(data)) {
    throw new Error('Invalid graph data: missing nodes or edges');
  }
  
  const state = createVisualizationState();
  
  // Parse metadata
  const metadata = extractMetadata(data);
  
  // Determine which grouping to use
  const grouping = selectGrouping(data, selectedGrouping);
  
  // Parse nodes first (base graph nodes)
  parseNodes(data.nodes, state);
  
  // Parse edges
  parseEdges(data.edges, state);
  
  // Parse hierarchy and create containers
  if (grouping) {
    parseHierarchy(data, grouping, state);
  }
  
  return {
    state,
    metadata: {
      ...metadata,
      selectedGrouping: grouping?.id || null,
      availableGroupings: metadata.hierarchyChoices || []
    }
  };
}

/**
 * Validate that the JSON data has the required structure
 */
function isValidGraphData(data) {
  return data && 
         Array.isArray(data.nodes) && 
         data.nodes.length > 0 &&
         Array.isArray(data.edges);
}

/**
 * Extract metadata from the JSON
 */
function extractMetadata(data) {
  return {
    hierarchyChoices: data.hierarchyChoices || [],
    nodeAssignments: data.nodeAssignments || {},
    originalNodeCount: data.nodes?.length || 0,
    originalEdgeCount: data.edges?.length || 0
  };
}

/**
 * Select which hierarchy grouping to use
 */
function selectGrouping(data, selectedGrouping) {
  if (!data.hierarchyChoices || data.hierarchyChoices.length === 0) {
    return null;
  }
  
  // Use specific grouping if requested and available
  if (selectedGrouping) {
    const found = data.hierarchyChoices.find(choice => choice.id === selectedGrouping);
    if (found) return found;
  }
  
  // Default to first available grouping
  return data.hierarchyChoices[0];
}

/**
 * Parse nodes from the JSON and add them to the state
 */
function parseNodes(nodes, state) {
  for (const node of nodes) {
    const visNode = convertNodeToVisFormat(node);
    state.setGraphNode(visNode.id, visNode);
  }
}

/**
 * Convert a JSON node to the vis format
 */
function convertNodeToVisFormat(jsonNode) {
  const node = {
    id: jsonNode.id,
    label: extractNodeLabel(jsonNode),
    style: convertNodeStyle(jsonNode),
    hidden: false,
    // Preserve original data for debugging/inspection
    originalData: jsonNode.data || {},
    // Extract common properties
    type: jsonNode.type || 'default',
    position: jsonNode.position || { x: 0, y: 0 }
  };
  
  // Add any additional properties from the original node
  if (jsonNode.width) node.width = jsonNode.width;
  if (jsonNode.height) node.height = jsonNode.height;
  if (jsonNode.className) node.className = jsonNode.className;
  
  return node;
}

/**
 * Extract a meaningful label from the node data
 */
function extractNodeLabel(jsonNode) {
  // Try various sources for the label
  if (jsonNode.data?.label) return jsonNode.data.label;
  if (jsonNode.label) return jsonNode.label;
  if (jsonNode.data?.name) return jsonNode.data.name;
  if (jsonNode.data?.title) return jsonNode.data.title;
  
  // Try to extract from backtrace data (Hydro-specific)
  if (jsonNode.data?.backtrace && jsonNode.data.backtrace.length > 0) {
    const topFrame = jsonNode.data.backtrace[0];
    if (topFrame.fn_name) {
      // Extract just the function name part
      const parts = topFrame.fn_name.split('::');
      return parts[parts.length - 1] || topFrame.fn_name;
    }
  }
  
  // Fallback to node ID
  return `Node ${jsonNode.id}`;
}

/**
 * Convert node styling from JSON format to vis format
 */
function convertNodeStyle(jsonNode) {
  // Map common style indicators to our style constants
  if (jsonNode.className?.includes('error') || jsonNode.data?.error) {
    return NODE_STYLES.ERROR;
  }
  if (jsonNode.className?.includes('warning') || jsonNode.data?.warning) {
    return NODE_STYLES.WARNING;
  }
  if (jsonNode.className?.includes('selected') || jsonNode.selected) {
    return NODE_STYLES.SELECTED;
  }
  if (jsonNode.className?.includes('highlighted')) {
    return NODE_STYLES.HIGHLIGHTED;
  }
  
  return NODE_STYLES.DEFAULT;
}

/**
 * Parse edges from the JSON and add them to the state
 */
function parseEdges(edges, state) {
  for (const edge of edges) {
    const visEdge = convertEdgeToVisFormat(edge);
    state.setGraphEdge(visEdge.id, visEdge);
  }
}

/**
 * Convert a JSON edge to the vis format
 */
function convertEdgeToVisFormat(jsonEdge) {
  const edge = {
    id: jsonEdge.id,
    source: jsonEdge.source,
    target: jsonEdge.target,
    style: convertEdgeStyle(jsonEdge),
    hidden: false,
    // Preserve original data
    label: jsonEdge.label || null,
    animated: jsonEdge.animated || false,
    type: jsonEdge.type || 'default'
  };
  
  return edge;
}

/**
 * Convert edge styling from JSON format to vis format
 */
function convertEdgeStyle(jsonEdge) {
  // Check for animated edges
  if (jsonEdge.animated) {
    return EDGE_STYLES.HIGHLIGHTED;
  }
  
  // Check style properties
  if (jsonEdge.style) {
    const style = jsonEdge.style;
    
    // Check for thick edges
    if (style.strokeWidth && style.strokeWidth > 2) {
      return EDGE_STYLES.THICK;
    }
    
    // Check for dashed edges
    if (style.strokeDasharray) {
      return EDGE_STYLES.DASHED;
    }
    
    // Check for warning colors (red, orange)
    if (style.stroke && (style.stroke.includes('red') || style.stroke.includes('orange'))) {
      return EDGE_STYLES.WARNING;
    }
  }
  
  return EDGE_STYLES.DEFAULT;
}

/**
 * Parse hierarchy data and create containers
 */
function parseHierarchy(data, grouping, state) {
  if (!grouping || !data.nodeAssignments) {
    return;
  }
  
  const assignments = data.nodeAssignments[grouping.id];
  if (!assignments) {
    console.warn(`No node assignments found for grouping: ${grouping.id}`);
    return;
  }
  
  // Build containers from hierarchy
  const containers = buildContainersFromHierarchy(grouping.hierarchy);
  
  // Add containers to state
  for (const container of containers) {
    state.setContainer(container.id, container);
  }
  
  // Assign nodes to containers based on assignments
  assignNodesToContainers(assignments, containers, state);
}

/**
 * Recursively build containers from hierarchy structure
 */
function buildContainersFromHierarchy(hierarchy, parentPath = '') {
  const containers = [];
  
  for (const item of hierarchy) {
    const containerPath = parentPath ? `${parentPath}.${item.id}` : item.id;
    
    const container = {
      id: item.id,
      label: item.name,
      expanded: true,
      collapsed: false,
      hidden: false,
      children: [],
      hierarchyPath: containerPath,
      expandedDimensions: { width: 200, height: 150 }
    };
    
    containers.push(container);
    
    // Recursively process children
    if (item.children && item.children.length > 0) {
      const childContainers = buildContainersFromHierarchy(item.children, containerPath);
      containers.push(...childContainers);
      
      // Set parent-child relationships
      container.children = item.children.map(child => child.id);
    }
  }
  
  return containers;
}

/**
 * Assign nodes to containers based on the nodeAssignments
 */
function assignNodesToContainers(assignments, containers, state) {
  // Create a map of container ID to container for quick lookup
  const containerMap = new Map();
  for (const container of containers) {
    containerMap.set(container.id, container);
  }
  
  // Track which nodes are assigned to which containers
  const nodeToContainer = new Map();
  
  // Process assignments
  for (const [nodeId, containerId] of Object.entries(assignments)) {
    if (state.getGraphNode(nodeId)) {
      nodeToContainer.set(nodeId, containerId);
    }
  }
  
  // Find leaf containers (containers with no child containers)
  const leafContainers = containers.filter(container => 
    !container.children.some(childId => containerMap.has(childId))
  );
  
  // Assign nodes to leaf containers
  for (const [nodeId, containerId] of nodeToContainer) {
    // Find the leaf container that this node should belong to
    let targetContainer = findLeafContainer(containerId, containerMap, containers);
    
    if (targetContainer) {
      targetContainer.children.push(nodeId);
      state.addContainerChild(targetContainer.id, nodeId);
    }
  }
  
  // Update all containers in state with their final children
  for (const container of containers) {
    const existingContainer = state.getContainer(container.id);
    if (existingContainer) {
      // Update the children set
      existingContainer.children = new Set(container.children);
      state.containerChildren.set(container.id, existingContainer.children);
    }
  }
}

/**
 * Find the leaf container in a hierarchy path
 */
function findLeafContainer(containerId, containerMap, allContainers) {
  const container = containerMap.get(containerId);
  if (!container) return null;
  
  // If this container has no child containers, it's a leaf
  const hasChildContainers = container.children.some(childId => containerMap.has(childId));
  if (!hasChildContainers) {
    return container;
  }
  
  // Otherwise, find a leaf among its children
  for (const childId of container.children) {
    const childContainer = containerMap.get(childId);
    if (childContainer) {
      const leaf = findLeafContainer(childId, containerMap, allContainers);
      if (leaf) return leaf;
    }
  }
  
  return container; // Fallback
}

/**
 * Create a new parser instance with custom configuration
 */
export function createHydroGraphParser(config = {}) {
  const {
    defaultNodeStyle = NODE_STYLES.DEFAULT,
    defaultEdgeStyle = EDGE_STYLES.DEFAULT,
    extractNodeLabel: customLabelExtractor = null,
    extractNodeStyle: customStyleExtractor = null
  } = config;
  
  return {
    parse: (jsonData, selectedGrouping) => parseHydroGraphJSON(jsonData, selectedGrouping),
    parseNodes: (nodes, state) => parseNodes(nodes, state),
    parseEdges: (edges, state) => parseEdges(edges, state),
    
    // Allow custom extractors
    setLabelExtractor: (extractor) => { customLabelExtractor = extractor; },
    setStyleExtractor: (extractor) => { customStyleExtractor = extractor; }
  };
}

/**
 * Utility function to get all available groupings from JSON data
 */
export function getAvailableGroupings(jsonData) {
  const data = typeof jsonData === 'string' ? JSON.parse(jsonData) : jsonData;
  return data.hierarchyChoices || [];
}

/**
 * Utility function to validate JSON structure
 */
export function validateHydroGraphJSON(jsonData) {
  try {
    const data = typeof jsonData === 'string' ? JSON.parse(jsonData) : jsonData;
    
    const errors = [];
    const warnings = [];
    
    if (!data.nodes || !Array.isArray(data.nodes)) {
      errors.push('Missing or invalid nodes array');
    }
    
    if (!data.edges || !Array.isArray(data.edges)) {
      errors.push('Missing or invalid edges array');
    }
    
    if (data.nodes && data.nodes.length === 0) {
      warnings.push('No nodes found in data');
    }
    
    if (data.hierarchyChoices && !Array.isArray(data.hierarchyChoices)) {
      warnings.push('Invalid hierarchyChoices format');
    }
    
    if (data.hierarchyChoices && data.hierarchyChoices.length > 0 && !data.nodeAssignments) {
      warnings.push('Hierarchy choices found but no node assignments');
    }
    
    return {
      isValid: errors.length === 0,
      errors,
      warnings,
      nodeCount: data.nodes?.length || 0,
      edgeCount: data.edges?.length || 0,
      hierarchyCount: data.hierarchyChoices?.length || 0
    };
  } catch (error) {
    return {
      isValid: false,
      errors: [`JSON parsing error: ${error.message}`],
      warnings: [],
      nodeCount: 0,
      edgeCount: 0,
      hierarchyCount: 0
    };
  }
}
