// External-facing container type (no expandedDimensions)
export interface ExternalContainer {
  id: string;
  collapsed: boolean;
  hidden: boolean;
  children: Set<string>;
  layout?: LayoutState;
  [key: string]: any;
}
/**
 * @fileoverview Type definitions for the Vis component
 * 
 * Core TypeScript interfaces and types for the graph visualization system.
 * These types provide compile-time safety and better developer experience.
 */

import { NODE_STYLES, EDGE_STYLES, CONTAINER_STYLES } from './config';

// Re-export constants for components that import them from shared/types
export { NODE_STYLES, EDGE_STYLES, CONTAINER_STYLES } from './config';

// Type definitions from constants
export type NodeStyle = typeof NODE_STYLES[keyof typeof NODE_STYLES];
export type EdgeStyle = typeof EDGE_STYLES[keyof typeof EDGE_STYLES];
export type ContainerStyle = typeof CONTAINER_STYLES[keyof typeof CONTAINER_STYLES];

export interface Dimensions {
  width: number;
  height: number;
}

export interface Position {
  x: number;
  y: number;
}

export interface LayoutState {
  position?: Position;
  dimensions?: Dimensions;
  sections?: any[]; // ELK edge routing sections
  elkFixed?: boolean; // Whether ELK should fix this element's position
  elkLayoutOptions?: Record<string, string>; // ELK-specific layout options
}

export interface GraphNode {
  id: string;
  label: string;
  style: NodeStyle;
  hidden: boolean;
  layout?: LayoutState; // Layout-related properties
  [key: string]: any; // Allow custom properties
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  style?: EdgeStyle | string;
  hidden?: boolean;
  type: 'graph';
}

export interface Container {
  id: string;
  expandedDimensions: Dimensions;
  collapsed: boolean;
  hidden: boolean;
  children: Set<string>;
  layout?: LayoutState; // Layout-related properties
  [key: string]: any; // Allow custom properties
}

export interface HyperEdge {
  id: string;
  source: string;
  target: string;
  style: EdgeStyle;
  type: 'hyper';
  [key: string]: any; // Allow custom properties
}

// ============ Input Types for Methods ============

export interface CreateNodeProps {
  label: string;
  style?: NodeStyle;
  hidden?: boolean;
  layout?: LayoutState;
  [key: string]: any;
}

export interface CreateEdgeProps {
  source: string;
  target: string;
  style?: EdgeStyle;
  hidden?: boolean;
  layout?: LayoutState;
  [key: string]: any;
}

export interface CreateContainerProps {
  expandedDimensions?: Dimensions;
  collapsed?: boolean;
  hidden?: boolean;
  children?: string[];
  layout?: LayoutState;
  [key: string]: any;
}

// ============ Parser Types ============

export interface ParseResult {
  state: any; // Will be replaced with actual VisualizationState type
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

// ============ Visualization State Interface ============

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
  setContainer(id: string, props: CreateContainerProps): ExternalContainer;
  getContainer(id: string): ExternalContainer | undefined;
  setContainerHidden(id: string, hidden: boolean): void;
  getContainerHidden(id: string): boolean | undefined;

  // Visibility properties (readonly getters)
  readonly visibleNodes: GraphNode[];
  readonly visibleEdges: GraphEdge[];
  readonly visibleContainers: ExternalContainer[];
  readonly allHyperEdges: HyperEdge[];

  // Container hierarchy methods
  addContainerChild(containerId: string, childId: string): void;
  removeContainerChild(containerId: string, childId: string): void;
  getContainerChildren(containerId: string): Set<string> | undefined;
  getNodeContainer(nodeId: string): string | undefined;

  // Container operations
  collapseContainer(containerId: string): void;
  expandContainer(containerId: string): void;
}
