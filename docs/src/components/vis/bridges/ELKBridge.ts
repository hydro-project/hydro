/**
 * @fileoverview ELK Bridge - Clean interface between VisState and ELK
 * 
 * This bridge implements the core architectural principle:
 * - VisState contains ALL data (nodes, edges, hyperEdges, containers)
 * - ELK gets visible elements only, with no distinction between edge types
 * - ELK returns layout positions that get applied back to VisState
 */

import type { VisualizationState } from '../core/VisState';
import type { GraphNode, GraphEdge, HyperEdge, Container } from '../alpha/shared/types';
import ELK from 'elkjs';
import type { ElkGraph, ElkNode, ElkEdge } from './elk-types';

export class ELKBridge {
  private elk = new ELK();

  /**
   * Convert VisState to ELK format and run layout
   * Key insight: Include ALL visible edges (regular + hyper) with no distinction
   */
  async layoutVisState(visState: VisualizationState): Promise<void> {
    console.log('[ELKBridge] 🚀 Starting ELK layout from VisState');
    
    // 1. Extract all visible data from VisState
    const elkGraph = this.visStateToELK(visState);
    
    // 2. Run ELK layout
    console.log('[ELKBridge] 📊 Sending to ELK:', {
      nodes: elkGraph.children?.length || 0,
      edges: elkGraph.edges?.length || 0
    });
    
    const elkResult = await this.elk.layout(elkGraph);
    
    console.log('[ELKBridge] ✅ ELK layout complete');
    
    // 3. Apply results back to VisState
    this.elkToVisState(elkResult, visState);
  }

  /**
   * Convert VisState to ELK format
   */
  private visStateToELK(visState: VisualizationState): ElkGraph {
    // Extract visible nodes (both regular nodes and collapsed containers)
    const visibleNodes = this.extractVisibleNodes(visState);
    
    // Extract container hierarchy for visible containers
    const visibleContainers = this.extractVisibleContainers(visState);
    
    // Extract ALL edges (regular + hyperedges) - this is the key fix!
    const allEdges = this.extractAllEdges(visState);
    
    console.log('[ELKBridge] 📋 Extracted from VisState:', {
      visibleNodes: visibleNodes.length,
      visibleContainers: visibleContainers.length,
      totalEdges: allEdges.length,
      regularEdges: allEdges.filter(e => !e.id.includes('hyper_')).length,
      hyperEdges: allEdges.filter(e => e.id.includes('hyper_')).length
    });
    
    return this.buildELKGraph(visibleNodes, visibleContainers, allEdges);
  }

  /**
   * Extract visible nodes (both GraphNodes and collapsed containers as nodes)
   */
  private extractVisibleNodes(visState: VisualizationState): GraphNode[] {
    const nodes: GraphNode[] = [];
    
    // Add visible regular nodes using the correct VisState API
    const visibleNodes = visState.visibleNodes;
    nodes.push(...visibleNodes);
    
    // Add collapsed containers as nodes (they should be treated as regular nodes by ELK)
    const visibleContainers = visState.visibleContainers;
    visibleContainers.forEach(container => {
      if (container.collapsed) {
        // Convert collapsed container to a node-like structure for ELK
        const containerAsNode: GraphNode = {
          id: container.id,
          label: container.id,
          // Use collapsed dimensions if available, otherwise use defaults
          width: container.layout?.dimensions?.width || 200,  // SIZES.COLLAPSED_CONTAINER_WIDTH
          height: container.layout?.dimensions?.height || 60, // SIZES.COLLAPSED_CONTAINER_HEIGHT
          x: container.layout?.position?.x || 0,
          y: container.layout?.position?.y || 0,
          hidden: false,
          style: 'default' // Use valid NodeStyle
        };
        nodes.push(containerAsNode);
      }
    });
    
    return nodes;
  }

  /**
   * Extract visible containers (only expanded ones that need hierarchical layout)
   */
  private extractVisibleContainers(visState: VisualizationState): Container[] {
    const containers: Container[] = [];
    
    const expandedContainers = visState.expandedContainers;
    containers.push(...expandedContainers);
    
    return containers;
  }

  /**
   * Extract ALL edges - both regular edges and hyperedges with no distinction
   * This is the critical fix: hyperedges were getting lost in the old implementation
   */
  private extractAllEdges(visState: VisualizationState): (GraphEdge | HyperEdge)[] {
    const allEdges: (GraphEdge | HyperEdge)[] = [];
    
    // Add visible regular edges
    const visibleEdges = visState.visibleEdges;
    allEdges.push(...visibleEdges);
    
    // Add ALL hyperedges (this was missing in the old implementation!)
    const hyperEdges = visState.allHyperEdges;
    allEdges.push(...hyperEdges);
    
    return allEdges;
  }

