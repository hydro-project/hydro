/**
 * @fileoverview Type declarations for elkjs for the bridges
 */

export interface ElkNode {
  id: string;
  x?: number;
  y?: number;
  width?: number;
  height?: number;
  children?: ElkNode[];
  layoutOptions?: Record<string, any>;
}

export interface ElkEdgeSection {
  id: string;
  startPoint: { x: number; y: number };
  endPoint: { x: number; y: number };
  incomingShape?: string;
  outgoingShape?: string;
  bendPoints?: { x: number; y: number }[];
}

export interface ElkEdge {
  id: string;
  sources: string[];
  targets: string[];
  sections?: ElkEdgeSection[];
  container?: string;
}

export interface ElkGraph {
  id: string;
  layoutOptions?: Record<string, any>;
  children?: ElkNode[];
  edges?: ElkEdge[];
}
