/**
 * @fileoverview Type declarations for elkjs
 * 
 * Minimal type declarations for ELK.js library to support development
 * when the actual package is not installed.
 */

declare module 'elkjs' {
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

  export default class ELK {
    layout(graph: ElkGraph): Promise<ElkGraph>;
  }
}
