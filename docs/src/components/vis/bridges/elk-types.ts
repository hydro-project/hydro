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

export interface ElkEdge {
  id: string;
  sources: string[];
  targets: string[];
}

export interface ElkGraph {
  id: string;
  layoutOptions?: Record<string, any>;
  children?: ElkNode[];
  edges?: ElkEdge[];
}
