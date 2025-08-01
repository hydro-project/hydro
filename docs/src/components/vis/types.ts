/**
 * @fileoverview Type definitions for the Vis component
 * 
 * Core TypeScript interfaces and types for the Hydro graph visualization system.
 * These types provide compile-time safety and better developer experience.
 */

// ============ Styling Constants ============

export const NODE_STYLES = {
  DEFAULT: 'default',
  HIGHLIGHTED: 'highlighted',
  SELECTED: 'selected',
  WARNING: 'warning',
  ERROR: 'error'
} as const;

export const EDGE_STYLES = {
  DEFAULT: 'default',
  HIGHLIGHTED: 'highlighted',
  DASHED: 'dashed',
  THICK: 'thick',
  WARNING: 'warning'
} as const;

export const CONTAINER_STYLES = {
  DEFAULT: 'default',
  HIGHLIGHTED: 'highlighted',
  SELECTED: 'selected',
  MINIMIZED: 'minimized'
} as const;

// ============ Type Definitions ============

export type NodeStyle = typeof NODE_STYLES[keyof typeof NODE_STYLES];
export type EdgeStyle = typeof EDGE_STYLES[keyof typeof EDGE_STYLES];
export type ContainerStyle = typeof CONTAINER_STYLES[keyof typeof CONTAINER_STYLES];

export interface Dimensions {
  width: number;
  height: number;
}

export interface GraphNode {
  id: string;
  label: string;
  style: NodeStyle;
  hidden: boolean;
  [key: string]: any; // Allow custom properties
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  style: EdgeStyle;
  hidden: boolean;
  [key: string]: any; // Allow custom properties
}

export interface Container {
  id: string;
  expandedDimensions: Dimensions;
  collapsed: boolean;
  hidden: boolean;
  children: Set<string>;
  [key: string]: any; // Allow custom properties
}

export interface HyperEdge {
  id: string;
  source: string;
  target: string;
  style: EdgeStyle;
  aggregatedEdges: GraphEdge[];
  [key: string]: any; // Allow custom properties
}

// ============ Input Types for Methods ============

export interface CreateNodeProps {
  label: string;
  style?: NodeStyle;
  hidden?: boolean;
  [key: string]: any;
}

export interface CreateEdgeProps {
  source: string;
  target: string;
  style?: EdgeStyle;
  hidden?: boolean;
  [key: string]: any;
}

export interface CreateContainerProps {
  expandedDimensions?: Dimensions;
  collapsed?: boolean;
  hidden?: boolean;
  children?: string[];
  [key: string]: any;
}

// ============ Parser Types ============

export interface ParseResult {
  state: VisualizationState;
  metadata: {
    selectedGrouping: string | null;
    nodeCount: number;
    edgeCount: number;
    containerCount: number;
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

export interface GroupingOption {
  id: string;
  name: string;
}

// ============ Class Interface ============

export interface VisualizationState {
  // Node methods
  setGraphNode(id: string, props: CreateNodeProps): GraphNode;
  getGraphNode(id: string): GraphNode | undefined;
  setNodeHidden(id: string, hidden: boolean): void;
  getNodeHidden(id: string): boolean | undefined;
  removeGraphNode(id: string): void;
  
  // Edge methods
  setGraphEdge(id: string, props: CreateEdgeProps): GraphEdge;
  getGraphEdge(id: string): GraphEdge | undefined;
  setEdgeHidden(id: string, hidden: boolean): void;
  getEdgeHidden(id: string): boolean | undefined;
  removeGraphEdge(id: string): void;
  
  // Container methods
  setContainer(id: string, props: CreateContainerProps): Container;
  getContainer(id: string): Container | undefined;
  setContainerCollapsed(id: string, collapsed: boolean): void;
  getContainerCollapsed(id: string): boolean | undefined;
  setContainerHidden(id: string, hidden: boolean): void;
  getContainerHidden(id: string): boolean | undefined;
  
  // Visibility methods
  getVisibleNodes(): GraphNode[];
  getVisibleEdges(): GraphEdge[];
  getVisibleContainers(): Container[];
  getHyperEdges(): HyperEdge[];
  
  // Container hierarchy methods
  addContainerChild(containerId: string, childId: string): void;
  removeContainerChild(containerId: string, childId: string): void;
  getContainerChildren(containerId: string): Set<string> | undefined;
  getNodeContainer(nodeId: string): string | undefined;
  
  // Container operations
  collapseContainer(containerId: string): void;
  expandContainer(containerId: string): void;
}
