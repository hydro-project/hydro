/**
 * @fileoverview ELK Bridge - Refactored to be DRY, stateless, and focused on format translation
 * 
 * This bridge now separates business logic from format transformation while maintaining
 * backward compatibility with existing tests and components.
 */

import type { VisualizationState } from '../core/VisState';
import type { LayoutConfig } from '../core/types';
import { getELKLayoutOptions } from '../shared/config';

import ELK from 'elkjs';
import type { ElkGraph, ElkNode, ElkEdge } from './elk-types';

export class ELKBridge {
  private layoutConfig: LayoutConfig;

  constructor(layoutConfig: LayoutConfig = {}) {
    this.layoutConfig = { algorithm: 'layered', ...layoutConfig };
    console.log(`[ELKBridge] 🆕 Created with config: ${JSON.stringify(this.layoutConfig)}`);
  }

  /**
   * Update layout configuration
   */
  updateLayoutConfig(config: LayoutConfig): void {
    this.layoutConfig = { ...this.layoutConfig, ...config };
  }

  /**
   * Convert VisState to ELK format and run layout
   * Now with cleaner separation between business logic and format translation
   */
  async layoutVisState(visState: VisualizationState): Promise<void> {
    console.log(`[ELKBridge] 🚀 Starting ELK layout from VisState`);
    
    try {
      // 1. Extract data using VisualizationState's official API (no business logic here)
      const elkGraph = this.visStateToELK(visState);
      
      // 2. Validate format (not business logic)
      this.validateELKInput(elkGraph);
      
      // 3. Run ELK layout (pure transformation)
      const elk = new ELK();
      const elkResult = await elk.layout(elkGraph);
      
      // 4. Apply results back to VisState (pure transformation)
      this.elkToVisState(elkResult, visState);
      
      console.log(`[ELKBridge] ✅ ELK layout complete`);
      
    } catch (error) {
      console.error('[ELKBridge] ❌ Layout failed:', error);
      throw error;
    }
  }

  /**
   * Convert VisualizationState to ELK format (pure transformation)
   */
  private visStateToELK(visState: VisualizationState): ElkGraph {
    // Use VisualizationState's official visible getters (business logic is already done)
    const visibleNodes = Array.from(visState.visibleNodes);
    const visibleContainers = Array.from(visState.visibleContainers);
    const visibleEdges = Array.from(visState.visibleEdges);
    
    console.log(`[ELKBridge] 📋 Converting: ${visibleNodes.length} nodes, ${visibleContainers.length} containers, ${visibleEdges.length} edges`);
    
    return this.buildELKGraph(visibleNodes, visibleContainers, visibleEdges);
  }

