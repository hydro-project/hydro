/**
 * @fileoverview ELK Bridge - Clean interface between VisState and ELK
 * 
 * This bridge implements the core architectural principle:
 * - VisState contains ALL data (nodes, edges, containers) 
 * - ELK gets visible elements only through visibleEdges (hyperedges included transparently)
 * - ELK returns layout positions that get applied back to VisState
 */

import { VisualizationState } from '../core/VisState';
import type { 
  GraphNode, 
  GraphEdge, 
  Container,
  HyperEdge
} from '../shared/types';
import type { LayoutConfig } from '../core/types';
import { getELKLayoutOptions } from '../shared/config';

import ELK from 'elkjs';
import type { ElkGraph, ElkNode, ElkEdge } from './elk-types';

export class ELKBridge {
  private elk: any; // ELK instance
  private layoutConfig: LayoutConfig;

  constructor(layoutConfig: LayoutConfig = {}) {
    this.elk = new ELK(); // ✅ Create fresh ELK instance for each ELKBridge
    this.layoutConfig = { algorithm: 'mrtree', ...layoutConfig };
  }

  /**
   * Update layout configuration
   */
  updateLayoutConfig(config: LayoutConfig): void {
    this.layoutConfig = { ...this.layoutConfig, ...config };
  }

  /**
   * Convert VisState to ELK format and run layout
   * Key insight: Include ALL visible edges (regular + hyper) with no distinction
   */
  async layoutVisState(visState: VisualizationState): Promise<void> {
    // Clear any existing edge layout data to ensure ReactFlow starts fresh
    visState.visibleEdges.forEach(edge => {
      try {
        visState.setEdgeLayout(edge.id, { sections: [] });
      } catch (error) {
        // Edge might not exist anymore, ignore
      }
    });
    
    // 1. Extract all visible data from VisState
    const elkGraph = this.visStateToELK(visState);
        
    // 2. Validate ELK input data
    this.validateELKInput(elkGraph);
    
    // 3. Yield control to browser to show loading state
    await new Promise(resolve => setTimeout(resolve, 10));
        
    // Debug: Check for data leaks in large graphs
    if ((elkGraph.children?.length || 0) > 10) {
      // CRITICAL: Check if we're accidentally including children of collapsed containers
      const leaks: string[] = [];
      for (const container of (elkGraph.children || [])) {
        // FIXED: Only check for leaks if container is marked as collapsed
        // Expanded containers (collapsed=false) are SUPPOSED to have children!
        // Check the original container state from visState
        const originalContainer = visState.getContainer(container.id);
        if (originalContainer?.collapsed && container.children && container.children.length > 0) {
          const leakMsg = `Container ${container.id} has ${container.children.length} children but should be collapsed!`;
          console.warn(`[ELKBridge] ⚠️  LEAK: ${leakMsg}`);
          console.warn(`[ELKBridge] ⚠️    Children: ${container.children.map(c => c.id).slice(0, 3).join(', ')}${container.children.length > 3 ? '...' : ''}`);
          leaks.push(leakMsg);
        }
      }
      
      // In test environments, throw an error if we have leaks
      if (leaks.length > 0 && (process.env.NODE_ENV === 'test' || process.env.VITEST === 'true')) {
        throw new Error(`ELK CONTAINER LEAKS DETECTED: ${leaks.length} collapsed containers have visible children. This violates the collapsed container invariant. Leaks: ${leaks.slice(0, 3).join('; ')}`);
      }
    }
    
    const elkResult = await this.elk.layout(elkGraph);
    
    // Debug: Log ELK output to compare with our working standalone test
    const elkOutputContainers = (elkResult.children || []);
    
    // Calculate actual spacing from ELK results
    const sortedByX = elkOutputContainers
      .filter(c => c.x !== undefined)
      .sort((a, b) => (a.x || 0) - (b.x || 0));
    
    if (sortedByX.length > 1) {
      const gaps = [];
      for (let i = 1; i < sortedByX.length; i++) {
        const gap = (sortedByX[i].x || 0) - ((sortedByX[i-1].x || 0) + (sortedByX[i-1].width || 0));
        gaps.push(gap);
      }
      const avgGap = gaps.reduce((a, b) => a + b, 0) / gaps.length;
    }
    
    // 5. Yield control again before applying results
    await new Promise(resolve => setTimeout(resolve, 10));
    
    // 6. Apply results back to VisState
    this.elkToVisState(elkResult, visState);
  }

