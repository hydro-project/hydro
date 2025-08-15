/**
 * @fileoverview Bridge Architecture Types
export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  style?: EdgeStyle | string;
  hidden?: boolean;
  type: 'graph';
}Clean type definitions for our bridge-based implementation.
 * No dependencies on alpha.
 */

import type { NodeStyle, EdgeStyle, ContainerStyle, ExternalContainer } from '../shared/types';

// Re-export style types
export type { NodeStyle, EdgeStyle, ContainerStyle, ExternalContainer } from '../shared/types';

// Basic dimension types
export interface Dimensions {
  width: number;
  height: number;
}

export interface Position {
  x: number;
  y: number;
}

// Core graph element types
export interface GraphNode {
  id: string;
  label: string;
  hidden?: boolean;
  style?: NodeStyle | string;
}

export interface GraphEdge {
  type: 'graph';
  id: string;
  source: string;
  target: string;
  hidden?: boolean;
  style?: EdgeStyle | string;
}

export interface Container {
  id: string;
  collapsed?: boolean;
  hidden?: boolean;
  children?: Set<string>;
  style?: ContainerStyle | string;
}

export interface HyperEdge {
  id: string;
  source: string;
  target: string;
  style?: EdgeStyle | string;
  hidden?: boolean;
  type: 'hyper';
  aggregatedEdges?: Map<string, GraphEdge>;
}

// Union type for all edge types
export type Edge = GraphEdge | HyperEdge;

// Type guards for distinguishing edge types
export function isHyperEdge(edge: Edge): edge is HyperEdge {
  return edge.type === 'hyper';
}

export function isGraphEdge(edge: Edge): edge is GraphEdge {
  return edge.type === 'graph';
}

// Creation props for builder pattern
export interface CreateNodeProps {
  label: string;
  hidden?: boolean;
  style?: NodeStyle | string;
}

export interface CreateEdgeProps {
  source: string;
  target: string;
  hidden?: boolean;
  style?: EdgeStyle | string;
}

export interface CreateContainerProps {
  collapsed?: boolean;
  hidden?: boolean;
  children?: Set<string>;
  style?: ContainerStyle | string;
}

// Layout types
export interface LayoutConfig {
  algorithm?: 'mrtree' | 'layered' | 'force' | 'stress' | 'radial';
  direction?: 'UP' | 'DOWN' | 'LEFT' | 'RIGHT';
  spacing?: number;
  nodeSize?: { width: number; height: number };
  enableSmartCollapse?: boolean;
}

export interface LayoutResult {
  nodes: PositionedNode[];
  edges: PositionedEdge[];
  containers: PositionedContainer[];
}

export interface PositionedNode extends GraphNode, Position, Dimensions {}
export interface PositionedEdge extends GraphEdge {
  points?: Position[];
}
export interface PositionedContainer extends Container, Position, Dimensions {}
export interface PositionedHyperEdge extends HyperEdge {
  points?: Position[];
}

// Union type for positioned edges
export type PositionedAnyEdge = PositionedEdge | PositionedHyperEdge;

// Layout engine interface
export interface LayoutEngine {
  layout(
    nodes: GraphNode[],
    edges: GraphEdge[],
    containers: ExternalContainer[],
    config?: LayoutConfig
  ): Promise<LayoutResult>;
  
  layoutWithChangedContainer?(
    nodes: GraphNode[],
    edges: GraphEdge[],
    containers: ExternalContainer[],
    config?: LayoutConfig,
    changedContainerId?: string | null,
    visualizationState?: any
  ): Promise<LayoutResult>;
}

// Event types
export interface LayoutStatistics {
  totalNodes: number;
  totalEdges: number;
  totalContainers: number;
  layoutDuration: number;
}

export interface LayoutEventData {
  type: 'start' | 'progress' | 'complete' | 'error';
  progress?: number;
  statistics?: LayoutStatistics;
  error?: Error;
}

export type LayoutEventCallback = (data: LayoutEventData) => void;

// Render types
export interface RenderConfig {
  enableMiniMap?: boolean;
  enableControls?: boolean;
  fitView?: boolean;
  nodesDraggable?: boolean;
  snapToGrid?: boolean;
  gridSize?: number;
  nodesConnectable?: boolean;
  elementsSelectable?: boolean;
  enableZoom?: boolean;
  enablePan?: boolean;
  enableSelection?: boolean;
  colorPalette?: string;
}

export interface FlowGraphEventHandlers {
  onNodeClick?: (event: any, node: any) => void;
  onEdgeClick?: (event: any, edge: any) => void;
  onNodeDrag?: (event: any, node: any) => void;
  onFitViewRequested?: () => void;
}
