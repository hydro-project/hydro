/**
 * @fileoverview ReactFlow Data Converter
 * 
 * Converts positioned layout data to ReactFlow-compatible format.
 */

import { LayoutResult } from '../layout/types';
import { Node, Edge } from 'reactflow';

export class ReactFlowConverter {
  static convert(layoutResult: LayoutResult): { nodes: Node[], edges: Edge[] } {
    const nodes: Node[] = [];
    const edges: Edge[] = [];

    // Convert nodes
    layoutResult.nodes.forEach(node => {
      nodes.push({
        id: node.id,
        type: 'standard',
        position: { x: node.x || 0, y: node.y || 0 },
        data: { 
          label: node.label || node.id,
          style: node.style || 'default'
        }
      });
    });

    // Convert containers
    layoutResult.containers.forEach(container => {
      nodes.push({
        id: container.id,
        type: 'container',
        position: { x: container.x || 0, y: container.y || 0 },
        data: { 
          label: container.id,
          collapsed: container.collapsed || false
        }
      });
    });

    // Convert edges
    layoutResult.edges.forEach(edge => {
      edges.push({
        id: edge.id,
        type: 'standard',
        source: edge.source,
        target: edge.target,
        data: {
          style: edge.style || 'default'
        }
      });
    });

    // Convert hyperEdges
    layoutResult.hyperEdges.forEach(hyperEdge => {
      edges.push({
        id: hyperEdge.id,
        type: 'hyper',
        source: hyperEdge.source,
        target: hyperEdge.target,
        data: {
          style: hyperEdge.style || 'default'
        }
      });
    });

    return { nodes, edges };
  }
}
