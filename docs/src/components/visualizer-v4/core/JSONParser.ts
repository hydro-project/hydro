/**
 * JSON Parser for Graph Data
 * 
 * Framework-independent JSON parser that converts graph data into a VisualizationState.
 * Handles nodes, edges, hierarchies, and grouping assignments.
 */

import { createVisualizationState, VisualizationState } from './VisualizationState';
import { NODE_STYLES, EDGE_STYLES, NodeStyle, EdgeStyle } from '../shared/config';

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
    styleConfig?: Record<string, any>;
    edgeStyleConfig?: {
      propertyMappings: Record<string, any>;
      defaultStyle: any;
      combinationRules: any;
    };
    nodeTypeConfig?: {
      defaultType?: string;
      types?: Array<{
        id: string;
        label: string;
        colorIndex: number;
      }>;
    };
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
  semanticTags?: string[];
  [key: string]: any;
}

interface RawEdge {
  id: string;
  source: string;
  target: string;
  semanticTags?: string[];
  edgeProperties?: string[];
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
  children?: RawHierarchyItem[];  // Direct children, no wrapper
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
  styleConfig?: {
    edgeStyles?: Record<string, any>;
    nodeStyles?: Record<string, any>;
  };
  edgeStyleConfig?: {
    propertyMappings: Record<string, any>;
    defaultStyle: any;
    combinationRules: any;
  };
  nodeTypeConfig?: {
    defaultType?: string;
    types?: Array<{
      id: string;
      label: string;
      colorIndex: number;
    }>;
  };
  metadata?: Record<string, any>;
}

/**
 * Parse graph JSON and populate a VisualizationState
 * 
 * @param jsonData - Raw graph data or JSON string
 * @param grouping - Optional hierarchy grouping to apply
 * @returns Object containing the populated state and parsing metadata
 * 
 * @example
 * ```javascript
 * const { state, metadata } = parseGraphJSON(graphData, 'myGrouping');
 * // // console.log((('Parsed', metadata.nodeCount, 'nodes')));
 * ```
 */
export function parseGraphJSON(
  jsonData: RawGraphData | string,
  selectedGrouping?: string
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
      availableGroupings: getAvailableGroupings(data),
      styleConfig: data.styleConfig || {}, // Include style configuration
      edgeStyleConfig: data.edgeStyleConfig, // Include edge style configuration
      nodeTypeConfig: metadata.nodeTypeConfig
    }
  };
}

/**
 * Create a reusable parser instance for processing multiple graph datasets.
 * 
 * @param options - Parser configuration options
 * @returns Configured parser instance with parse method
 * 
 * @example
 * ```javascript
 * const parser = createGraphParser({
 *   enableValidation: true,
 *   defaultStyle: 'highlighted'
 * });
 * 
 * const result1 = parser.parse(data1);
 * const result2 = parser.parse(data2);
 * ```
 */
