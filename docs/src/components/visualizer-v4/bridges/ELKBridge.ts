/**
 * @fileoverview ELK Bridge - Pure transformation between VisualizationState and ELK format
 * 
 * This bridge is now a stateless, pure transformation layer:
 * - NO business logic - only format translation
 * - NO state management - just pure functions
 * - NO decisions about what to include - VisualizationEngine decides that
 * - Focuses solely on converting between VisualizationState and ELK formats
 */

import type { VisualizationState } from '../core/VisState';
import type { LayoutConfig } from '../core/types';
import { getELKLayoutOptions } from '../shared/config';

import ELK from 'elkjs';
import type { ElkGraph, ElkNode, ElkEdge } from './elk-types';

/**
 * Pure ELK transformation functions - no state, no business logic
 */
export class ELKBridge {
  /**
   * Convert VisualizationState to ELK format (pure transformation)
   * All business logic decisions are made by VisualizationEngine before calling this
   */
  static visStateToELK(visState: VisualizationState, layoutConfig: LayoutConfig): ElkGraph {
    // Extract data that VisualizationEngine has already determined should be visible
    const visibleNodes = visState.visibleNodes;
    const visibleContainers = visState.visibleContainers;
    const visibleEdges = Array.from(visState.visibleEdges);
    
    // Pure transformation - just convert formats
    return ELKBridge.buildELKGraph(Array.from(visibleNodes), Array.from(visibleContainers), visibleEdges, layoutConfig);
  }

  /**
   * Apply ELK results back to VisualizationState (pure transformation)
   */
  static elkToVisState(elkResult: ElkGraph, visState: VisualizationState): void {
    if (!elkResult.children) {
      return;
    }
    
    // Apply positions and dimensions back to VisState
    elkResult.children.forEach(elkNode => {
      if (elkNode.children && elkNode.children.length > 0) {
        // This is a container
        ELKBridge.updateContainerFromELK(elkNode, visState);
      } else {
        // This is a node (including collapsed containers treated as nodes)
        ELKBridge.updateNodeFromELK(elkNode, visState);
      }
    });
    
    // Apply edge routing information
    const allEdges = elkResult.edges || [];
    allEdges.forEach(elkEdge => {
      ELKBridge.updateEdgeFromELK(elkEdge, visState);
    });
  }

  /**
   * Run ELK layout (orchestration method - called by VisualizationEngine)
   */
  static async layoutVisState(visState: VisualizationState, layoutConfig: LayoutConfig): Promise<void> {
    // 1. Convert to ELK format
    const elkGraph = ELKBridge.visStateToELK(visState, layoutConfig);
    
    // 2. Validate ELK input
    ELKBridge.validateELKInput(elkGraph);
    
    // 3. Run ELK layout
    const elk = new ELK();
    const elkResult = await elk.layout(elkGraph);
    
    // 4. Apply results back to VisState
    ELKBridge.elkToVisState(elkResult, visState);
  }

  /**
   * Build ELK graph from VisualizationState data (pure transformation)
   */
  private static buildELKGraph(
    nodes: any[], 
    containers: any[], 
    edges: any[],
    layoutConfig: LayoutConfig
  ): ElkGraph {
    const elkNodes: ElkNode[] = [];
    
    // Find root containers (containers with no parent container)
    const rootContainers = containers.filter(container => {
      // Check if this container has a parent that's also a container
      const hasContainerParent = containers.some(otherContainer => 
        otherContainer.children && otherContainer.children.has(container.id)
      );
      return !hasContainerParent;
    });
    
    // Build hierarchy for each root container
    rootContainers.forEach(container => {
      const hierarchyNode = ELKBridge.buildContainerHierarchy(container, containers, nodes, layoutConfig);
      elkNodes.push(hierarchyNode);
    });
    
    // Add top-level nodes (not in any container)
    const collapsedContainerIds = new Set(containers.filter(c => c.collapsed).map(c => c.id));
    const topLevelNodes = nodes.filter(node => 
      !ELKBridge.isNodeInAnyContainer(node.id, containers) && 
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
      layoutOptions: getELKLayoutOptions(layoutConfig.algorithm, nodes.length)
    };
  }

