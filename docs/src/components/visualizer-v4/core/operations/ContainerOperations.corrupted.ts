/**
 * Container Operations - Collapse/Expand Logic
 * 
 * Handles all container state transitions including collapse/expand operations,
 * hyperEdge management, and visibility cascading. Extracted from VisState.ts
 * for better separation of concerns.
 */

import { LAYOUT_CONSTANTS, HYPEREDGE_CONSTANTS, SIZES } from '../../shared/config';
import { VisualizationState } from '../VisualizationState';
import { GraphNode, Container, GraphEdge, HyperEdge, Edge, isGraphEdge } from '../types';
import { createHyperEdge } from '../EdgeFactory';

export class ContainerOperations {
  private readonly state: any;
  private isExpanding: boolean = false;

  constructor(state: any) {
    this.state = state;
  }

  /**
   * Handle container collapse with hyperEdge management
   */
  handleContainerCollapse(containerId: string): void {
    const children = this.state.getContainerChildren(containerId) || new Set();
    
    // Step 1: Recursively collapse any expanded child containers first
    for (const childId of children) {
      const container = this.state.getContainer(childId);
      if (container && !this.state.getContainerCollapsed(childId)) {
        // Child container is expanded, collapse it first
        this.handleContainerCollapse(childId);
      }
    }
    
    // Step 2: Delete all GraphEdges adjacent to children
    this.deleteAdjacentEdges(children);
    
    // Step 3: Create hyperEdges based on external connections
    this.createHyperEdgesForCollapsedContainer(containerId);
    
    // Step 4: Mark container as collapsed
    this.state.setContainerCollapsed(containerId, true);
  }

  /**
   * Find all GraphEdges that are adjacent to the given children
   */
  private findEdgesAdjacentToChildren(children: Set<string>): Set<string> {
    const edgesToDelete = new Set<string>();
    
    // Look through all visible GraphEdges
    for (const edge of this.state.visibleEdges) {
      if (isGraphEdge(edge)) {
        // If either source or target is in the children set, this edge should be deleted
        if (children.has(edge.source) || children.has(edge.target)) {
          edgesToDelete.add(edge.id);
        }
      }
    }
    
    return edgesToDelete;
  }

  /**
   * Use CoveredEdgesIndex to determine what hyperEdges we need for this container
   */
  private createHyperEdgesFromIndex(containerId: string): HyperEdge[] {
    const hyperEdges: HyperEdge[] = [];
    
    // Get all edges that will be covered by this container
    const coveredEdges = this.state.getAggregatedEdges(containerId);
    
    // Group covered edges by their external endpoints
    const edgeGroups = new Map<string, GraphEdge[]>();
    
    for (const edge of coveredEdges) {
      // Determine which endpoint is external to the container
      const children = this.state.getContainerChildren(containerId) || new Set();
      const sourceInContainer = this.isNodeInContainerRecursive(edge.source, containerId);
      const targetInContainer = this.isNodeInContainerRecursive(edge.target, containerId);
      
      // Skip edges that are entirely internal to the container
      if (sourceInContainer && targetInContainer) {
        continue;
      }
      
      // Determine the external endpoint
      const externalEndpoint = sourceInContainer ? edge.target : edge.source;
      const isOutgoing = sourceInContainer;
      
      // Find the visible ancestor of the external endpoint
      const visibleExternalEndpoint = this.findLowestVisibleAncestor(externalEndpoint);
      
      // Skip self-referencing edges (container to itself)
      if (visibleExternalEndpoint === containerId) {
        continue;
      }
      
      // Group by direction: container->external or external->container
      const groupKey = isOutgoing ? `${containerId}->${visibleExternalEndpoint}` : `${visibleExternalEndpoint}->${containerId}`;
      
      if (!edgeGroups.has(groupKey)) {
        edgeGroups.set(groupKey, []);
      }
      edgeGroups.get(groupKey)!.push(edge);
    }
    
    // Create hyperEdges for each group
    for (const [groupKey, edges] of edgeGroups.entries()) {
      const [source, target] = groupKey.split('->');
      const hyperEdgeId = `hyper_${source}_to_${target}`;
      
      const hyperEdge = createHyperEdge({
        id: hyperEdgeId,
        source,
        target,
        style: 'default'
      });
      
      hyperEdges.push(hyperEdge);
    }
    
    return hyperEdges;
  }

