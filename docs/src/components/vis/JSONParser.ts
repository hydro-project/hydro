/**
 * JSON Parser for Hydro Graph Data
 * 
 * Converts the old visualizer's JSON format into the new VisualizationState format.
 * Handles nodes, edges, hierarchies, and grouping assignments.
 */

import { createVisualizationState, VisualizationState } from './VisState.js';
import { NODE_STYLES, EDGE_STYLES, NodeStyle, EdgeStyle } from './constants.js';

// ============ Type Definitions ============

export interface GroupingOption {
  id: string;
  name: string;
}

export interface ParseResult {
  state: VisualizationState;
  metadata: {
    selectedGrouping: string | null;
    nodeCount: number;
    edgeCount: number;
    containerCount: number;
    availableGroupings: GroupingOption[];
  };
}

export interface ValidationResult {
  isValid: boolean;
  errors: string[];
  warnings: string[];
  nodeCount: number;
  edgeCount: number;
  hierarchyCount: number;
}

export interface ParserOptions {
  validateData?: boolean;
  strictMode?: boolean;
  defaultNodeStyle?: NodeStyle;
  defaultEdgeStyle?: EdgeStyle;
}

// Raw JSON data interfaces (from legacy format)
interface RawNode {
  id: string;
  label?: string;
  style?: string;
  hidden?: boolean;
  [key: string]: any;
}

interface RawEdge {
  id: string;
  source: string;
  target: string;
  style?: string;
  hidden?: boolean;
  [key: string]: any;
}

interface RawHierarchy {
  id: string;
  name: string;
  groups: Record<string, string[]>;
}

interface RawHierarchyChoice {
  id: string;
  name: string;
  hierarchy: RawHierarchyItem[];
}

interface RawHierarchyItem {
  id: string;
  name: string;
  children?: RawHierarchyItem[];
}

interface RawGraphData {
  nodes: RawNode[];
  edges: RawEdge[];
  hierarchies?: RawHierarchy[];
  hierarchyChoices?: RawHierarchyChoice[];
  nodeAssignments?: Record<string, Record<string, string>>;
  metadata?: Record<string, any>;
}

/**
 * Parse Hydro graph JSON and populate a VisualizationState
 * 
 * @param jsonData - The JSON data (object or JSON string)
 * @param selectedGrouping - Which hierarchy grouping to use (defaults to first available)
 * @returns Object containing the populated state and metadata
 * @throws {Error} When JSON data is invalid or malformed
 * @example
 * ```typescript
 * const { state, metadata } = parseHydroGraphJSON(hydroData, 'myGrouping');
 * console.log(`Parsed ${state.getVisibleNodes().length} nodes`);
 * console.log(`Used grouping: ${metadata.selectedGrouping}`);
 * ```
 */
export function parseHydroGraphJSON(
  jsonData: RawGraphData | string, 
  selectedGrouping: string | null = null
): ParseResult {
  // Parse JSON if it's a string
  const data: RawGraphData = typeof jsonData === 'string' ? JSON.parse(jsonData) : jsonData;
  
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
  let containerCount = 0;
  if (grouping) {
    containerCount = parseHierarchy(data, grouping, state);
  }
  
  return {
    state,
    metadata: {
      nodeCount: metadata.nodeCount,
      edgeCount: metadata.edgeCount,
      selectedGrouping: grouping,
      containerCount,
      availableGroupings: getAvailableGroupings(data)
    }
  };
}

/**
 * Create a reusable parser instance for processing multiple Hydro graph datasets.
 * Useful when parsing multiple graphs with similar structure/settings.
 * 
 * @param options - Parser configuration options
 * @returns Parser function that accepts JSON data
 * @example
 * ```typescript
 * const parser = createHydroGraphParser({
 *   validateData: true,
 *   defaultNodeStyle: NODE_STYLES.HIGHLIGHTED
 * });
 * 
 * const result1 = parser(graphData1);
 * const result2 = parser(graphData2);
 * ```
 */
