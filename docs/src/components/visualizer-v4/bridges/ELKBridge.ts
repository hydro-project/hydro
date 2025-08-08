/**
 * @fileoverview ELK Bridge - Clean interface between VisState and ELK
 * 
 * This bridge implements the core architectural principle:
 * - VisState contains ALL data (nodes, edges, containers) 
 * - ELK gets visible elements only through visibleEdges (hyperedges included transparently)
 * - ELK returns layout positions that get applied back to VisState
 */

import { VisualizationState } from '../core/VisState';
import { ContainerPadding } from '../core/ContainerPadding';
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
    this.layoutConfig = { algorithm: 'layered', ...layoutConfig };
    console.log(`[ELKBridge] 🆕 Created fresh ELK instance: ${this.elk.constructor.name} (${Date.now()})`);
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
    // console.log('[ELKBridge] 🚀 Starting ELK layout from VisState');
    
    // 1. Extract all visible data from VisState
    const elkGraph = this.visStateToELK(visState);
    
    // 2. Log ELK input for debugging spacing issues
    this.logELKGraphStructure(elkGraph);
    
    // 3. Validate ELK input data
    this.validateELKInput(elkGraph);
    
    // 3. Yield control to browser to show loading state
    await new Promise(resolve => setTimeout(resolve, 10));
    
    // 4. Run ELK layout (this is the potentially blocking operation)
    // console.log((`[ELKBridge] 📊 Sending to ELK children count:`, elkGraph.children?.length));
    // console.log(`[ELKBridge] 📊 ELK Graph structure:`, {
    //   id: elkGraph.id,
    //   children: elkGraph.children?.length,
    //   edges: elkGraph.edges?.length,
    //   firstFewChildren: elkGraph.children?.slice(0, 3).map(c => ({ id: c.id, width: c.width, height: c.height })),
    //   firstFewEdges: elkGraph.edges?.slice(0, 3).map(e => ({ id: e.id, sources: e.sources, targets: e.targets }))
    // });
    //   id: elkGraph.id,
    //   childrenCount: elkGraph.children?.length,
    //   childrenIds: elkGraph.children?.map(c => c.id),
    //   edgesCount: elkGraph.edges?.length
    // });
    
    // console.log(('[ELKBridge] ⏳ Running ELK layout - this may take a moment for large graphs...'));
    
    // Debug: Log sample of input structure for large graphs
    if ((elkGraph.children?.length || 0) > 10) {
      console.log('[ELKBridge] 🔍 Large graph detected, logging input structure...');
      console.log(`[ELKBridge] 📊 Total containers: ${elkGraph.children?.length || 0}`);
      console.log(`[ELKBridge] 📊 Total edges: ${elkGraph.edges?.length || 0}`);
      
      // CRITICAL: Check if we're accidentally including children of collapsed containers
      console.log('[ELKBridge] 🔍 Checking for children of collapsed containers...');
      for (const container of (elkGraph.children || [])) {
        if (container.children && container.children.length > 0) {
          console.log(`[ELKBridge] ⚠️  LEAK: Container ${container.id} has ${container.children.length} children but should be collapsed!`);
          console.log(`[ELKBridge] ⚠️    Children: ${container.children.map(c => c.id).slice(0, 3).join(', ')}${container.children.length > 3 ? '...' : ''}`);
        }
      }
      
      // Log sample container dimensions
      const sampleContainers = (elkGraph.children || []).slice(0, 5);
      console.log('[ELKBridge] 📦 Sample container dimensions:');
      for (const container of sampleContainers) {
        console.log(`[ELKBridge] 📦   ${container.id}: ${container.width}x${container.height}${container.x !== undefined ? ` pos=(${container.x},${container.y})` : ''}${container.children ? ` children=${container.children.length}` : ''}`);
      }
    }
    
    const elkResult = await this.elk.layout(elkGraph);
    
    // Debug: Log sample of output for large graphs
    if ((elkResult.children?.length || 0) > 10) {
      console.log('[ELKBridge] 📊 ELK Output for large graph:');
      const sampleOutput = (elkResult.children || []).slice(0, 5);
      for (const container of sampleOutput) {
        console.log(`[ELKBridge] 📍   ${container.id}: pos=(${container.x},${container.y}) size=${container.width}x${container.height}`);
      }
      
      // Check for suspiciously large coordinates
      const largeCoords = (elkResult.children || []).filter(c => (c.y || 0) > 5000);
      if (largeCoords.length > 0) {
        console.log(`[ELKBridge] ⚠️  WARNING: ${largeCoords.length} containers have Y > 5000:`);
        for (const container of largeCoords.slice(0, 3)) {
          console.log(`[ELKBridge] ⚠️    ${container.id}: Y=${container.y}`);
        }
      }
    }
    // console.log(('[ELKBridge] ✅ ELK layout complete'));
    
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
      for (const container of containersWithPositions.slice(0, 3)) { // Log first 3
        console.log(`[ELKBridge] 📍 ${container.id}: position=(${container.x}, ${container.y}), size=${container.width}x${container.height}`);
      }
    } else {
      console.log('[ELKBridge] ✅ No existing positions in ELK input (good for fresh layout)');
    }
    
    // Log a sample of container dimensions
    const containers = (elkGraph.children || []).slice(0, 3);
    for (const container of containers) {
      console.log(`[ELKBridge] 📦 ${container.id}: ${container.width}x${container.height}`);
    }
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
    
    // Validate each edge has required properties
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
   */
  private visStateToELK(visState: VisualizationState): ElkGraph {
    // Extract visible nodes (both regular nodes and collapsed containers)
    const visibleNodes = this.extractVisibleNodes(visState);
    
    // Extract container hierarchy for visible containers
    const visibleContainers = this.extractVisibleContainers(visState);
    
    // Extract ALL edges via the unified visibleEdges interface
    // This now includes hyperedges transparently - ELK doesn't need to know the difference
    const allEdges = visState.visibleEdges;
    
    // console.log('[ELKBridge] 📋 Extracted from VisState:', {
    //   visibleNodes: visibleNodes.length,
    //   visibleContainers: visibleContainers.length,
    //   totalEdges: allEdges.length,
    //   regularEdges: allEdges.length,
    //   hyperEdges: 0
    // });
    
    return this.buildELKGraph(visibleNodes, visibleContainers, allEdges, visState);
  }

  /**
   * Extract visible nodes (both GraphNodes and collapsed containers as nodes)
   * NOTE: VisualizationState should handle all business logic for node conversion
   */
  private extractVisibleNodes(visState: VisualizationState): GraphNode[] {
    const nodes: GraphNode[] = [];
    
    // Add visible regular nodes using the correct VisState API
    const visibleNodes = visState.visibleNodes;
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
   * Build ELK graph from extracted data
   */
  private buildELKGraph(
    nodes: GraphNode[], 
    containers: Container[], 
    edges: GraphEdge[],
    visState: VisualizationState
  ): ElkGraph {
    // // console.log(((`[ELKBridge] 🔨 Building ELK graph with ${nodes.length} nodes, ${containers.length} containers, ${edges.length} edges`)));
    // // console.log(((`[ELKBridge] 🔍 Available nodes:`, nodes.map(n => n.id))));
    // // console.log(((`[ELKBridge] 🔍 Available containers:`, containers.map(c => ({ id: c.id, children: Array.from(c.children), collapsed: c.collapsed })))));
    
    // Build hierarchy: create nested ELK structure
    const elkNodes: ElkNode[] = [];
    
    // Find root containers (containers with no parent container)
    const rootContainers = containers.filter(container => {
      // Check if this container has a parent that's also a container
      const hasContainerParent = containers.some(otherContainer => 
        otherContainer.children.has(container.id)
      );
      return !hasContainerParent;
    });
    
    // // console.log(((`[ELKBridge] 🔍 Found ${rootContainers.length} root containers:`, rootContainers.map(c => c.id))));
    
    // Recursively build ELK hierarchy starting from root containers
    const buildContainerHierarchy = (container: Container): ElkNode => {
      // Find child nodes (regular nodes)
      const childNodes = nodes.filter(node => container.children.has(node.id));
      
      // Find child containers (nested containers)
      const childContainers = containers.filter(childContainer => 
        container.children.has(childContainer.id)
      );
      
      // console.log(`[ELKBridge] 🔍 Container ${container.id} has ${childNodes.length} nodes and ${childContainers.length} containers:`, {
      //   nodes: childNodes.map(n => n.id),
      //   containers: childContainers.map(c => c.id),
      //   allChildren: Array.from(container.children)
      // });
      
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
      
      console.log(`[ELKBridge] 📐 Container ${container.id} dimensions: ${containerWidth}x${containerHeight} (collapsed: ${container.collapsed})`);

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
      // // console.log(((`[ELKBridge] 🏗️ Built hierarchy for ${container.id}:`, JSON.stringify(hierarchyNode, null, 2))));
      elkNodes.push(hierarchyNode);
    });
    
    // // console.log(((`[ELKBridge] 🔍 Final elkNodes array length: ${elkNodes.length}`)));
    
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
    
    // console.log((`[ELKBridge] � Processing ${elkEdges.length} valid edges from VisState`));
    
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
    // // console.log((('[ELKBridge] 📝 Applying ELK results back to VisState')));
    // // console.log((('[ELKBridge] 🔍 ELK Result Structure:', JSON.stringify(elkResult, null, 2))));
    
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
    
    // Apply edge routing information
    // // console.log((('--- ELKBRIDGE_EDGE_PROCESSING_START ---')));
    const allEdges = elkResult.edges || [];
    if (allEdges.length > 0) {
      // // console.log(((`[ELKBridge] 🔍 Processing ${allEdges.length} edges for sections`)));
      allEdges.forEach(elkEdge => {
        this.updateEdgeFromELK(elkEdge, visState);
      });
    } else {
      // // console.log((('[ELKBridge] ⚠️ No edges array in ELK result')));
    }
    // // console.log((('--- ELKBRIDGE_EDGE_PROCESSING_END ---')));
    
    // // console.log((('[ELKBridge] ✅ Applied all ELK results to VisState')));
  }
  
  /**
   * Update edge routing information from ELK result
   */
  private updateEdgeFromELK(elkEdge: ElkEdge, visState: VisualizationState): void {
    // Use VisState's proper layout method instead of direct property access
    if (elkEdge.sections && elkEdge.sections.length > 0) {
      // // console.log(((`[ELKBridge] 🔧 About to set layout for edge ${elkEdge.id} with ${elkEdge.sections.length} sections`)));
      
      // Check if the edge still exists in VisState before trying to update it
      try {
        visState.setEdgeLayout(elkEdge.id, { sections: elkEdge.sections });
        // // console.log(((`[ELKBridge] 📍 Updated edge ${elkEdge.id} with ${elkEdge.sections.length} sections`)));
        
        // Debug: Try to read back the edge to see if it was set
        const edge = visState.getGraphEdge(elkEdge.id);
        // // console.log(((`[ELKBridge] 🔍 Debug: Edge ${elkEdge.id} layout after update:`, edge?.layout)));
      } catch (error) {
        // Edge no longer exists in VisState (probably filtered out as hyperedge)
        // console.log((`[ELKBridge] ⚠️ Skipping layout update for edge ${elkEdge.id} - edge no longer exists in VisState:`, error.message));
      }
    } else {
      // // console.log(((`[ELKBridge] 📍 Edge ${elkEdge.id} has no sections (cross-container edge)`)));
    }
  }

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
      
      // Apply container padding logic to ELK results
      const elkWidth = elkNode.width || 0;
      const elkHeight = elkNode.height || 0;
      const container = visState.getContainer(elkNode.id);
      const adjustedDims = ContainerPadding.adjustPostELKDimensions(elkWidth, elkHeight, container?.collapsed || false);
      
      layoutUpdates.dimensions.width = adjustedDims.width;
      layoutUpdates.dimensions.height = adjustedDims.height;
    }
    
    if (Object.keys(layoutUpdates).length > 0) {
      visState.setContainerLayout(elkNode.id, layoutUpdates);
      // // console.log(((`[ELKBridge] 📏 Updated container ${elkNode.id} layout: ${JSON.stringify(layoutUpdates)}`)));
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
    // // console.log(((`[ELKBridge] 🔧 Attempting to update node ${elkNode.id} with ELK coords (${elkNode.x}, ${elkNode.y})`)));
    
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
        // // console.log(((`[ELKBridge] 📏 Calling setNodeLayout for ${elkNode.id} with:`, layoutUpdates)));
        visState.setNodeLayout(elkNode.id, layoutUpdates);
        // // console.log(((`[ELKBridge] ✅ Successfully updated node ${elkNode.id}`)));
      }
      return;
    } catch (nodeError) {
      // // console.log(((`[ELKBridge] ⚠️ Node ${elkNode.id} not found as regular node, trying as container:`, nodeError.message)));
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
