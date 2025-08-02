/**
 * Visualization Design Constants
 * 
 * Centralized constants for styling and configuration of the visualization system
 */

// Node style constants
export const NODE_STYLES = {
  DEFAULT: 'default',
  HIGHLIGHTED: 'highlighted',
  SELECTED: 'selected',
  WARNING: 'warning',
  ERROR: 'error'
} as const;

// Edge style constants
export const EDGE_STYLES = {
  DEFAULT: 'default',
  HIGHLIGHTED: 'highlighted',
  DASHED: 'dashed',
  THICK: 'thick',
  WARNING: 'warning'
} as const;

// Container style constants
export const CONTAINER_STYLES = {
  DEFAULT: 'default',
  HIGHLIGHTED: 'highlighted',
  SELECTED: 'selected',
  MINIMIZED: 'minimized'
} as const;

// Layout constants
export const LAYOUT_CONSTANTS = {
  DEFAULT_NODE_WIDTH: 100,
  DEFAULT_NODE_HEIGHT: 40,
  DEFAULT_CONTAINER_PADDING: 20,
  MIN_CONTAINER_WIDTH: 150,
  MIN_CONTAINER_HEIGHT: 100
} as const;

// ============ Type Definitions ============

export type NodeStyle = typeof NODE_STYLES[keyof typeof NODE_STYLES];
export type EdgeStyle = typeof EDGE_STYLES[keyof typeof EDGE_STYLES];
export type ContainerStyle = typeof CONTAINER_STYLES[keyof typeof CONTAINER_STYLES];

export interface Dimensions {
  width: number;
  height: number;
}

export interface BaseEntity {
  id: string;
  hidden: boolean;
}

export interface GraphNode extends BaseEntity {
  label: string;
  style: NodeStyle;
}

export interface GraphEdge extends BaseEntity {
  source: string;
  target: string;
  style: EdgeStyle;
}

export interface Container extends BaseEntity {
  expandedDimensions: Dimensions;
  collapsed: boolean;
  children: Set<string>;
  label?: string;
}

export interface HyperEdge {
  id: string;
  source: string;
  target: string;
  style: EdgeStyle;
  aggregatedEdges: GraphEdge[];
}

export interface CollapsedContainer {
  id: string;
  originalContainer: Container;
  position: { x: number; y: number };
  dimensions: Dimensions;
}

// ============ Input Types for Methods ============

export interface CreateNodeProps {
  label: string;
  style?: NodeStyle;
  hidden?: boolean;
}

export interface CreateEdgeProps {
  source: string;
  target: string;
  style?: EdgeStyle;
  hidden?: boolean;
}

export interface CreateContainerProps {
  expandedDimensions?: Dimensions;
  collapsed?: boolean;
  hidden?: boolean;
  children?: string[];
  label?: string;
}