  /**
   * Build ELK graph from extracted data
   */
  private buildELKGraph(
    nodes: GraphNode[], 
    containers: Container[], 
    edges: (GraphEdge | HyperEdge)[]
  ): ElkGraph {
    // Build hierarchy: top-level nodes and containers
    const elkNodes: ElkNode[] = [];
    
    // Add expanded containers as ELK nodes with children
    containers.forEach(container => {
      const childNodes = nodes.filter(node => {
        // Find nodes that belong to this container using VisState's hierarchy info
        return this.isNodeInContainer(node.id, container.id, container);
      });
      
      elkNodes.push({
        id: container.id,
        width: container.layout?.dimensions?.width,
        height: container.layout?.dimensions?.height,
        children: childNodes.map(node => ({
          id: node.id,
          width: node.width || 180,
          height: node.height || 60
        })),
        layoutOptions: {
          'elk.algorithm': 'layered',
          'elk.direction': 'DOWN',
          'elk.spacing.nodeNode': '75'
        }
      });
    });
    
    // Add top-level nodes (not in any container, including collapsed containers)
    nodes.forEach(node => {
      if (!this.isNodeInAnyContainer(node.id, containers)) {
        elkNodes.push({
          id: node.id,
          width: node.width || 180,
          height: node.height || 60
        });
      }
    });
    
    // Convert all edges to ELK format
    const elkEdges: ElkEdge[] = edges.map(edge => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target]
    }));
    
    return {
      id: 'root',
      children: elkNodes,
      edges: elkEdges,
      layoutOptions: {
        'elk.algorithm': 'layered',
        'elk.direction': 'DOWN',
        'elk.spacing.nodeNode': '100',
        'elk.spacing.edgeNode': '50'
      }
    };
  }

  /**
   * Apply ELK results back to VisState
   */
  private elkToVisState(elkResult: ElkGraph, visState: VisualizationState): void {
    console.log('[ELKBridge] 📝 Applying ELK results back to VisState');
    
    if (!elkResult.children) {
      console.warn('[ELKBridge] ⚠️ No children in ELK result');
      return;
    }
    
    // Apply positions to containers and nodes
    elkResult.children.forEach(elkNode => {
      if (elkNode.children && elkNode.children.length > 0) {
        // This is a container
        this.updateContainerFromELK(elkNode, visState);
      } else {
        // This is a top-level node (or collapsed container)
        this.updateNodeFromELK(elkNode, visState);
      }
    });
    
    console.log('[ELKBridge] ✅ Applied all ELK results to VisState');
  }

  /**
   * Update container dimensions and child positions from ELK result
   */
  private updateContainerFromELK(elkNode: ElkNode, visState: VisualizationState): void {
    const container = visState.getContainer(elkNode.id);
    if (!container) {
      console.warn(`[ELKBridge] Container ${elkNode.id} not found in VisState`);
      return;
    }
    
    // Update container position and dimensions
    container.layout = container.layout || {};
    if (elkNode.x !== undefined) {
      container.layout.position = container.layout.position || {};
      container.layout.position.x = elkNode.x;
    }
    if (elkNode.y !== undefined) {
      container.layout.position = container.layout.position || {};
      container.layout.position.y = elkNode.y;
    }
    if (elkNode.width !== undefined || elkNode.height !== undefined) {
      container.layout.dimensions = container.layout.dimensions || {};
      if (elkNode.width !== undefined) container.layout.dimensions.width = elkNode.width;
      if (elkNode.height !== undefined) container.layout.dimensions.height = elkNode.height;
    }
    
    // Update child node positions
    elkNode.children?.forEach(elkChildNode => {
      this.updateNodeFromELK(elkChildNode, visState);
    });
  }

  /**
   * Update node position from ELK result
   */
  private updateNodeFromELK(elkNode: ElkNode, visState: VisualizationState): void {
    // Try to find as regular node first
    let node = visState.getGraphNode(elkNode.id);
    if (node) {
      node.x = elkNode.x || 0;
      node.y = elkNode.y || 0;
      if (elkNode.width) node.width = elkNode.width;
      if (elkNode.height) node.height = elkNode.height;
      return;
    }
    
    // If not found as node, might be a collapsed container
    const container = visState.getContainer(elkNode.id);
    if (container && container.collapsed) {
      container.layout = container.layout || {};
      if (elkNode.x !== undefined || elkNode.y !== undefined) {
        container.layout.position = container.layout.position || {};
        if (elkNode.x !== undefined) container.layout.position.x = elkNode.x;
        if (elkNode.y !== undefined) container.layout.position.y = elkNode.y;
      }
      if (elkNode.width !== undefined || elkNode.height !== undefined) {
        container.layout.dimensions = container.layout.dimensions || {};
        if (elkNode.width !== undefined) container.layout.dimensions.width = elkNode.width;
        if (elkNode.height !== undefined) container.layout.dimensions.height = elkNode.height;
      }
      return;
    }
    
    console.warn(`[ELKBridge] Node/Container ${elkNode.id} not found in VisState`);
  }

  // Helper methods for containment logic
  private isNodeInContainer(nodeId: string, containerId: string, container: Container): boolean {
    // Use the container's children set
    return container.children.has(nodeId);
  }
  
  private isNodeInAnyContainer(nodeId: string, containers: Container[]): boolean {
    return containers.some(container => this.isNodeInContainer(nodeId, container.id, container));
  }
}