export function createHydroGraphParser(options: ParserOptions = {}): { parse: (data: RawGraphData | string, grouping?: string) => ParseResult } {
  const {
    validateData = true,
    strictMode = false,
    defaultNodeStyle = NODE_STYLES.DEFAULT,
    defaultEdgeStyle = EDGE_STYLES.DEFAULT
  } = options;

  return {
    parse: (data: RawGraphData | string, grouping?: string): ParseResult => {
      if (validateData) {
        const validation = validateHydroGraphJSON(data);
        if (!validation.isValid) {
          if (strictMode) {
            throw new Error(`Validation failed: ${validation.errors.join(', ')}`);
          } else {
            console.warn('Parser validation warnings:', validation.warnings);
          }
        }
      }
      
      return parseHydroGraphJSON(data, grouping);
    }
  };
}/**
 * Extract available hierarchical groupings from Hydro graph JSON data.
 * Useful for presenting grouping options to users before parsing.
 * 
 * @param jsonData - The JSON data (object or JSON string)
 * @returns Array of available grouping objects
 * @example
 * ```typescript
 * const groupings = getAvailableGroupings(hydroData);
 * groupings.forEach(g => console.log(`${g.name} (${g.id})`));
 * ```
 */
export function getAvailableGroupings(jsonData: RawGraphData | string): GroupingOption[] {
  const data: RawGraphData = typeof jsonData === 'string' ? JSON.parse(jsonData) : jsonData;
  
  // Check for new format (hierarchyChoices)
  if (data.hierarchyChoices && Array.isArray(data.hierarchyChoices)) {
    return data.hierarchyChoices.map(choice => ({
      id: choice.id,
      name: choice.name || choice.id
    }));
  }
  
  // Check for old format (hierarchies)
  if (data.hierarchies && Array.isArray(data.hierarchies)) {
    return data.hierarchies.map(hierarchy => ({
      id: hierarchy.id,
      name: hierarchy.name || hierarchy.id
    }));
  }
  
  return [];
}

/**
 * Validate Hydro graph JSON data structure and content.
 * Provides detailed validation results including errors and warnings.
 * 
 * @param jsonData - The JSON data (object or JSON string)
 * @returns Validation result object
 * @example
 * ```typescript
 * const validation = validateHydroGraphJSON(suspiciousData);
 * if (!validation.isValid) {
 *   console.error('Validation failed:', validation.errors);
 *   return;
 * }
 * if (validation.warnings.length > 0) {
 *   console.warn('Warnings found:', validation.warnings);
 * }
 * ```
 */
export function validateHydroGraphJSON(jsonData: RawGraphData | string): ValidationResult {
  try {
    const data: RawGraphData = typeof jsonData === 'string' ? JSON.parse(jsonData) : jsonData;
    
    const errors: string[] = [];
    const warnings: string[] = [];
    
    // Check basic structure
    if (!data || typeof data !== 'object') {
      errors.push('Data must be an object');
      return { isValid: false, errors, warnings, nodeCount: 0, edgeCount: 0, hierarchyCount: 0 };
    }
    
    // Validate nodes
    if (!Array.isArray(data.nodes)) {
      errors.push('Missing or invalid nodes array');
    } else {
      for (let i = 0; i < data.nodes.length; i++) {
        const node = data.nodes[i];
        if (!node) {
          errors.push(`Node at index ${i} is null or undefined`);
          continue;
        }
        if (!node.id || typeof node.id !== 'string') {
          errors.push(`Node at index ${i} missing or invalid id`);
          continue;
        }
        if (!node.label || typeof node.label !== 'string') {
          warnings.push(`Node '${node.id}' missing or invalid label`);
        }
      }
    }
    
    // Validate edges
    if (!Array.isArray(data.edges)) {
      errors.push('Missing or invalid edges array');
    } else {
      const nodeIds = new Set(data.nodes?.map(n => n?.id).filter(Boolean) || []);
      for (let i = 0; i < data.edges.length; i++) {
        const edge = data.edges[i];
        if (!edge) {
          errors.push(`Edge at index ${i} is null or undefined`);
          continue;
        }
        if (!edge.id || typeof edge.id !== 'string') {
          errors.push(`Edge at index ${i} missing or invalid id`);
          continue;
        }
        if (!edge.source || typeof edge.source !== 'string') {
          errors.push(`Edge '${edge.id}' missing or invalid source`);
        } else if (!nodeIds.has(edge.source)) {
          warnings.push(`Edge '${edge.id}' references unknown source node '${edge.source}'`);
        }
        if (!edge.target || typeof edge.target !== 'string') {
          errors.push(`Edge '${edge.id}' missing or invalid target`);
        } else if (!nodeIds.has(edge.target)) {
          warnings.push(`Edge '${edge.id}' references unknown target node '${edge.target}'`);
        }
      }
    }
    
    // Validate hierarchies (optional)
    let hierarchyCount = 0;
    if (data.hierarchies) {
      if (!Array.isArray(data.hierarchies)) {
        warnings.push('Hierarchies should be an array');
      } else {
        hierarchyCount = data.hierarchies.length;
        for (let i = 0; i < data.hierarchies.length; i++) {
          const hierarchy = data.hierarchies[i];
          if (!hierarchy) {
            warnings.push(`Hierarchy at index ${i} is null or undefined`);
            continue;
          }
          if (!hierarchy.id || typeof hierarchy.id !== 'string') {
            warnings.push(`Hierarchy at index ${i} missing or invalid id`);
            continue;
          }
          if (!hierarchy.groups || typeof hierarchy.groups !== 'object') {
            warnings.push(`Hierarchy '${hierarchy.id}' missing or invalid groups`);
          }
        }
      }
    }
    
    return {
      isValid: errors.length === 0,
      errors,
      warnings,
      nodeCount: data.nodes?.length || 0,
      edgeCount: data.edges?.length || 0,
      hierarchyCount
    };
    
  } catch (error) {
    return {
      isValid: false,
      errors: [`JSON parsing error: ${error instanceof Error ? error.message : 'Unknown error'}`],
      warnings: [],
      nodeCount: 0,
      edgeCount: 0,
      hierarchyCount: 0
    };
  }
}

