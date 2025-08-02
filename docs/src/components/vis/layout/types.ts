/**
 * @fileoverview Minimal layout types
 */

import type { ElkNode } from 'elkjs';
import type { GraphNode, GraphEdge, Container, HyperEdge } from '../shared/types';

export interface LayoutConfig {
  algorithm?: 'layered' | 'stress' | 'mrtree' | 'radial' | 'force';
  direction?: 'DOWN' | 'UP' | 'LEFT' | 'RIGHT';
  spacing?: number;
  nodeSize?: { width: number; height: number };
}

export interface LayoutResult {
  nodes: Array<GraphNode & { x: number; y: number; width: number; height: number }>;
  edges: Array<GraphEdge & { points?: Array<{ x: number; y: number }> }>;
  containers: Array<Container & { x: number; y: number; width: number; height: number }>;
  hyperEdges: Array<HyperEdge & { points?: Array<{ x: number; y: number }> }>;
}

export interface LayoutEngine {
  layout(
    nodes: GraphNode[],
    edges: GraphEdge[],
    containers: Container[],
    hyperEdges: HyperEdge[],
    config?: LayoutConfig
  ): Promise<LayoutResult>;
}
