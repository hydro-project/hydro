/**
 * @fileoverview Minimal ELK Layout Engine
 */

import ELK from 'elkjs';
import type { GraphNode, GraphEdge, Container, HyperEdge } from '../shared/types';
import type { LayoutConfig, LayoutResult, LayoutEngine } from './types';
import { DEFAULT_LAYOUT_CONFIG } from './config';

export class ELKLayoutEngine implements LayoutEngine {
  private elk = new ELK();

  async layout(
    nodes: GraphNode[],
    edges: GraphEdge[],
    containers: Container[],
    hyperEdges: HyperEdge[],
    config: LayoutConfig = {}
  ): Promise<LayoutResult> {
    const finalConfig = { ...DEFAULT_LAYOUT_CONFIG, ...config };
    
    // Build ELK graph
    const elkGraph = {
      id: 'root',
      layoutOptions: {
        'elk.algorithm': finalConfig.algorithm,
        'elk.direction': finalConfig.direction,
        'elk.spacing.nodeNode': finalConfig.spacing.toString()
      },
      children: [
        // Containers as parent nodes
        ...containers.map(container => ({
          id: container.id,
          width: container.expandedDimensions.width,
          height: container.expandedDimensions.height,
          children: nodes
            .filter(node => container.children.has(node.id))
            .map(node => ({
              id: node.id,
              width: finalConfig.nodeSize.width,
              height: finalConfig.nodeSize.height
            }))
        })),
        // Standalone nodes
        ...nodes
          .filter(node => !containers.some(c => c.children.has(node.id)))
          .map(node => ({
            id: node.id,
            width: finalConfig.nodeSize.width,
            height: finalConfig.nodeSize.height
          }))
      ],
      edges: [
        ...edges.map(edge => ({ id: edge.id, sources: [edge.source], targets: [edge.target] })),
        ...hyperEdges.map(edge => ({ id: edge.id, sources: [edge.source], targets: [edge.target] }))
      ]
    };

    const layouted = await this.elk.layout(elkGraph);
    
    // Convert back to our format
    const nodeMap = new Map<string, any>();
    const edgeMap = new Map<string, any>();
    
    const processNodes = (elkNodes: any[], offsetX = 0, offsetY = 0) => {
      elkNodes?.forEach(elkNode => {
        const original = nodes.find(n => n.id === elkNode.id) || containers.find(c => c.id === elkNode.id);
        if (original && elkNode.x !== undefined && elkNode.y !== undefined) {
          nodeMap.set(elkNode.id, {
            ...original,
            x: elkNode.x + offsetX,
            y: elkNode.y + offsetY,
            width: elkNode.width,
            height: elkNode.height
          });
        }
        if (elkNode.children) processNodes(elkNode.children, elkNode.x + offsetX, elkNode.y + offsetY);
      });
    };
    
    processNodes(layouted.children || []);
    
    // Process edges
    layouted.edges?.forEach(elkEdge => {
      const original = edges.find(e => e.id === elkEdge.id) || hyperEdges.find(e => e.id === elkEdge.id);
      if (original) {
        edgeMap.set(elkEdge.id, {
          ...original,
          points: elkEdge.sections?.[0]?.bendPoints?.map((bp: any) => ({ x: bp.x, y: bp.y }))
        });
      }
    });

    return {
      nodes: Array.from(nodeMap.values()).filter(n => nodes.some(node => node.id === n.id)),
      edges: Array.from(edgeMap.values()).filter(e => edges.some(edge => edge.id === e.id)),
      containers: Array.from(nodeMap.values()).filter(n => containers.some(container => container.id === n.id)),
      hyperEdges: Array.from(edgeMap.values()).filter(e => hyperEdges.some(edge => edge.id === e.id))
    };
  }
}