  /**
   * Log ELK graph structure for debugging layout issues
   */
  private logELKGraphStructure(elkGraph: ElkGraph): void {
    console.log('[ELKBridge] 🔍 ELK Input Graph Structure:');
    console.log(`[ELKBridge] 📊 Root: ${elkGraph.children?.length || 0} children`);
    
    // Log container positions if they exist (this might be the issue)
    const containersWithPositions = (elkGraph.children || []).filter(child => 
      child.x !== undefined || child.y !== undefined
    );
    
    if (containersWithPositions.length > 0) {
      console.log(`[ELKBridge] ⚠️  Found ${containersWithPositions.length} containers with existing positions:`);
      for (const container of containersWithPositions) { // Log ALL containers with positions!
        console.log(`[ELKBridge] 📍 ${container.id}: position=(${container.x}, ${container.y}), size=${container.width}x${container.height}`);
      }
    } else {
      console.log('[ELKBridge] ✅ No existing positions in ELK input (good for fresh layout)');
    }
    
    // Log ALL container dimensions to see if there are inconsistencies
    const containers = (elkGraph.children || []);
    console.log(`[ELKBridge] 📦 All container dimensions:`);
    for (const container of containers) {
      console.log(`[ELKBridge] 📦 ${container.id}: ${container.width}x${container.height}`);
    }
    
    // CRITICAL: Log the exact layout options being sent
    console.log(`[ELKBridge] ⚙️  Layout options:`, JSON.stringify(elkGraph.layoutOptions, null, 2));
  }