  /**
   * Build ELK graph from visible elements (pure transformation)
   */
  private buildELKGraph(nodes: any[], containers: any[], edges: any[]): ElkGraph {
    const elkNodes: ElkNode[] = [];
    
    // Build container hierarchy
    const rootContainers = this.findRootContainers(containers);
    rootContainers.forEach(container => {
      const hierarchyNode = this.buildContainerHierarchy(container, containers, nodes);
      elkNodes.push(hierarchyNode);
    });
    
    // Add top-level nodes (not in any container, excluding collapsed containers)
    const collapsedContainerIds = new Set(containers.filter(c => c.collapsed).map(c => c.id));
    const topLevelNodes = nodes.filter(node => 
      !this.isNodeInAnyContainer(node.id, containers) && 
      !collapsedContainerIds.has(node.id)
    );
    
    topLevelNodes.forEach(node => {
      elkNodes.push({
        id: node.id,
        width: node.width || 180,
        height: node.height || 60
      });
    });
    
    // Convert edges to ELK format
    const elkEdges: ElkEdge[] = edges.map(edge => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target]
    }));
    
    return {
      id: 'root',
      children: elkNodes,
      edges: elkEdges,
      layoutOptions: getELKLayoutOptions(this.layoutConfig.algorithm, nodes.length)
    };
  }

  /**
   * Find root containers (no parents) - pure logic
   */
  private findRootContainers(containers: any[]): any[] {
    return containers.filter(container => {
      const hasContainerParent = containers.some(otherContainer => 
        otherContainer.children && otherContainer.children.has(container.id)
      );
      return !hasContainerParent;
    });
  }

  /**
   * Build container hierarchy recursively - pure transformation
   */
  private buildContainerHierarchy(container: any, allContainers: any[], allNodes: any[]): ElkNode {
    // Find children
    const childNodes = allNodes.filter(node => container.children.has(node.id));
    const childContainers = allContainers.filter(childContainer => 
      container.children.has(childContainer.id)
    );
    
    // Create ELK children
    const elkChildren: ElkNode[] = [
      ...childNodes.map(node => ({
        id: node.id,
        width: node.width || 180,
        height: node.height || 60
      })),
      ...childContainers.map(childContainer => 
        this.buildContainerHierarchy(childContainer, allContainers, allNodes)
      )
    ];
    
    return {
      id: container.id,
      width: container.width || 200,
      height: container.height || 150,
      children: elkChildren,
      layoutOptions: getELKLayoutOptions(this.layoutConfig.algorithm, allNodes.length)
    };
  }

  /**
   * Check if node is in any container - pure logic
   */
  private isNodeInAnyContainer(nodeId: string, containers: any[]): boolean {
    return containers.some(container => container.children && container.children.has(nodeId));
  }

  /**
   * Validate ELK input format - format validation only
   */
  private validateELKInput(elkGraph: ElkGraph): void {
    if (!elkGraph.children) elkGraph.children = [];
    if (!elkGraph.edges) elkGraph.edges = [];
    
    // Validate node format
    elkGraph.children.forEach(node => {
      if (!node.id) {
        throw new Error(`ELK node missing ID`);
      }
      if (typeof node.width !== 'number' || node.width <= 0) {
        throw new Error(`ELK node ${node.id} has invalid width: ${node.width}`);
      }
      if (typeof node.height !== 'number' || node.height <= 0) {
        throw new Error(`ELK node ${node.id} has invalid height: ${node.height}`);
      }
    });
    
    // Validate edge format
    elkGraph.edges.forEach(edge => {
      if (!edge.id) throw new Error(`ELK edge missing ID`);
      if (!edge.sources || edge.sources.length === 0) {
        throw new Error(`ELK edge missing sources: ${edge.id}`);
      }
      if (!edge.targets || edge.targets.length === 0) {
        throw new Error(`ELK edge missing targets: ${edge.id}`);
      }
    });
  }

  /**
   * Apply ELK results back to VisualizationState - pure transformation
   */
  private elkToVisState(elkResult: ElkGraph, visState: VisualizationState): void {
    if (!elkResult.children) return;
    
    // Apply positions to containers and nodes
    elkResult.children.forEach(elkNode => {
      if (elkNode.children && elkNode.children.length > 0) {
        this.updateContainerFromELK(elkNode, visState);
      } else {
        this.updateNodeFromELK(elkNode, visState);
      }
    });
    
    // Apply edge routing
    const allEdges = elkResult.edges || [];
    allEdges.forEach(elkEdge => {
      this.updateEdgeFromELK(elkEdge, visState);
    });
  }

  /**
   * Update container from ELK result - pure transformation
   */
  private updateContainerFromELK(elkNode: ElkNode, visState: VisualizationState): void {
    const layoutUpdates: any = {};
    
    if (elkNode.x !== undefined || elkNode.y !== undefined) {
      layoutUpdates.position = {};
      if (elkNode.x !== undefined) layoutUpdates.position.x = elkNode.x;
      if (elkNode.y !== undefined) layoutUpdates.position.y = elkNode.y;
    }
    
    if (elkNode.width !== undefined || elkNode.height !== undefined) {
      layoutUpdates.dimensions = {};
      if (elkNode.width !== undefined) layoutUpdates.dimensions.width = elkNode.width;
      if (elkNode.height !== undefined) layoutUpdates.dimensions.height = elkNode.height;
    }
    
    if (Object.keys(layoutUpdates).length > 0) {
      visState.setContainerLayout(elkNode.id, layoutUpdates);
    }
    
    // Update children recursively
    elkNode.children?.forEach(elkChildNode => {
      if (elkChildNode.children && elkChildNode.children.length > 0) {
        this.updateContainerFromELK(elkChildNode, visState);
      } else {
        this.updateNodeFromELK(elkChildNode, visState);
      }
    });
  }

  /**
   * Update node from ELK result - pure transformation
   */
  private updateNodeFromELK(elkNode: ElkNode, visState: VisualizationState): void {
    const layoutUpdates: any = {};
    
    if (elkNode.x !== undefined || elkNode.y !== undefined) {
      layoutUpdates.position = {};
      if (elkNode.x !== undefined) layoutUpdates.position.x = elkNode.x;
      if (elkNode.y !== undefined) layoutUpdates.position.y = elkNode.y;
    }
    
    if (elkNode.width !== undefined || elkNode.height !== undefined) {
      layoutUpdates.dimensions = {};
      if (elkNode.width !== undefined) layoutUpdates.dimensions.width = elkNode.width;
      if (elkNode.height !== undefined) layoutUpdates.dimensions.height = elkNode.height;
    }
    
    if (Object.keys(layoutUpdates).length > 0) {
      try {
        visState.setNodeLayout(elkNode.id, layoutUpdates);
      } catch (nodeError) {
        // Try as container if not found as node
        try {
          visState.setContainerLayout(elkNode.id, layoutUpdates);
        } catch (containerError) {
          console.warn(`[ELKBridge] Node/Container ${elkNode.id} not found in VisState`);
        }
      }
    }
  }

  /**
   * Update edge from ELK result - pure transformation
   */
  private updateEdgeFromELK(elkEdge: ElkEdge, visState: VisualizationState): void {
    if (elkEdge.sections && elkEdge.sections.length > 0) {
      try {
        visState.setEdgeLayout(elkEdge.id, { sections: elkEdge.sections });
      } catch (error) {
        console.warn(`[ELKBridge] Skipping layout update for edge ${elkEdge.id} - edge no longer exists`);
      }
    }
  }
}