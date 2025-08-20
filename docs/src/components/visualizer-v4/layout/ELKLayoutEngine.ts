/**
 * @fileoverview New Bridge-Based Layout Engine
 * 
 * Complete replacement for alpha ELKLayoutEngine using our bridge architecture.
 * Maintains identical API while using the new VisualizationEngine internally.
 */

import { VisualizationEngine, createVisualizationEngine } from '../core/VisualizationEngine';
import { createVisualizationState } from '../core/VisualizationState';
import type { GraphNode, GraphEdge, ExternalContainer, HyperEdge } from '../shared/types';
import type { NodeStyle, EdgeStyle } from '../shared/config';
import type {
  PositionedNode,
  PositionedEdge, 
  PositionedContainer,
  PositionedHyperEdge,
  LayoutResult,
  LayoutEngine,
  LayoutConfig,
  LayoutEventCallback,
  LayoutStatistics
} from '../core/types';

export class ELKLayoutEngine implements LayoutEngine {
  private config: LayoutConfig;
  private callbacks: Map<string, LayoutEventCallback> = new Map();
  private lastStatistics: LayoutStatistics | null = null;

  constructor(config: LayoutConfig = {}) {
    this.config = { ...DEFAULT_LAYOUT_CONFIG, ...config };
  }

  /**
   * Run layout - SAME API as alpha
   */
  async layout(
    nodes: GraphNode[],
    edges: GraphEdge[],
    containers: ExternalContainer[],
    config?: LayoutConfig
  ): Promise<LayoutResult> {
    // ...existing code...
    
    const finalConfig = { ...this.config, ...config };
    const startTime = Date.now();
    
    try {
      // Create temporary VisState and load data
      const visState = createVisualizationState();
      
      // Load nodes
      nodes.forEach(node => {
        visState.setGraphNode(node.id, {
          label: node.label,
          hidden: node.hidden || false,
          style: (node.style || 'default') as NodeStyle,
          ...node
        });
      });
      
      // Load edges
      edges.forEach(edge => {
        visState.setGraphEdge(edge.id, {
          source: edge.source,
          target: edge.target,
          hidden: edge.hidden || false,
          style: (edge.style || 'default') as EdgeStyle
        });
      });
      
      // Load containers
      containers.forEach(container => {
        visState.setContainer(container.id, {
          collapsed: container.collapsed || false,
          hidden: container.hidden || false,
          children: Array.from(container.children || new Set()),
          style: container.style || 'default'
        });
      });
      
      // Create engine and run layout
      const engine = createVisualizationEngine(visState, {
        autoLayout: false,
        enableLogging: false
      });
      
      // Emit start event
      this.emit('layout', { type: 'start' });
      
      // Use our bridge-based engine
      await engine.runLayout();
      
      const endTime = Date.now();
      const duration = endTime - startTime;
      
      // Convert results back to alpha format
      const result: LayoutResult = {
        nodes: this.convertNodes([...visState.visibleNodes]),
        edges: this.convertEdges([...visState.visibleEdges]),
        containers: this.convertContainers([...visState.visibleContainers])
      };
      
      // Update statistics
      this.lastStatistics = {
        totalNodes: result.nodes.length,
        totalEdges: result.edges.length,
        totalContainers: result.containers.length,
        layoutDuration: duration
      };
      
      // Emit completion event
      this.emit('layout', { 
        type: 'complete', 
        statistics: this.lastStatistics 
      });
      
      // ...existing code...
      engine.dispose();
      return result;
      
    } catch (error) {
      const errorData = { 
        type: 'error' as const, 
        error: error instanceof Error ? error : new Error(String(error))
      };
      
      this.emit('layout', errorData);
      throw error;
    }
  }

  /**
   * Layout with changed container - compatibility method
   */
  async layoutWithChangedContainer(
    nodes: GraphNode[],
    edges: GraphEdge[],
    containers: ExternalContainer[],
    config?: LayoutConfig,
    changedContainerId?: string | null,
    visualizationState?: any
  ): Promise<LayoutResult> {
    // For now, just call regular layout - the bridge architecture handles changes efficiently
    return this.layout(nodes, edges, containers, config);
  }

  /**
   * Convert nodes to positioned format
   */
  private convertNodes(nodes: any[]): PositionedNode[] {
    return nodes.map(node => ({
      ...node,
      x: node.x || 0,
      y: node.y || 0,
      width: node.width || 180,
      height: node.height || 60
    }));
  }

  /**
   * Convert edges to positioned format
   */
  private convertEdges(edges: any[]): PositionedEdge[] {
    return edges.map(edge => ({
      ...edge,
      points: [] // ELK doesn't provide edge routing in our simple implementation
    }));
  }

  /**
   * Convert containers to positioned format
   */
  private convertContainers(containers: any[]): PositionedContainer[] {
    return containers.map(container => ({
      ...container,
      x: container.x || 0,
      y: container.y || 0,
      width: container.width || 400,
      height: container.height || 300
    }));
  }

  /**
   * Emit event to listeners
   */
  private emit(event: string, data: any): void {
    const callback = this.callbacks.get(event);
    if (callback) {
      try {
        callback(data);
      } catch (error) {
        console.error(`[ELKLayoutEngine] Event callback error:`, error);
      }
    }
  }

  /**
   * Get last layout statistics
   */
  getLastLayoutStatistics(): LayoutStatistics | null {
    return this.lastStatistics;
  }

  /**
   * Add event listener
   */
  on(event: string, callback: LayoutEventCallback): void {
    this.callbacks.set(event, callback);
  }

  /**
   * Remove event listener  
   */
  off(event: string, callback: LayoutEventCallback): void {
    this.callbacks.delete(event);
  }
}

/**
 * Default layout configuration - MRTree as default for better hierarchical display
 */
export const DEFAULT_LAYOUT_CONFIG: LayoutConfig = {
  algorithm: 'mrtree',
  direction: 'DOWN',
  spacing: 100,
  nodeSize: { width: 180, height: 60 }
};


