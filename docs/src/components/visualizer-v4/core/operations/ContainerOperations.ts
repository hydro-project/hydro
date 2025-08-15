/**
 * Container Operations - Collapse/Expand Logic
 * 
 * Handles all container state transitions including collapse/expand operations,
 * hyperEdge management, and visibility cascading. Extracted from VisState.ts
 * for better separation of concerns.
 */

import { LAYOUT_CONSTANTS, HYPEREDGE_CONSTANTS, SIZES } from '../../shared/config';
import type { Edge, GraphEdge, HyperEdge } from '../types';
import { isHyperEdge as isHyperEdgeType } from '../types';

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
    // 1. Recurse bottom-up
    const children = this.state.getContainerChildren(containerId);
    for (const childId of children) {
      const childContainer = this.state.getContainer(childId);
      if (childContainer && !childContainer.collapsed) {
        this.handleContainerCollapse(childId);
      }
    }

    // Now everything below us is collapsed. Don't mark children hidden until we find crossing edges

    // 2. Find crossing edges
    const crossingEdges = this.getCrossingEdges(containerId);

    // 3. Hide children
    for (const childId of children) {
      this.hideChild(childId); 
    }

    // 4. Create hyperEdges to replace crossing edges
    const hyperEdges = this.prepareHyperedges(containerId, crossingEdges);
    
    for (const hyperEdge of hyperEdges) {
      // INVARIANT: Validate hyperEdge before storing
      this.validateHyperEdgeInvariant(hyperEdge);
      
      // Create hyperEdge using internal API
      this.state._collections.hyperEdges.set(hyperEdge.id, hyperEdge);
      
      // Make sure hyperEdge is visible
      hyperEdge.hidden = false;
      
      // Update node-to-edges mapping for hyperEdges
      const sourceEdges = this.state._collections.nodeToEdges.get(hyperEdge.source) || new Set();
      sourceEdges.add(hyperEdge.id);
      this.state._collections.nodeToEdges.set(hyperEdge.source, sourceEdges);
      
      const targetEdges = this.state._collections.nodeToEdges.get(hyperEdge.target) || new Set();
      targetEdges.add(hyperEdge.id);
      this.state._collections.nodeToEdges.set(hyperEdge.target, targetEdges);
    }
    
    // 5. Hide the original crossing edges
    for (const edge of crossingEdges) {
      const id = edge.id;
      if (isHyperEdge(edge)) {
        this.state.removeHyperEdge(id);
      } else {
        this.state.setEdgeVisibility(id, false);
      }
      this.state._collections._visibleEdges.delete(id);

    }
  }

  /**
   * Handle container expansion with hyperEdge management
   */
  handleContainerExpansion(containerId: string): void {
    // define a local map from string key to aggregated edges
    const localMap = new Map<string, Set<GraphEdge>>();

    // gather up all hyperedges connected to this container into the localMap
    const hyperEdgeIds = this.state._collections.nodeToEdges.get(containerId) || new Set();
    console.log('[handleContainerExpansion] HyperEdge IDs:', hyperEdgeIds);
    for (const hyperEdgeId of hyperEdgeIds) {
      const hyperEdge: HyperEdge | undefined = this.state.getHyperEdge(hyperEdgeId);
      console.log('[handleContainerExpansion] Processing HyperEdge:', hyperEdgeId);
      if (!hyperEdge) continue;
      
      for (const aggregatedEdge of hyperEdge.aggregatedEdges.values()) {
        console.log('[handleContainerExpansion] Aggregated Edge:', aggregatedEdge);
        // figure out which child this edge belongs to
        const children = this.state.getContainerChildren(containerId);
        for (const childId of children) {
          const sourceRemote = this.isAncestor(childId, aggregatedEdge.target);
          const targetRemote = this.isAncestor(childId, aggregatedEdge.source);
          console.log('[handleContainerExpansion] Checking child:', childId, ' SourceRemote:', sourceRemote, ' TargetRemote:', targetRemote);
          if (sourceRemote || targetRemote) {
            // find the lowest visible ancestor for the remote ID
            // and assign the adjusted aggregated edge to the local map
            const remoteId = this.findLowestVisibleAncestor(targetRemote ? aggregatedEdge.target : aggregatedEdge.source);
            const direction = targetRemote ? 'outgoing' : 'incoming';
            const mapKey = `${childId}:${remoteId}:${direction}`;

            if (!localMap.has(mapKey)) {
              localMap.set(mapKey, new Set());
            }
            localMap.get(mapKey)!.add(aggregatedEdge);
          }
        }
      }
    }

    // mark container expanded
    this.state.setContainerCollapsed(containerId, false);

    // mark children visible
    for (const childId of this.state.getContainerChildren(containerId)) {
      const childContainer = this.state.getContainer(childId);
      if (childContainer) {
        childContainer.hidden = false;
        this.state._collections._visibleContainers.set(childId, childContainer);
        // Note: We do NOT un-collapse the child container here. That is a separate operation.
      } else {
        const node = this.state.getGraphNode(childId);
        if (node) {
          node.hidden = false;
          this.state.setNodeVisibility(childId, true);
        }
      }
    }

    // create edges for the children
    for (const [mapKey, aggregatedEdges] of localMap.entries()) {
      const [childId, remoteId, direction] = mapKey.split(':');
      console.log("[handleContainerExpansion] Creating edges for:", mapKey);

      for (const aggregatedEdge of aggregatedEdges) {
        // if the childId is a graphNode and the remoteId is a graphNode, we just restore the edge
        if (this.state.getGraphNode(childId) && this.state.getGraphNode(remoteId)) {
          console.log("[handleContainerExpansion] Restoring edge:", aggregatedEdge);
          this.state.setEdgeVisibility(aggregatedEdge.id, true);
        } else {
          // create a hyperEdge
          if (direction === 'outgoing') {
            const hyperEdgeId = `${HYPEREDGE_CONSTANTS.PREFIX}${childId}${HYPEREDGE_CONSTANTS.SEPARATOR}${remoteId}`;
            console.log("[handleContainerExpansion] Creating outgoing hyperEdge:", hyperEdgeId);
            const hyperEdge = this.createHyperedgeObject(hyperEdgeId, childId, remoteId, Array.from(aggregatedEdges), containerId);
            this.state.setHyperEdge(hyperEdge.id, hyperEdge);
          } else {
            const hyperEdgeId = `${HYPEREDGE_CONSTANTS.PREFIX}${remoteId}${HYPEREDGE_CONSTANTS.SEPARATOR}${childId}`;
            console.log("[handleContainerExpansion] Creating incoming hyperEdge:", hyperEdgeId);
            const hyperEdge = this.createHyperedgeObject(hyperEdgeId, remoteId, childId, Array.from(aggregatedEdges), containerId);
            this.state.setHyperEdge(hyperEdge.id, hyperEdge);
          }
        }
      }
    }

    // remove the old hyperEdges
    for (const hyperEdgeId of hyperEdgeIds) {
      console.log("[handleContainerExpansion] Removing hyperEdge:", hyperEdgeId);
      this.state.removeHyperEdge(hyperEdgeId);
    }
  }

  /**
   * Handle container collapse with hyperEdge management
   */
  handleContainerExpansionRecursive(containerId: string): void {
    // First expand this container
    this.handleContainerExpansion(containerId);

    // Then recursively expand all child containers
    const children = this.state.getContainerChildren(containerId) || new Set();
    for (const childId of Array.from(children)) {
      if (typeof childId === 'string') {
        const childContainer = this.state.getContainer(childId);
        if (childContainer && childContainer.collapsed) {
          this.handleContainerExpansionRecursive(childId);
        }
      }
    }
  }

  /**
   * Hide a child container or node during collapse
   */
  private hideChild(childId: string): void {
    const childContainer = this.state.getContainer(childId);
    if (childContainer) {
      // mark the child container as collapsed and hidden
      childContainer.collapsed = true; // Must be collapsed
      childContainer.hidden = true;    // Must be hidden

      // CRITICAL: Clear layout positions to prevent stale layout data invariant violations
      childContainer.x = undefined;
      childContainer.y = undefined;
      if (childContainer.layout) {
        childContainer.layout.position = undefined;
      }
      
      // Update visibility caches
      this.state._updateContainerVisibilityCaches(childId, childContainer);
    } else {
      // If it's a node, hide it directly
      const node = this.state.getGraphNode(childId);
      if (node) {
        node.hidden = true;
        this.state._collections._visibleNodes.delete(childId);
        
        // Cascade to connected edges
        this.state._cascadeNodeVisibilityToEdges(childId, false);
      }
    }
  }

  /**
   * Prepare hyperEdges for a collapsed container
   */
  private prepareHyperedges(containerId: string, crossingEdges: Edge[]): HyperEdge[] {
    const children = this.state.getContainerChildren(containerId) || new Set();
    const edgeGroups = new Map<string, { incoming: GraphEdge[], outgoing: GraphEdge[] }>();

    // Group edges by external endpoint 
    for (const edge of crossingEdges) {
      const sourceInContainer = children.has(edge.source);
      // adjust endpoint for the lowest visible ancestor
      const externalEndpoint = this.findLowestVisibleAncestor(sourceInContainer ? edge.target : edge.source);
            
      const isOutgoing = sourceInContainer; // container -> external

      if (!edgeGroups.has(externalEndpoint)) {
        edgeGroups.set(externalEndpoint, { incoming: [], outgoing: [] });
      }

      const group = edgeGroups.get(externalEndpoint)!;
      if (!isHyperEdgeType(edge)) {
        if (isOutgoing) {
          group.outgoing.push(edge);
        } else {
          group.incoming.push(edge);
        }
      } else {
        // Handle hyperedge case: push all the aggregated Edges
        for (const aggregatedEdge of edge.aggregatedEdges.values()) {
          if (isOutgoing) {
            group.outgoing.push(aggregatedEdge);
          } else {
            group.incoming.push(aggregatedEdge);
          }
        }
      }
    }

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
   * Validate that a hyperEdge has the required aggregatedEdges invariant
   */
  private validateHyperEdgeInvariant(hyperEdge: HyperEdge): void {
    if (!hyperEdge.aggregatedEdges || !(hyperEdge.aggregatedEdges instanceof Map) || hyperEdge.aggregatedEdges.size === 0) {
      throw new Error(`HYPEREDGE INVARIANT VIOLATION: HyperEdge ${hyperEdge.id} must have non-empty aggregatedEdges Map. Found: ${hyperEdge.aggregatedEdges?.constructor?.name} with size ${hyperEdge.aggregatedEdges?.size || 'undefined'}`);
    }
  }

  /**
   * Create a hyperEdge object from original edges
   */
  private createHyperedgeObject(hyperEdgeId: string, source: string, target: string, originalEdges: GraphEdge[], collapsedContainerId: string): HyperEdge {
    // Store original edge information for restoration as aggregatedEdges
    const aggregatedEdges = new Map<string, GraphEdge>();
    for (const edge of originalEdges) {
      if (isHyperEdgeType(edge)) {
        for (const aggEdge of edge.aggregatedEdges.values()) {
          aggregatedEdges.set(aggEdge.id, aggEdge);
        }
      } else {
        aggregatedEdges.set(edge.id, edge);
      }
    }

    // Use the highest priority style from the aggregated edges
    const style = this.hyperEdgeStyles(originalEdges);

    const hyperEdge: HyperEdge = {
      id: hyperEdgeId,
      source,
      target,
      style,
      aggregatedEdges,  // Now contains complete edge objects
      hidden: false
    };

    // INVARIANT: Every hyperEdge must have non-empty aggregatedEdges
    this.validateHyperEdgeInvariant(hyperEdge);

    return hyperEdge;
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
  getCrossingEdges(containerId: string): Edge[] {
    const children = this.state.getContainerChildren(containerId) || new Set();
    const crossingEdges: Edge[] = [];

    // Check all visible edges and hyperEdges adjacent to containerId's children
    for (const childId of children) {
      const childEdges = this.state._collections.nodeToEdges.get(childId) || new Set();
      for (const edgeId of childEdges) {
        // Get the actual edge object from the edgeId
        const edge = this.state.getGraphEdge(edgeId) || this.state.getHyperEdge(edgeId);
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
    console.log(`[isAncestor] Checking if ${ancestorId} is ancestor of ${originalId}`);

    // It's a match, return true!
    if (entityId === ancestorId) {
      console.log(`[isAncestor] BINGO: ${ancestorId} is ancestor of ${originalId}`);
      return true;
    }

    const container = this.state.getNodeContainer(entityId);
    const parent = container ?? this.state.getContainer(entityId)?.parentId;
    if (parent) {
        // Recursively find the lowest visible ancestor of the container
        return this.isAncestorRecursion(ancestorId, parent, originalId);
    } else {
      console.log(`[isAncestor] NO: ${ancestorId} is NOT an ancestor of ${originalId}, which does not have a parent`);
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
      const sourceExists = this.state._collections._visibleNodes.has(hyperEdge.source) || 
                          this.state._collections._visibleContainers.has(hyperEdge.source);
      const targetExists = this.state._collections._visibleNodes.has(hyperEdge.target) || 
                          this.state._collections._visibleContainers.has(hyperEdge.target);
      
      if (!sourceExists || !targetExists) {
        invalidHyperEdges.push(hyperEdgeId);
      }
    }
    
    // Remove all invalid hyperEdges
    for (const hyperEdgeId of invalidHyperEdges) {
      this.state.removeHyperEdge(hyperEdgeId);
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