  /**
   * Build container hierarchy (pure transformation)
   */
  private static buildContainerHierarchy(
    container: any, 
    allContainers: any[], 
    allNodes: any[], 
    layoutConfig: LayoutConfig
  ): ElkNode {
    // Find child nodes
    const childNodes = allNodes.filter(node => container.children.has(node.id));
    
    // Find child containers
    const childContainers = allContainers.filter(childContainer => 
      container.children.has(childContainer.id)
    );
    
    // Create ELK children array
    const elkChildren: ElkNode[] = [
      // Add child nodes
      ...childNodes.map(node => ({
        id: node.id,
        width: node.width || 180,
        height: node.height || 60
      })),
      // Add child containers (recursively)
      ...childContainers.map(childContainer => 
        ELKBridge.buildContainerHierarchy(childContainer, allContainers, allNodes, layoutConfig)
      )
    ];
    
    return {
      id: container.id,
      width: container.width || 200,
      height: container.height || 150,
      children: elkChildren,
      layoutOptions: getELKLayoutOptions(layoutConfig.algorithm, allNodes.length)
    };
  }

  /**
   * Validate ELK input (format validation only - no business logic)
   */
  private static validateELKInput(elkGraph: ElkGraph): void {
    if (!elkGraph.children) {
      elkGraph.children = [];
    }
    
    if (!elkGraph.edges) {
      elkGraph.edges = [];
    }
    
    // Validate node format
    elkGraph.children.forEach(node => {
      if (!node.id) {
        throw new Error(`ELK node missing ID: ${JSON.stringify(node)}`);
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
      if (!edge.id) {
        throw new Error(`ELK edge missing ID: ${JSON.stringify(edge)}`);
      }
      if (!edge.sources || edge.sources.length === 0) {
        throw new Error(`ELK edge missing sources: ${edge.id}`);
      }
      if (!edge.targets || edge.targets.length === 0) {
        throw new Error(`ELK edge missing targets: ${edge.id}`);
      }
    });
  }

  /**
   * Update container from ELK result (pure transformation)
   */
  private static updateContainerFromELK(elkNode: ElkNode, visState: VisualizationState): void {
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
    
    // Update child positions recursively
    elkNode.children?.forEach(elkChildNode => {
      if (elkChildNode.children && elkChildNode.children.length > 0) {
        ELKBridge.updateContainerFromELK(elkChildNode, visState);
      } else {
        ELKBridge.updateNodeFromELK(elkChildNode, visState);
      }
    });
  }

  /**
   * Update node from ELK result (pure transformation)
   */
  private static updateNodeFromELK(elkNode: ElkNode, visState: VisualizationState): void {
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
        // If not found as node, try as container
        try {
          visState.setContainerLayout(elkNode.id, layoutUpdates);
        } catch (containerError) {
          console.warn(`[ELKBridge] Node/Container ${elkNode.id} not found in VisState`);
        }
      }
    }
  }

  /**
   * Update edge from ELK result (pure transformation)
   */
  private static updateEdgeFromELK(elkEdge: ElkEdge, visState: VisualizationState): void {
    if (elkEdge.sections && elkEdge.sections.length > 0) {
      try {
        visState.setEdgeLayout(elkEdge.id, { sections: elkEdge.sections });
      } catch (error) {
        // Edge no longer exists in VisState (probably filtered out as hyperedge)
        console.warn(`[ELKBridge] Skipping layout update for edge ${elkEdge.id} - edge no longer exists`);
      }
    }
  }

  // Helper methods
  private static isNodeInAnyContainer(nodeId: string, containers: any[]): boolean {
    return containers.some(container => container.children && container.children.has(nodeId));
  }
}