// ============ Private Helper Functions ============

function isValidGraphData(data: any): data is RawGraphData {
  return data && 
         typeof data === 'object' && 
         Array.isArray(data.nodes) && 
         Array.isArray(data.edges);
}

function extractMetadata(data: RawGraphData): Record<string, any> {
  return {
    nodeCount: data.nodes.length,
    edgeCount: data.edges.length,
    hasHierarchies: !!(data.hierarchies && data.hierarchies.length > 0),
    ...data.metadata
  };
}

function selectGrouping(data: RawGraphData, selectedGrouping: string | null): string | null {
  // Check for new format (hierarchyChoices)
  if (data.hierarchyChoices && data.hierarchyChoices.length > 0) {
    if (selectedGrouping) {
      const found = data.hierarchyChoices.find(h => h.id === selectedGrouping);
      if (found) return selectedGrouping;
      console.warn(`Grouping '${selectedGrouping}' not found, using first available`);
    }
    return data.hierarchyChoices[0].id;
  }
  
  // Check for old format (hierarchies) 
  if (data.hierarchies && data.hierarchies.length > 0) {
    if (selectedGrouping) {
      const found = data.hierarchies.find(h => h.id === selectedGrouping);
      if (found) return selectedGrouping;
      console.warn(`Grouping '${selectedGrouping}' not found, using first available`);
    }
    return data.hierarchies[0].id;
  }
  
  return null;
}

function parseNodes(nodes: RawNode[], state: VisualizationState): void {
  for (const rawNode of nodes) {
    try {
      // Map raw style to our constants
      const style = mapStyleConstant(rawNode.style, NODE_STYLES, NODE_STYLES.DEFAULT) as NodeStyle;
      
      const { id, label, hidden, style: rawStyle, ...otherProps } = rawNode;
      
      // Extract label with priority: explicit label > backtrace fn_name > data.name > data.label > id
      let nodeLabel = label;
      if (!nodeLabel && rawNode.data) {
        // Try to extract from backtrace
        if (rawNode.data.backtrace && Array.isArray(rawNode.data.backtrace) && rawNode.data.backtrace.length > 0) {
          const firstBacktrace = rawNode.data.backtrace[0];
          if (firstBacktrace.fn_name) {
            // Extract the last part after :: (e.g., "broadcast_bincode" from "Stream<T,L,B,O,R>::broadcast_bincode")
            const parts = firstBacktrace.fn_name.split('::');
            nodeLabel = parts[parts.length - 1];
          }
        }
        // Fallback to other data properties
        if (!nodeLabel) {
          nodeLabel = rawNode.data.label || rawNode.data.name;
        }
      }
      
      state.setGraphNode(id, {
        label: nodeLabel || id,
        style,
        hidden: !!hidden,
        ...otherProps
      });
    } catch (error) {
      console.warn(`Failed to parse node '${rawNode.id}':`, error);
    }
  }
}