  /**
   * Check if a node is recursively contained within a container
   */
  private isNodeInContainerRecursive(nodeId: string, containerId: string): boolean {
    const directParent = this.state.getNodeContainer(nodeId);
    if (!directParent) return false;
    if (directParent === containerId) return true;
    
    // Check recursively up the hierarchy
    return this.isNodeInContainerRecursive(directParent, containerId);
  }

  /**
   * Handle container expansion with hyperEdge management
   */
  /**
   * Delete all GraphEdges that are adjacent to the given nodes/containers
   */
  private deleteAdjacentEdges(nodeIds: Set<string>): void {
    const edgesToDelete: string[] = [];
    
    // Find all GraphEdges adjacent to the nodes being collapsed
    for (const edge of this.state.visibleEdges) {
      if (!isHyperEdge(edge)) {
        const graphEdge = edge as GraphEdge;
        if (nodeIds.has(graphEdge.source) || nodeIds.has(graphEdge.target)) {
          edgesToDelete.push(graphEdge.id);
        }
      }
    }
    
    // Delete the GraphEdges
    for (const edgeId of edgesToDelete) {
      this.state.removeGraphEdge(edgeId);
    }
  }

  /**
   * Create hyperEdges for a collapsed container based on its external connections
   */
  private createHyperEdgesForCollapsedContainer(containerId: string): void {
    // Get all edges that would be covered by this container
    const coveredEdges = this.state.getAggregatedEdges(containerId);
    
    // Group external connections by endpoint
    const externalConnections = new Map<string, { incoming: GraphEdge[], outgoing: GraphEdge[] }>();
    
    for (const edge of coveredEdges) {
      const children = this.state.getContainerChildren(containerId) || new Set();
      const sourceInContainer = children.has(edge.source);
      const targetInContainer = children.has(edge.target);
      
      // Skip internal edges (both endpoints in container)
      if (sourceInContainer && targetInContainer) {
        continue;
      }
      
      // Determine external endpoint
      const externalEndpoint = sourceInContainer ? edge.target : edge.source;
      const isOutgoing = sourceInContainer;
      
      if (!externalConnections.has(externalEndpoint)) {
        externalConnections.set(externalEndpoint, { incoming: [], outgoing: [] });
      }
      
      const group = externalConnections.get(externalEndpoint)!;
      if (isOutgoing) {
        group.outgoing.push(edge);
      } else {
        group.incoming.push(edge);
      }
    }
    
    // Create hyperEdges for each external connection
    for (const [externalEndpoint, group] of externalConnections) {
      // Skip self-referencing connections
      if (externalEndpoint === containerId) {
        continue;
      }
      
      // Create incoming hyperEdge (external -> container)
      if (group.incoming.length > 0) {
        const hyperEdgeId = `hyper_${externalEndpoint}_to_${containerId}`;
        const hyperEdge = createHyperEdge({
          id: hyperEdgeId,
          source: externalEndpoint,
          target: containerId
        });
        this.state.addHyperEdge(hyperEdge.id, hyperEdge);
      }
      
      // Create outgoing hyperEdge (container -> external)
      if (group.outgoing.length > 0) {
        const hyperEdgeId = `hyper_${containerId}_to_${externalEndpoint}`;
        const hyperEdge = createHyperEdge({
          id: hyperEdgeId,
          source: containerId,
          target: externalEndpoint
        });
        this.state.addHyperEdge(hyperEdge.id, hyperEdge);
      }
    }
  }