export function createGraphParser(options: ParserOptions = {}): { parse: (data: RawGraphData | string, grouping?: string) => ParseResult } {
  const {
    validateData = true,
    strictMode = false,
    defaultNodeStyle = NODE_STYLES.DEFAULT,
    defaultEdgeStyle = EDGE_STYLES.DEFAULT
  } = options;

  return {
    parse: (data: RawGraphData | string, grouping?: string): ParseResult => {
      if (validateData) {
        const validation = validateGraphJSON(data);
        if (!validation.isValid) {
          if (strictMode) {
            throw new Error(`Validation failed: ${validation.errors.join(', ')}`);
          } else {
            console.warn('Parser validation warnings:', validation.warnings);
          }
        }
      }
      
      return parseGraphJSON(data, grouping);
    }
  };
}/**
 * Extract available hierarchical groupings from graph JSON data.
 * 
 * @param jsonData - Raw graph data or JSON string
 * @returns Array of available grouping options
 * 
 * @example
 * ```javascript
 * const groupings = getAvailableGroupings(graphData);
 * // // console.log((('Available groupings:', groupings.map(g => g.name))));
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
export function validateGraphJSON(jsonData: RawGraphData | string): ValidationResult {
  try {
    const data: RawGraphData = typeof jsonData === 'string' ? JSON.parse(jsonData) : jsonData;
    
    const errors: string[] = [];
    const warnings: string[] = [];
    
    // Check basic structure
    if (!data || typeof data !== 'object') {
      errors.push('Data must be an object');
      return { isValid: false, errors, warnings, nodeCount: 0, edgeCount: 0, hierarchyCount: 0 };
    }
    
    // 🔥 VALIDATE: JSON must NOT contain mutable state fields
    // These fields represent UI state and should be managed by VisualizationState only
    const forbiddenFields = ['collapsed', 'hidden', 'style'];
    const allowedSemanticFields = ['semanticTags']; // Allow semantic tags for styling configuration
    
    // Check nodes for forbidden fields
    if (Array.isArray(data.nodes)) {
      for (let i = 0; i < data.nodes.length; i++) {
        const node = data.nodes[i];
        if (node) {
          for (const forbiddenField of forbiddenFields) {
            if (forbiddenField in node) {
              errors.push(`Node '${node.id || `at index ${i}`}' contains forbidden mutable state field '${forbiddenField}'. JSON should only contain immutable graph structure.`);
            }
          }
          // Validate semanticTags if present
          if ('semanticTags' in node && !Array.isArray(node.semanticTags)) {
            errors.push(`Node '${node.id || `at index ${i}`}' has invalid semanticTags - must be an array of strings.`);
          }
        }
      }
    }
    
    // Check edges for forbidden fields
    if (Array.isArray(data.edges)) {
      for (let i = 0; i < data.edges.length; i++) {
        const edge = data.edges[i];
        if (edge) {
          for (const forbiddenField of forbiddenFields) {
            if (forbiddenField in edge) {
              errors.push(`Edge '${edge.id || `at index ${i}`}' contains forbidden mutable state field '${forbiddenField}'. JSON should only contain immutable graph structure.`);
            }
          }
          // Validate semanticTags if present
          if ('semanticTags' in edge && !Array.isArray(edge.semanticTags)) {
            errors.push(`Edge '${edge.id || `at index ${i}`}' has invalid semanticTags - must be an array of strings.`);
          }
          // Validate edgeProperties if present
          if ('edgeProperties' in edge && !Array.isArray(edge.edgeProperties)) {
            errors.push(`Edge '${edge.id || `at index ${i}`}' has invalid edgeProperties - must be an array of strings.`);
          }
        }
      }
    }
    
    // Check hierarchies for forbidden fields
    if (data.hierarchies) {
      for (const hierarchy of data.hierarchies) {
        if (hierarchy.groups) {
          // Old format: check if groups contain forbidden fields
          for (const [containerId, nodeIds] of Object.entries(hierarchy.groups)) {
            // Groups in old format are just arrays, but check if accidentally object with state
            if (typeof nodeIds === 'object' && !Array.isArray(nodeIds)) {
              for (const forbiddenField of forbiddenFields) {
                if (forbiddenField in nodeIds) {
                  errors.push(`Container '${containerId}' in hierarchy '${hierarchy.id}' contains forbidden mutable state field '${forbiddenField}'. JSON should only contain immutable graph structure.`);
                }
              }
            }
          }
        }
      }
    }
    
    // Check hierarchyChoices for forbidden fields
    if (data.hierarchyChoices) {
      for (const choice of data.hierarchyChoices) {
        if (choice.children) {
          // Recursively check hierarchy items
          function validateHierarchyItems(items: any[], hierarchyId: string): void {
            for (const item of items) {
              for (const forbiddenField of forbiddenFields) {
                if (forbiddenField in item) {
                  errors.push(`Container '${item.id}' in hierarchy '${hierarchyId}' contains forbidden mutable state field '${forbiddenField}'. JSON should only contain immutable graph structure.`);
                }
              }
              if (item.children && Array.isArray(item.children)) {
                validateHierarchyItems(item.children, hierarchyId);
              }
            }
          }
          validateHierarchyItems(choice.children, choice.id);
        }
      }
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
        if (!node.shortLabel || typeof node.shortLabel !== 'string') {
          warnings.push(`Node '${node.id}' missing or invalid shortLabel`);
        }
        if (!node.fullLabel || typeof node.fullLabel !== 'string') {
          warnings.push(`Node '${node.id}' missing or invalid fullLabel`);
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

/**
 * Type guard to validate that input data has the required structure for graph parsing
 * @param data - Input data to validate
 * @returns true if data has valid graph structure (nodes and edges arrays)
 */
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
    nodeTypeConfig: data.nodeTypeConfig || null,
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
      // Extract immutable properties and filter out UI state fields
      const { id, shortLabel, fullLabel, nodeType, semanticTags, position, type, expanded, collapsed, hidden, style, ...safeProps } = rawNode;
      
      state.addGraphNode(id, {
        label: shortLabel || id, // Initial display label (starts with short, can be toggled to full)
        shortLabel: shortLabel || id,
        fullLabel: fullLabel || shortLabel || id,
        style: NODE_STYLES.DEFAULT, // Default style - will be applied by bridge based on semanticTags
        // ✅ All nodes start visible - VisualizationState manages visibility
        hidden: false,
        nodeType: nodeType || 'default',
        semanticTags: semanticTags || [],
        ...safeProps // Only include non-UI-state properties
      });
    } catch (error) {
      console.warn(`Failed to parse node '${rawNode.id}':`, error);
    }
  }
}