function parseEdges(edges: RawEdge[], state: VisualizationState): void {
  for (const rawEdge of edges) {
    try {
      // Determine edge style from raw style data
      let style: EdgeStyle;
      if (typeof rawEdge.style === 'string') {
        style = mapStyleConstant(rawEdge.style, EDGE_STYLES, EDGE_STYLES.DEFAULT) as EdgeStyle;
      } else if (rawEdge.style && typeof rawEdge.style === 'object') {
        style = detectEdgeStyleFromObject(rawEdge.style) as EdgeStyle;
      } else {
        style = EDGE_STYLES.DEFAULT as EdgeStyle;
      }
      
      // Handle animated edges - they should be highlighted
      if (rawEdge.animated) {
        style = EDGE_STYLES.HIGHLIGHTED as EdgeStyle;
      }
      
      const { id, source, target, hidden, style: rawStyle, ...otherProps } = rawEdge;
      
      state.setGraphEdge(id, {
        source,
        target,
        style,
        hidden: !!hidden,
        ...otherProps
      });
    } catch (error) {
      console.warn(`Failed to parse edge '${rawEdge.id}':`, error);
    }
  }
}

/**
 * Detect edge style from a style object
 */
function detectEdgeStyleFromObject(styleObj: any): string {
  // Check for thick edges (strokeWidth >= 3)
  if (styleObj.strokeWidth && styleObj.strokeWidth >= 3) {
    return EDGE_STYLES.THICK;
  }
  
  // Check for warning edges (red stroke)
  if (styleObj.stroke && (styleObj.stroke === 'red' || styleObj.stroke === '#ff0000' || styleObj.stroke === '#f00')) {
    return EDGE_STYLES.WARNING;
  }
  
  // Check for dashed edges
  if (styleObj.strokeDasharray) {
    return EDGE_STYLES.DASHED;
  }
  
  // Default style
  return EDGE_STYLES.DEFAULT;
}

function parseHierarchy(data: RawGraphData, groupingId: string, state: VisualizationState): number {
  let containerCount = 0;
  
  // Find the requested hierarchy choice
  const hierarchyChoice = data.hierarchyChoices?.find(choice => choice.id === groupingId);
  if (!hierarchyChoice) {
    console.warn(`Hierarchy choice '${groupingId}' not found`);
    return 0;
  }
  
  // Create containers from the hierarchy structure
  function createContainersFromHierarchy(hierarchyItems: any[], parentId?: string): void {
    for (const item of hierarchyItems) {
      // Create the container
      const children: string[] = [];
      
      // Add child containers if they exist
      if (item.children && Array.isArray(item.children)) {
        for (const childItem of item.children) {
          children.push(childItem.id);
        }
      }
      
      state.setContainer(item.id, {
        label: item.name || item.id,
        children,
        collapsed: false
      });
      containerCount++;
      
      // If this container has a parent, add it to the parent's children
      if (parentId) {
        const parent = state.getContainer(parentId);
        if (parent) {
          const parentChildren = state.getContainerChildren(parentId);
          if (!parentChildren.has(item.id)) {
            state.setContainer(parentId, {
              ...parent,
              children: [...parentChildren, item.id]
            });
          }
        }
      }
      
      // Recursively create child containers
      if (item.children && Array.isArray(item.children)) {
        createContainersFromHierarchy(item.children, item.id);
      }
    }
  }
  
  // Create all containers first
  createContainersFromHierarchy(hierarchyChoice.hierarchy);
  
  // Assign nodes to containers based on nodeAssignments
  const assignments = data.nodeAssignments?.[groupingId];
  if (assignments) {
    for (const [nodeId, containerId] of Object.entries(assignments)) {
      const container = state.getContainer(containerId);
      if (container && state.getGraphNode(nodeId)) {
        // Add node to container's children
        const currentChildren = state.getContainerChildren(containerId);
        if (!currentChildren.has(nodeId)) {
          state.setContainer(containerId, {
            ...container,
            children: [...currentChildren, nodeId]
          });
        }
      }
    }
  }
  
  return containerCount;
}

function mapStyleConstant(
  rawStyle: string | undefined, 
  styleConstants: Record<string, string>, 
  defaultStyle: string
): string {
  if (!rawStyle || typeof rawStyle !== 'string') {
    return defaultStyle;
  }
  
  // Try exact match first
  const upperStyle = rawStyle.toUpperCase();
  for (const [key, value] of Object.entries(styleConstants)) {
    if (key === upperStyle) {
      return value;
    }
  }
  
  // Try value match
  for (const value of Object.values(styleConstants)) {
    if (value === rawStyle.toLowerCase()) {
      return value;
    }
  }
  
  console.warn(`Unknown style '${rawStyle}', using default`);
  return defaultStyle;
}