  handleContainerExpansion(containerId: string): void {
    // 1. Find all hyperEdges connected to this container and remove them
    const hyperEdgeIds = this.state._collections.nodeToEdges.get(containerId) || new Set();
    const hyperEdgesToRemove: string[] = [];
    
    for (const hyperEdgeId of hyperEdgeIds) {
      const hyperEdge: HyperEdge | undefined = this.state.getHyperEdge(hyperEdgeId);
      if (hyperEdge && (hyperEdge.source === containerId || hyperEdge.target === containerId)) {
        hyperEdgesToRemove.push(hyperEdgeId);
      }
    }

    // 2. Remove the hyperEdges (they are no longer needed)
    for (const hyperEdgeId of hyperEdgesToRemove) {
      this.state.removeHyperEdge(hyperEdgeId);
    }

    // 3. Mark container as expanded
    this.state.setContainerCollapsed(containerId, false);

    // 4. Restore visibility of GraphEdges using CoveredEdgesIndex
    const coveredEdges = this.state.getAggregatedEdges(containerId);
    for (const edge of coveredEdges) {
      // Check if both endpoints are now visible
      const sourceVisible = this.isNodeOrContainerVisible(edge.source);
      const targetVisible = this.isNodeOrContainerVisible(edge.target);
      if (sourceVisible && targetVisible) {
        this.state.setEdgeVisibility(edge.id, true);
      }
    }
  }
    
    // Find all hyperEdges that have this container as source or target
    for (const [hyperEdgeId, hyperEdge] of this.state._collections.hyperEdges) {
      if (hyperEdge.source === hiddenContainerId || hyperEdge.target === hiddenContainerId) {
        hyperEdgesToRemove.push(hyperEdgeId);
      }
    }
    
    // Remove all found hyperEdges
    for (const hyperEdgeId of hyperEdgesToRemove) {
      this.state.removeHyperEdge(hyperEdgeId);
    }
    