  /**
   * Validate ELK input data to prevent null reference errors
   * NOTE: This should only validate format, not apply business rules
   */
  private validateELKInput(elkGraph: ElkGraph): void {
    // Ensure children array exists
    if (!elkGraph.children) {
      elkGraph.children = [];
    }
    
    // Ensure edges array exists
    if (!elkGraph.edges) {
      elkGraph.edges = [];
    }
    
    // Validate each node has required properties (VisualizationState should ensure this)
    elkGraph.children.forEach(node => {
      if (!node.id) {
        throw new Error(`ELK node missing ID: ${JSON.stringify(node)}`);
      }
      if (typeof node.width !== 'number' || node.width <= 0) {
        throw new Error(`ELK node ${node.id} has invalid width: ${node.width}. VisualizationState should provide valid dimensions.`);
      }
      if (typeof node.height !== 'number' || node.height <= 0) {
        throw new Error(`ELK node ${node.id} has invalid height: ${node.height}. VisualizationState should provide valid dimensions.`);
      }
      
      // Validate children if this is a container
      if (node.children) {
        node.children.forEach(child => {
          if (!child.id) {
            throw new Error(`ELK child node missing ID: ${JSON.stringify(child)}`);
          }
          if (typeof child.width !== 'number' || child.width <= 0) {
            throw new Error(`ELK child node ${child.id} has invalid width: ${child.width}. VisualizationState should provide valid dimensions.`);
          }
          if (typeof child.height !== 'number' || child.height <= 0) {
            throw new Error(`ELK child node ${child.id} has invalid height: ${child.height}. VisualizationState should provide valid dimensions.`);
          }
        });
      }
    });
    
    // Get all valid node IDs from the ELK graph for edge validation
    const allValidNodeIds = new Set<string>();
    const collectNodeIds = (elkNode: ElkNode) => {
      allValidNodeIds.add(elkNode.id);
      elkNode.children?.forEach(collectNodeIds);
    };
    elkGraph.children?.forEach(collectNodeIds);
    
    // STRICT VALIDATION: Throw error for edges with invalid endpoints instead of silently filtering
    // This forces VisualizationState to provide clean, valid data
    elkGraph.edges?.forEach(edge => {
      const hasValidSource = edge.sources?.some(sourceId => allValidNodeIds.has(sourceId));
      const hasValidTarget = edge.targets?.some(targetId => allValidNodeIds.has(targetId));
      
      if (!hasValidSource || !hasValidTarget) {
        const sourceIds = edge.sources?.join(', ') || 'none';
        const targetIds = edge.targets?.join(', ') || 'none';
        const availableNodes = Array.from(allValidNodeIds).slice(0, 10).join(', ') + (allValidNodeIds.size > 10 ? '...' : '');
        
        throw new Error(
          `ELKBridge received edge ${edge.id} with invalid endpoints!\n` +
          `Sources: [${sourceIds}] (valid: ${hasValidSource})\n` +
          `Targets: [${targetIds}] (valid: ${hasValidTarget})\n` +
          `Available nodes: ${availableNodes}\n` +
          `This indicates a bug in VisualizationState - it should not send edges that reference non-existent nodes.`
        );
      }
    });
    
    // Validate each remaining edge has required properties
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
   * Convert VisState to ELK format
   * HIERARCHICAL: Use proper ELK hierarchy to match ReactFlow parent-child relationships
   */
  private visStateToELK(visState: VisualizationState): ElkGraph {    
    // HIERARCHICAL: Build container nodes with their children
    const rootNodes: ElkNode[] = [];
    const processedNodes = new Set<string>();
    
    // Process containers first
    visState.visibleContainers.forEach(container => {
      const containerNode: ElkNode = {
        id: container.id,
        width: container.width || 200,
        height: container.height || 150,
        children: []
      };
      
      // Add child nodes to this container
      if (!container.collapsed && container.children) {
        container.children.forEach(childId => {
          const childNode = visState.visibleNodes.find(n => n.id === childId);
          if (childNode) {
            containerNode.children!.push({
              id: childNode.id,
              width: childNode.width || 180,
              height: childNode.height || 60
            });
            processedNodes.add(childId);
          }
        });
      }
      
      rootNodes.push(containerNode);
    });
    
    // Add any uncontained nodes at root level
    visState.visibleNodes.forEach(node => {
      if (!processedNodes.has(node.id)) {
        rootNodes.push({
          id: node.id,
          width: node.width || 180,
          height: node.height || 60
        });
      }
    });
    
    // Convert edges - ELK will handle hierarchy automatically
    const allEdges: ElkEdge[] = Array.from(visState.visibleEdges).map(edge => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target]
    }));
    
    return {
      id: 'root',
      children: rootNodes,
      edges: allEdges,
      layoutOptions: getELKLayoutOptions(this.layoutConfig.algorithm)
    };
  }

  /**
   * Extract visible nodes (both GraphNodes and collapsed containers as nodes)
   * NOTE: VisualizationState should handle all business logic for node conversion
   */
  private extractVisibleNodes(visState: VisualizationState): GraphNode[] {
    const nodes: GraphNode[] = [];
    
    // Add visible regular nodes using the correct VisState API
    const visibleNodes = visState.visibleNodes;
    
    // SAFETY CHECK: Verify that visibleNodes excludes hidden nodes
    const hiddenNodesInVisible = visibleNodes.filter(n => n.hidden === true);
    if (hiddenNodesInVisible.length > 0) {
      console.error(`[ELKBridge] 🚨 CRITICAL: Found ${hiddenNodesInVisible.length} hidden nodes in visibleNodes:`, 
        hiddenNodesInVisible.map(n => n.id));
      throw new Error(`ELKBridge received hidden nodes from VisState.visibleNodes - this violates the hiding contract`);
    }
    
    nodes.push(...visibleNodes);
    
    // Add collapsed containers as nodes - VisualizationState should provide these pre-converted
    // TODO: Move this logic to VisualizationState.getCollapsedContainersAsNodes()
    const visibleContainers = visState.visibleContainers;
    visibleContainers.forEach(container => {
      if (container.collapsed) {
        // BUSINESS LOGIC VIOLATION: This conversion should be in VisualizationState
        const containerAsNode: GraphNode = {
          id: container.id,
          label: container.id,
          width: container.width || 200,  // VisualizationState should guarantee valid dimensions
          height: container.height || 150, // VisualizationState should guarantee valid dimensions
          x: container.x || 0,            
          y: container.y || 0,            
          hidden: false,
          style: 'default' // VisualizationState should determine style
        };
        
        // SAFETY CHECK: Ensure collapsed container is not hidden
        if (container.hidden) {
          console.error(`[ELKBridge] 🚨 CRITICAL: Collapsed container ${container.id} is hidden but was included in visibleContainers`);
          throw new Error(`ELKBridge contract violation: hidden collapsed container ${container.id} should not be converted to node`);
        }
                
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
    
    // CRITICAL ARCHITECTURAL FIX: Use visibleContainers (includes collapsed) not expandedContainers (excludes collapsed)
    // RULE: Bridges should only see the public visible interface, not internal state like expandedContainers
    // According to our rules: collapsed containers should appear in ELK, hidden containers should not
    const visibleContainers = visState.visibleContainers;
    
    // SAFETY CHECK: Verify that visibleContainers excludes hidden containers
    // This is a defensive check to ensure our dependency on VisState's filtering is correct
    const hiddenContainersInVisible = visibleContainers.filter(c => c.hidden === true);
    if (hiddenContainersInVisible.length > 0) {
      console.error(`[ELKBridge] 🚨 CRITICAL: Found ${hiddenContainersInVisible.length} hidden containers in visibleContainers:`, 
        hiddenContainersInVisible.map(c => c.id));
      throw new Error(`ELKBridge received hidden containers from VisState.visibleContainers - this violates the hiding contract`);
    }
    
    // Convert computed views back to raw Container objects
    for (const computedContainer of visibleContainers) {
      const rawContainer = visState.getContainer(computedContainer.id);
      if (rawContainer) {
        // Double-check: ensure the raw container is also not hidden
        if (rawContainer.hidden) {
          console.error(`[ELKBridge] 🚨 CRITICAL: Raw container ${rawContainer.id} is hidden but was in visibleContainers`);
          throw new Error(`ELKBridge contract violation: hidden container ${rawContainer.id} should not be in visibleContainers`);
        }
        containers.push(rawContainer);
      }
    }
    
    return containers;
  }

  /**
   * Build ELK graph from extracted data
   */
  private buildELKGraph(
    nodes: GraphNode[], 
    containers: Container[], 
    edges: GraphEdge[],
    visState: VisualizationState
  ): ElkGraph {
    // Build hierarchy: create nested ELK structure
    const elkNodes: ElkNode[] = [];
    
    // Find root containers (containers with no parent container)
    const rootContainers = containers.filter(container => {
      // Check if this container has a parent that's also a container
      const hasContainerParent = containers.some(otherContainer => 
        otherContainer.children && otherContainer.children.has(container.id)
      );
      return !hasContainerParent;
    });
    
    // Recursively build ELK hierarchy starting from root containers
    const buildContainerHierarchy = (container: Container): ElkNode => {
      // Find child nodes (regular nodes)
      const childNodes = nodes.filter(node => container.children.has(node.id));
      
      // Find child containers (nested containers)
      const childContainers = containers.filter(childContainer => 
        container.children.has(childContainer.id)
      );
      
      // Create ELK children array with both nodes and nested containers
      const elkChildren: ElkNode[] = [
        // Add child nodes
        ...childNodes.map(node => ({
          id: node.id,
          width: node.width || 180,
          height: node.height || 60
        })),
        // Add child containers (recursively)
        ...childContainers.map(childContainer => buildContainerHierarchy(childContainer))
      ];
      
      // Use layout dimensions if available (e.g., from collapsed state), otherwise use defaults
      // IMPORTANT: Use VisualizationState API to get proper dimensions (handles collapsed containers)
      const effectiveDimensions = visState.getContainerAdjustedDimensions(container.id);
      const containerWidth = effectiveDimensions.width;
      const containerHeight = effectiveDimensions.height;
      
      return {
        id: container.id,
        width: containerWidth,
        height: containerHeight,
        children: elkChildren,
        layoutOptions: getELKLayoutOptions(this.layoutConfig.algorithm)
      };
    };
    
    // Build hierarchy for each root container
    rootContainers.forEach(container => {
      const hierarchyNode = buildContainerHierarchy(container);
      elkNodes.push(hierarchyNode);
    });
    
    // Add top-level nodes (not in any container, excluding collapsed containers that were already added as nodes)
    // TODO: VisualizationState should provide getTopLevelNodes() method
    const collapsedContainerIds = new Set(visState.visibleContainers.filter(c => c.collapsed).map(c => c.id));
    const topLevelNodes = nodes.filter(node => 
      !this.isNodeInAnyContainer(node.id, containers) && 
      !collapsedContainerIds.has(node.id)
    );
    
    topLevelNodes.forEach(node => {
      // VisualizationState should guarantee these nodes have valid dimensions
      if (!node.width || !node.height) {
        throw new Error(`Top-level node ${node.id} missing dimensions. VisualizationState should provide valid dimensions.`);
      }
      
      elkNodes.push({
        id: node.id,
        width: node.width,
        height: node.height
      });
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
      layoutOptions: getELKLayoutOptions(this.layoutConfig.algorithm)
    };
  }

  /**
   * Apply ELK results back to VisState
   */
  private elkToVisState(elkResult: ElkGraph, visState: VisualizationState): void {
    if (!elkResult.children) {
      console.warn('[ELKBridge] ⚠️ No children in ELK result');
      return;
    }
    
    // SIMPLIFIED: Use ELK coordinates directly following ReactFlow best practices
    // No custom offset corrections - ELK provides the correct coordinate system
    
    // Apply positions to containers and nodes using ELK coordinates directly
    elkResult.children.forEach(elkNode => {
      // Check if this ID exists as a container in VisState first
      try {
        const container = visState.getContainer(elkNode.id);
        if (container) {
          this.updateContainerFromELK(elkNode, visState);
          return;
        }
      } catch (e) {
        // Not a container, continue to node logic
      }
      
      // Handle as node or container based on ELK structure
      if (elkNode.children && elkNode.children.length > 0) {
        this.updateContainerFromELK(elkNode, visState);
      } else {
        this.updateNodeFromELK(elkNode, visState);
      }
    });
    
    // SIMPLIFIED: No edge routing processing - let ReactFlow handle all edge positioning
    // ReactFlow will automatically route edges between nodes using handles
  }
  
  // REMOVED: applyOffsetToChildren - dead code in canonical flat pattern
  
  // REMOVED: updateEdgeFromELK - ReactFlow handles all edge routing automatically

  /**
   * Update container dimensions and child positions from ELK result
   */
  private updateContainerFromELK(elkNode: ElkNode, visState: VisualizationState): void {
    // Use VisState's proper layout methods instead of direct property access
    const layoutUpdates: any = {};
    
    if (elkNode.x !== undefined || elkNode.y !== undefined) {
      layoutUpdates.position = {};
      if (elkNode.x !== undefined) layoutUpdates.position.x = elkNode.x;
      if (elkNode.y !== undefined) layoutUpdates.position.y = elkNode.y;
    }
    
    if (elkNode.width !== undefined || elkNode.height !== undefined) {
      layoutUpdates.dimensions = {};
      
      // Bridge is just a format translator - pass through ELK dimensions as-is
      if (elkNode.width !== undefined) layoutUpdates.dimensions.width = elkNode.width;
      if (elkNode.height !== undefined) layoutUpdates.dimensions.height = elkNode.height;
    }
    
    if (Object.keys(layoutUpdates).length > 0) {
      visState.setContainerLayout(elkNode.id, layoutUpdates);
    }
    
    // Update child positions (recursively handle containers vs nodes)
    elkNode.children?.forEach(elkChildNode => {
      if (elkChildNode.children && elkChildNode.children.length > 0) {
        // This child is also a container - recurse into it
        this.updateContainerFromELK(elkChildNode, visState);
      } else {
        // This child is a leaf node - update its position
        this.updateNodeFromELK(elkChildNode, visState);
      }
    });
  }

  /**
   * Update node position from ELK result
   */
  private updateNodeFromELK(elkNode: ElkNode, visState: VisualizationState): void {
    // Try to update as regular node first using VisState's layout methods
    try {
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
        visState.setNodeLayout(elkNode.id, layoutUpdates);
      }
      return;
    } catch (nodeError) {
      // If not found as node, might be a collapsed container
      try {
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
        return;
      } catch (containerError) {
        console.warn(`[ELKBridge] Node/Container ${elkNode.id} not found in VisState`);
      }
    }
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
