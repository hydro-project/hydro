/**
 * Factory functions for creating properly typed GraphEdges and HyperEdges
 * 
 * These helper functions ensure that all edges are created with the correct
 * type annotation, making type guards reliable and preventing inconsistencies.
 */

import { GraphEdge, HyperEdge, EdgeStyle } from './types';

/**
 * Create a new GraphEdge with proper type annotation
 */
export function createGraphEdge(props: {
  id: string;
  source: string;
  target: string;
  style?: EdgeStyle | string;
  hidden?: boolean;
}): GraphEdge {
  return {
    type: 'graph',
    id: props.id,
    source: props.source,
    target: props.target,
    style: props.style,
    hidden: props.hidden || false
  };
}

/**
 * Create a new HyperEdge with proper type annotation
 */
export function createHyperEdge(props: {
  id: string;
  source: string;
  target: string;
  style?: EdgeStyle | string;
  hidden?: boolean;
  aggregatedEdges?: Map<string, GraphEdge>;
}): HyperEdge {
  return {
    type: 'hyper',
    id: props.id,
    source: props.source,
    target: props.target,
    style: props.style,
    hidden: props.hidden || false,
    aggregatedEdges: props.aggregatedEdges || new Map()
  };
}

/**
 * Utility to check if an edge object has the correct type field
 */
export function validateEdgeType(edge: any): edge is GraphEdge | HyperEdge {
  return edge && typeof edge === 'object' && 
         (edge.type === 'graph' || edge.type === 'hyper') &&
         typeof edge.id === 'string' &&
         typeof edge.source === 'string' &&
         typeof edge.target === 'string';
}