    // In the new architecture, we don't return orphaned edges
    // The CoveredEdgesIndex will handle aggregated edge computation
    return [];
  }

  /**
   * Prepare hyperEdges for a collapsed container
   */
  private prepareHyperedges(containerId: string, crossingEdges: GraphEdge[]): HyperEdge[] {
    const children = this.state.getContainerChildren(containerId) || new Set();
    const edgeGroups = new Map<string, { incoming: GraphEdge[], outgoing: GraphEdge[] }>();

        // Process crossing edges
    this.processEdgesForHyperEdgeGrouping(crossingEdges, children, edgeGroups, containerId);

    // Create hyperedge objects
    const hyperEdges: HyperEdge[] = [];
    
    for (const [externalEndpoint, group] of Array.from(edgeGroups.entries())) {
      // Validate that the external endpoint exists
      const endpointExists = this.state._collections.graphNodes.has(externalEndpoint) || 
                           this.state._collections.containers.has(externalEndpoint);
      
      if (!endpointExists) {
        throw new Error(`[HYPEREDGE] Cannot create hyperEdge - external endpoint ${externalEndpoint} does not exist`);
      }
      
      // NOTE: Temporarily skip visibility validation to debug other issues
      // const endpointVisible = this.isNodeOrContainerVisible(externalEndpoint);
      // if (!endpointVisible) {
      //   throw new Error(`[HYPEREDGE] Cannot create hyperEdge - external endpoint ${externalEndpoint} is not visible`);
      // }
      
      // Create hyperedge for incoming connections (external -> container)
      if (group.incoming.length > 0) {
        const hyperEdgeId = `${HYPEREDGE_CONSTANTS.PREFIX}${externalEndpoint}${HYPEREDGE_CONSTANTS.SEPARATOR}${containerId}`;
        hyperEdges.push(this.createHyperedgeObject(hyperEdgeId, externalEndpoint, containerId, group.incoming, containerId));
      }

      // Create hyperedge for outgoing connections (container -> external)
      if (group.outgoing.length > 0) {
        const hyperEdgeId = `${HYPEREDGE_CONSTANTS.PREFIX}${containerId}${HYPEREDGE_CONSTANTS.SEPARATOR}${externalEndpoint}`;
        hyperEdges.push(this.createHyperedgeObject(hyperEdgeId, containerId, externalEndpoint, group.outgoing, containerId));
      }
    }

    return hyperEdges;
  }

  /**
   * Helper method to process edges and group them by external endpoint
   */
  private processEdgesForHyperEdgeGrouping(
    edges: Edge[] | GraphEdge[], 
    children: Set<string>, 
    edgeGroups: Map<string, { incoming: GraphEdge[], outgoing: GraphEdge[] }>,
    containerId: string
  ): void {
    // Group edges by external endpoint 
    for (const edge of edges) {
      const sourceInContainer = children.has(edge.source);
      
      // Get the external endpoint (the one NOT in the container being collapsed)
      const rawExternalEndpoint = sourceInContainer ? edge.target : edge.source;
      
      // Get the visible representation of the external endpoint
      const externalEndpoint = this.findLowestVisibleAncestor(rawExternalEndpoint);
      
      // CRITICAL FIX: Prevent self-referencing hyperEdges
      // If the external endpoint resolves to the same container we're collapsing,
      // skip this edge to avoid self-references
      if (externalEndpoint === containerId) {
        continue;
      }
            
      const isOutgoing = sourceInContainer; // container -> external

      if (!edgeGroups.has(externalEndpoint)) {
        edgeGroups.set(externalEndpoint, { incoming: [], outgoing: [] });
      }

      const group = edgeGroups.get(externalEndpoint)!;
      if (!isHyperEdge(edge)) {
        if (isOutgoing) {
          group.outgoing.push(edge);
        } else {
          group.incoming.push(edge);
        }
      } else {
        // Handle hyperedge case: since we removed aggregatedEdges, 
        // we can't process hyperEdges in this method anymore.
        // This should be handled differently in the new architecture.
        console.warn(`⚠️ HyperEdge ${edge.id} encountered in processEdgesForHyperEdgeGrouping - this should not happen in the new architecture`);
      }
    }
  }

  /**
   * Create a hyperEdge object from original edges
   * In the new architecture, hyperEdges are simple connection representations
   * without embedded aggregated edge data
   */
  private createHyperedgeObject(hyperEdgeId: string, source: string, target: string, originalEdges: GraphEdge[], collapsedContainerId: string): HyperEdge {
    // Use the highest priority style from the original edges
    const style = this.hyperEdgeStyles(originalEdges);

    return createHyperEdge({
      id: hyperEdgeId,
      source,
      target,
      style,
      hidden: false
    });
  }

  /**
   * Aggregate styles from multiple edges (highest priority wins)
   */
  private hyperEdgeStyles(edges: Edge[]): string {
    // Priority order: ERROR > WARNING > THICK > HIGHLIGHTED > DEFAULT
    const stylePriority: Record<string, number> = {
      'error': 5,
      'warning': 4,
      'thick': 3,
      'highlighted': 2,
      'default': 1
    };
    
    let highestPriority = 0;
    let resultStyle = 'default';
    
    for (const edge of edges) {
      const priority = stylePriority[edge.style as string] || 1;
      if (priority > highestPriority) {
        highestPriority = priority;
        resultStyle = edge.style as string;
      }
    }
    
    return resultStyle;
  }

  private isNodeOrContainerVisible(entityId: string): boolean {
    if (this.state._collections._visibleNodes.has(entityId)) return true;
    if (this.state._collections._visibleContainers.has(entityId)) return true;
    return false;
  }

  private removeFromNodeToEdges(nodeId: string, edgeId: string): void {
    const edges = this.state._collections.nodeToEdges.get(nodeId);
    if (edges) {
      edges.delete(edgeId);
      if (edges.size === 0) {
        this.state._collections.nodeToEdges.delete(nodeId);
      }
    }
  }

  /**
   * Get crossing edges for a container
   * Core operation needed for container collapse/expand functionality
   */
  getCrossingEdges(containerId: string): GraphEdge[] {
    const children = this.state.getContainerChildren(containerId) || new Set();
    const crossingEdges: GraphEdge[] = [];

    // Check all visible GraphEdges adjacent to containerId's children
    // Note: We only process GraphEdges here, not HyperEdges, since HyperEdges 
    // represent already-collapsed containers and should be handled separately
    for (const childId of children) {
      const childEdges = this.state._collections.nodeToEdges.get(childId) || new Set();
      for (const edgeId of childEdges) {
        // Only get GraphEdges - HyperEdges are handled separately
        const edge = this.state.getGraphEdge(edgeId);
        if (!edge) continue;
        
        const sourceInContainer = children.has(edge.source);
        const targetInContainer = children.has(edge.target);

        // Edge crosses boundary if exactly one endpoint is in container
        if (sourceInContainer !== targetInContainer) {
          crossingEdges.push(edge);
        }
      }
    }
    return crossingEdges;
  }

  // Helper methods
  private findLowestVisibleAncestor(entityId: string): string {
    // First check if the entity itself is visible
    if (this.isNodeOrContainerVisible(entityId)) {
      return entityId;
    }

    // If it's a node, find its visible ancestor container
    const nodeContainer = this.state.getNodeContainer(entityId);
    if (nodeContainer) {
      // Recursively find the lowest visible ancestor of the container
      return this.findLowestVisibleAncestor(nodeContainer);
    }

    // If it's a container, find its visible ancestor
    const container = this.state.getContainer(entityId);
    if (container && container.parentId) {
      return this.findLowestVisibleAncestor(container.parentId);
    }

    // If no visible ancestor found, return the entity itself
    // This shouldn't happen in well-formed data, but prevents infinite loops
    return entityId;
  }

  private isAncestorRecursion(ancestorId: string, entityId: string, originalId: string): boolean {

    // It's a match, return true!
    if (entityId === ancestorId) {
      return true;
    }

    const container = this.state.getNodeContainer(entityId);
    const parent = container ?? this.state.getContainer(entityId)?.parentId;
    if (parent) {
        // Recursively find the lowest visible ancestor of the container
        return this.isAncestorRecursion(ancestorId, parent, originalId);
    } else {
      return false;
    }
  }

  private isAncestor(ancestorId: string, entityId: string): boolean {
    return this.isAncestorRecursion(ancestorId, entityId, entityId);
  }

  validateHyperEdgeLifting(): void {
    // Check that hyperEdges only exist between valid, visible endpoints that ELK can process
    const invalidHyperEdges: string[] = [];
    
    for (const [hyperEdgeId, hyperEdge] of this.state._collections.hyperEdges) {
      // Check if both endpoints exist and are visible to ELK
      const sourceNodeExists = this.state._collections._visibleNodes.has(hyperEdge.source);
      const sourceContainerExists = this.state._collections._visibleContainers.has(hyperEdge.source);
      const targetNodeExists = this.state._collections._visibleNodes.has(hyperEdge.target);
      const targetContainerExists = this.state._collections._visibleContainers.has(hyperEdge.target);

      const sourceExists = sourceNodeExists || sourceContainerExists;
      const targetExists = targetNodeExists || targetContainerExists;

      // HyperEdges are valid if:
      // 1. Both endpoints exist and are visible to ELK
      // 2. At least one endpoint is a collapsed container
      const hasCollapsedContainer = sourceContainerExists || targetContainerExists;
      
      if (!sourceExists || !targetExists || !hasCollapsedContainer) {
        invalidHyperEdges.push(hyperEdgeId);
      }
    }
    
    // Note: This method should only validate, not mutate
    // If invalid hyperEdges are found, they should be reported but not removed here
    if (invalidHyperEdges.length > 0) {
      console.warn(`[validateHyperEdgeLifting] Found ${invalidHyperEdges.length} invalid hyperEdges: ${invalidHyperEdges.join(', ')}`);
    }
  }
}



function handleContainerExpansionRecursive(containerId: string, string: any) {
  throw new Error('Function not implemented.');
}

/**
 * Check if an edge is a hyperEdge
 * @deprecated Use the type guard from types.ts instead
 */
export function isHyperEdge(edge: any): edge is HyperEdge {
    return (edge && typeof edge === 'object' && 'aggregatedEdges' in edge);
}
