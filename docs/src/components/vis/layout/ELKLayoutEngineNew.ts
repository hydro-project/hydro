/**
 * @fileoverview ELK Layout Engine (Enhanced with working patterns)
 * 
 * ELK-based automatic layout engine using proven patterns from the working visualizer.
 * Handles hierarchical layouts with proper container dimension management.
 */

import { LayoutEngine, LayoutResult, LayoutConfig } from './types';
import { GraphNode, GraphEdge, Container, HyperEdge } from '../shared/types';
import { createELKStateManager, ELKStateManager } from './ELKStateManager';

export class ELKLayoutEngine implements LayoutEngine {
  private elkStateManager: ELKStateManager;
  private dimensionsCache = new Map<string, { width: number; height: number }>();

  constructor() {
    this.elkStateManager = createELKStateManager();
  }

  async layout(
    nodes: GraphNode[],
    edges: GraphEdge[],
    containers: Container[],
    hyperEdges: HyperEdge[],
    config: LayoutConfig = {}
  ): Promise<LayoutResult> {
    try {
      console.log('[ELKLayoutEngine] Starting layout with proven approach...');
      
      // Use the proven ELK state manager approach
      const result = await this.elkStateManager.calculateFullLayout(
        nodes,
        edges,
        containers,
        config.algorithm || 'layered'
      );

      // Cache container dimensions for future use
      result.nodes.forEach(node => {
        if (node.type === 'container' || containers.find(c => c.id === node.id)) {
          this.dimensionsCache.set(node.id, {
            width: node.width || node.dimensions?.width || 400,
            height: node.height || node.dimensions?.height || 300
          });
          console.log(`[ELKLayoutEngine] Cached dimensions for ${node.id}: ${node.width}x${node.height}`);
        }
      });

      // Convert to our LayoutResult format
      const layoutResult: LayoutResult = {
        nodes: result.nodes
          .filter(node => nodes.find(n => n.id === node.id)) // Only include actual nodes
          .map(node => {
            const originalNode = nodes.find(n => n.id === node.id)!;
            return {
              ...originalNode,
              x: node.position?.x || 0,
              y: node.position?.y || 0,
              width: node.width || node.dimensions?.width || 180,
              height: node.height || node.dimensions?.height || 60
            };
          }),
        edges: edges.map(edge => ({
          ...edge,
          points: [] // ELK routing will be added later if needed
        })),
        containers: containers.map(container => {
          const layoutedNode = result.nodes.find(n => n.id === container.id);
          return {
            ...container,
            x: layoutedNode?.position?.x || 0,
            y: layoutedNode?.position?.y || 0,
            width: layoutedNode?.width || layoutedNode?.dimensions?.width || container.expandedDimensions?.width || 400,
            height: layoutedNode?.height || layoutedNode?.dimensions?.height || container.expandedDimensions?.height || 300
          };
        }),
        hyperEdges: hyperEdges.map(hyperEdge => ({
          ...hyperEdge,
          points: []
        }))
      };

      console.log('[ELKLayoutEngine] Layout completed successfully');
      return layoutResult;

    } catch (error) {
      console.error('[ELKLayoutEngine] Layout failed:', error);
      throw error;
    }
  }

  /**
   * Get cached container dimensions
   */
  getCachedDimensions(containerId: string): { width: number; height: number } | undefined {
    return this.dimensionsCache.get(containerId);
  }

  /**
   * Clear the dimensions cache
   */
  clearCache(): void {
    this.dimensionsCache.clear();
  }
}