function parseEdges(edges: RawEdge[], state: VisualizationState): void {
  for (const rawEdge of edges) {
    try {
      // Extract immutable properties and filter out UI state fields
      const { id, source, target, semanticTags, edgeProperties, style, hidden, animated, ...safeProps } = rawEdge;
      
      state.addGraphEdge(id, {
        source,
        target,
        style: EDGE_STYLES.DEFAULT, // Default style - will be applied by bridge based on properties
        // ✅ All edges start visible - VisualizationState manages visibility
        hidden: false,
        semanticTags: semanticTags || [],
        edgeProperties: edgeProperties || semanticTags || [], // Use edgeProperties or fall back to semanticTags
        ...safeProps // Only include non-UI-state properties
      });
    } catch (error) {
      console.warn(`Failed to parse edge '${rawEdge.id}':`, error);
    }
  }
}

function parseHierarchy(data: RawGraphData, groupingId: string, state: VisualizationState): number {
  let containerCount = 0;
  
  // Find the requested hierarchy - check both new and old formats
  let hierarchyChoice: any = null;
  let groupsData: any = null;
  
  // Check new format (hierarchyChoices)
  if (data.hierarchyChoices) {
    hierarchyChoice = data.hierarchyChoices.find(choice => choice.id === groupingId);
  }
  
  // Check old format (hierarchies) if not found in new format
  if (!hierarchyChoice && data.hierarchies) {
    const oldHierarchy = data.hierarchies.find(h => h.id === groupingId);
    if (oldHierarchy) {
      // Convert old format to expected structure
      hierarchyChoice = oldHierarchy;
      groupsData = oldHierarchy.groups; // Old format has groups object
    }
  }
  
  if (!hierarchyChoice) {
    console.warn(`Hierarchy choice '${groupingId}' not found`);
    return 0;
  }
  
  // Handle both old and new formats
  if (groupsData) {
    // Old format: groups is an object { containerID: [nodeID1, nodeID2, ...] }
    for (const [containerId, nodeIds] of Object.entries(groupsData)) {
      if (Array.isArray(nodeIds)) {
        state.addContainer(containerId, {
          label: containerId,
          children: nodeIds,
          collapsed: false
        });
        containerCount++;
        
        // Add nodes to container children
        for (const nodeId of nodeIds) {
          if (state.getGraphNode(nodeId)) {
            state.addContainerChild(containerId, nodeId);
          }
        }
      }
    }
  } else if (hierarchyChoice.children) {
    // New format: hierarchical structure
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
        
        state.addContainer(item.id, {
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
              state.addContainer(parentId, {
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
    createContainersFromHierarchy(hierarchyChoice.children);
    
    // Assign nodes to containers based on nodeAssignments
    const assignments = data.nodeAssignments?.[groupingId];
    if (assignments) {
      for (const [nodeId, containerId] of Object.entries(assignments)) {
        const container = state.getContainer(containerId);
        if (container && state.getGraphNode(nodeId)) {
          // Add node to container's children
          const currentChildren = state.getContainerChildren(containerId);
          if (!currentChildren.has(nodeId)) {
            state.addContainer(containerId, {
              ...container,
              children: [...currentChildren, nodeId]
            });
          }
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

/**
 * Create a RenderConfig from ParseResult metadata
 * This helper makes it easy to pass edgeStyleConfig and other metadata to FlowGraph
 */
export function createRenderConfig(parseResult: ParseResult, baseConfig: any = {}): any {
  return {
    ...baseConfig,
    edgeStyleConfig: parseResult.metadata.edgeStyleConfig,
    // Include other metadata as needed
    styleConfig: parseResult.metadata.styleConfig,
    nodeTypeConfig: parseResult.metadata.nodeTypeConfig
  };
}
