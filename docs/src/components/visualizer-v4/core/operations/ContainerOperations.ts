/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * Container Operations - Collapse/Expand Logic
 * 
 * Handles all container state transitions including collapse/expand operations,
 * hyperEdge management, and visibility cascading. Extracted from VisState.ts
 * for better separation of concerns.
 */

import { VisualizationState } from '../VisualizationState';
import { GraphNode, Container, GraphEdge, HyperEdge, Edge, isGraphEdge, isHyperEdge } from '../types';
import { createHyperEdge } from '../EdgeFactory';
import { HYPEREDGE_CONSTANTS } from '../../shared/config';

export class ContainerOperations {
  private readonly state: any; // Use any to access private methods

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
    
    // Step 2: Hide all child nodes and containers, and collapse child containers
    for (const childId of children) {
      const container = this.state.getContainer(childId);
      const node = this.state.getGraphNode(childId);
      
      if (container) {
        // Child containers must be both collapsed and hidden when parent is collapsed
        this.state.setContainerState(childId, { collapsed: true, hidden: true });
        // Clean up invalid hyperEdges immediately after hiding each container
        this.cleanupInvalidHyperEdges();
      } else if (node) {
        this.state.setNodeVisibility(childId, false);
      }
    }
    
    // Step 3: Clean up any hyperEdges that are no longer valid due to visibility changes
    // Do this immediately after hiding children to prevent validation errors
    this.cleanupInvalidHyperEdges();
    
    // Step 4: Delete all GraphEdges adjacent to children
    this.deleteAdjacentEdges(new Set(children));
    
    // Step 5: Create hyperEdges based on external connections
    this.createHyperEdgesForCollapsedContainer(containerId);
    
    // Step 6: Mark container as collapsed
    this.state.setContainerCollapsed(containerId, true);
  }

  /**
   * Handle container expansion by removing hyperEdges and restoring GraphEdge visibility
   */
  handleContainerExpansion(containerId: string): void {
    // 1. Find all hyperEdges connected to this container, prep to remove
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

    // 4. Show all child nodes and expanded child containers
    const children = this.state.getContainerChildren(containerId) || new Set();
    for (const childId of children) {
      const container = this.state.getContainer(childId);
      const node = this.state.getGraphNode(childId);
      
      if (container) {
        // For child containers, show them and respect their collapsed state
        this.state.setContainerState(childId, { hidden: false });
      } else if (node) {
        // Show child nodes
        this.state.setNodeVisibility(childId, true);
      }
    }

    // 5. Restore visibility of GraphEdges using CoveredEdgesIndex
    const coveredEdgeIds = this.state.getAggregatedEdges(containerId);
    for (const edgeId of coveredEdgeIds) {
      const edge = this.state.getGraphEdge(edgeId);
      if (!edge) continue;
      
      // Check if both endpoints are now visible
      const sourceVisible = this.isNodeOrContainerVisible(edge.source);
      const targetVisible = this.isNodeOrContainerVisible(edge.target);
      if (sourceVisible && targetVisible) {
        this.state.setEdgeVisibility(edge.id, true);
      }
    }

    // 6. Create hyperEdges for any child containers that should remain collapsed
    for (const childId of children) {
      const childContainer = this.state.getContainer(childId);
      if (childContainer && this.state.getContainerCollapsed(childId)) {
        // Child container should remain collapsed, so we need to create its hyperEdges
        this.createHyperEdgesForCollapsedContainer(childId);
      }
    }
    
    // 7. Other collapsed containers may need new hyperEdges to this container's nodes
  this.recreateHyperEdgesForCollapsedNeighbors(new Set(children));
  }

  /**
   * Delete all GraphEdges that are adjacent to the given nodes
   */
  private deleteAdjacentEdges(nodeIds: Set<string>): void {
    const edgesToDelete: string[] = [];
    
    for (const edge of this.state.visibleEdges) {
      if (isGraphEdge(edge)) {
        if (nodeIds.has(edge.source) || nodeIds.has(edge.target)) {
          edgesToDelete.push(edge.id);
        }
      }
    }
    
    for (const edgeId of edgesToDelete) {
      this.state.removeGraphEdge(edgeId);
    }
  }

  /**
   * Get crossing edges for a container (public API)
   */
  getCrossingEdges(containerId: string): GraphEdge[] {
    return this.findCrossingEdges(containerId);
  }

  /**
   * Find all edges that cross the boundary of a container
   * (one endpoint inside the container, one outside)
   */
  private findCrossingEdges(containerId: string): GraphEdge[] {
    const crossingEdges: GraphEdge[] = [];
    
    // Check all edges in the state (access via private collections)
    for (const [edgeId, edge] of this.state._collections.graphEdges.entries()) {
      const sourceInContainer = this.isNodeInContainerRecursive(edge.source, containerId);
      const targetInContainer = this.isNodeInContainerRecursive(edge.target, containerId);
      
      // Edge crosses container boundary if exactly one endpoint is inside
      if (sourceInContainer !== targetInContainer) {
        crossingEdges.push(edge);
      }
    }
    
    return crossingEdges;
  }

  /**
   * Create hyperEdges for a collapsed container by finding crossing edges
   */
  private createHyperEdgesForCollapsedContainer(containerId: string): void {
    // Ensure the container is actually collapsed before creating hyperEdges
    const container = this.state.getContainer(containerId);
    if (!container || !container.collapsed) {
      return; // Only create hyperEdges for collapsed containers
    }
    
    // Find all edges that cross the container boundary
    const crossingEdges = this.findCrossingEdges(containerId);
    
    if (crossingEdges.length === 0) {
      return;
    }
    
    // Group external connections by endpoint
    const externalConnections = new Map<string, { incoming: GraphEdge[], outgoing: GraphEdge[] }>();
    
    for (const edge of crossingEdges) {
      const sourceInContainer = this.isNodeInContainerRecursive(edge.source, containerId);
      const targetInContainer = this.isNodeInContainerRecursive(edge.target, containerId);
      
      // Determine external endpoint
      const externalEndpoint = sourceInContainer ? edge.target : edge.source;
      const isOutgoing = sourceInContainer;
      
      // Find the visible representation of the external endpoint
      const visibleExternalEndpoint = this.findLowestVisibleAncestor(externalEndpoint);
      
      // Skip self-referencing connections
      if (visibleExternalEndpoint === containerId) {
        continue;
      }
      
      if (!externalConnections.has(visibleExternalEndpoint)) {
        externalConnections.set(visibleExternalEndpoint, { incoming: [], outgoing: [] });
      }
      
      const group = externalConnections.get(visibleExternalEndpoint)!;
      if (isOutgoing) {
        group.outgoing.push(edge);
      } else {
        group.incoming.push(edge);
      }
    }
    
    // Create hyperEdges for each external connection
    for (const [externalEndpoint, group] of externalConnections) {
      // Validate that both endpoints are visible before creating hyperEdges
      const containerVisible = this.isNodeOrContainerVisible(containerId);
      const externalVisible = this.isNodeOrContainerVisible(externalEndpoint);
      
      if (!containerVisible || !externalVisible) {
        continue; // Skip creating hyperEdges with invalid endpoints
      }
      
      // Create incoming hyperEdge (external -> container)
      if (group.incoming.length > 0) {
  const hyperEdgeId = `${HYPEREDGE_CONSTANTS.PREFIX}${externalEndpoint}${HYPEREDGE_CONSTANTS.SEPARATOR}${containerId}`;
        const aggregatedEdges = new Map<string, GraphEdge>();
        for (const edge of group.incoming) {
          aggregatedEdges.set(edge.id, edge);
        }
        
        const hyperEdge = createHyperEdge({
          id: hyperEdgeId,
          source: externalEndpoint,
          target: containerId,
          aggregatedEdges: aggregatedEdges
        });
        this.state.setHyperEdge(hyperEdge.id, hyperEdge);
      }
      
      // Create outgoing hyperEdge (container -> external)
      if (group.outgoing.length > 0) {
  const hyperEdgeId = `${HYPEREDGE_CONSTANTS.PREFIX}${containerId}${HYPEREDGE_CONSTANTS.SEPARATOR}${externalEndpoint}`;
        const aggregatedEdges = new Map<string, GraphEdge>();
        for (const edge of group.outgoing) {
          aggregatedEdges.set(edge.id, edge);
        }
        
        const hyperEdge = createHyperEdge({
          id: hyperEdgeId,
          source: containerId,
          target: externalEndpoint,
          aggregatedEdges: aggregatedEdges
        });
        this.state.setHyperEdge(hyperEdge.id, hyperEdge);
      }
    }
  }

  /**
   * Check if a node is contained within a container recursively
   */
  private isNodeInContainerRecursive(nodeId: string, containerId: string): boolean {
    const directParent = this.state.getNodeContainer(nodeId);
    if (!directParent) return false;
    if (directParent === containerId) return true;
    
    // Check if the direct parent container is itself contained in the target container
    return this.isNodeInContainerRecursive(directParent, containerId);
  }

  /**
   * Find the lowest visible ancestor of a given entity (node or container)
   */
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

  /**
   * Check if a node or container is currently visible to ELK
   */
  private isNodeOrContainerVisible(entityId: string): boolean {
    return this.state._collections._visibleNodes.has(entityId) || this.state._collections._visibleContainers.has(entityId);
  }

  /**
   * Validate that hyperEdges only exist between valid, visible endpoints
   */
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

  /**
   * Clean up hyperEdges that connect to nodes that are no longer visible
   */
  private cleanupInvalidHyperEdges(): void {
    const hyperEdgesToRemove: string[] = [];
    
    // Check all hyperEdges for invalid endpoints
    for (const [hyperEdgeId, hyperEdge] of this.state._collections.hyperEdges.entries()) {
      const sourceVisible = this.isNodeOrContainerVisible(hyperEdge.source);
      const targetVisible = this.isNodeOrContainerVisible(hyperEdge.target);
      
      // Remove hyperEdges with any hidden endpoints
      // HyperEdges should only exist between visible entities
      if (!sourceVisible || !targetVisible) {
        hyperEdgesToRemove.push(hyperEdgeId);
      }
    }
    
    // Remove invalid hyperEdges
    for (const hyperEdgeId of hyperEdgesToRemove) {
      this.state.removeHyperEdge(hyperEdgeId);
    }
  }

  /**
   * Recreate hyperEdges for all collapsed containers 
   * Used after expanding a container to ensure other collapsed containers get proper hyperEdges
   */
  private recreateHyperEdgesForAllCollapsedContainers(): void {
    // Find all collapsed containers
    for (const [containerId, container] of this.state._collections.containers.entries()) {
      if (container.collapsed) {
        // First remove existing hyperEdges for this container
        const existingHyperEdges = Array.from(this.state._collections.hyperEdges.entries())
          .filter(([_, hyperEdge]) => hyperEdge.source === containerId || hyperEdge.target === containerId)
          .map(([hyperEdgeId, _]) => hyperEdgeId);
        
        for (const hyperEdgeId of existingHyperEdges) {
          this.state.removeHyperEdge(hyperEdgeId);
        }
        
        // Then recreate hyperEdges based on current visibility
        this.createHyperEdgesForCollapsedContainer(containerId);
      }
    }
  }

  /**
   * Recreate hyperEdges only for collapsed containers that have crossings with the provided nodes.
   * This is a targeted alternative to scanning all collapsed containers, improving efficiency
   * after local visibility changes (e.g., expansion of a single container).
   */
  private recreateHyperEdgesForCollapsedNeighbors(changedNodes: Set<string>): void {
    if (changedNodes.size === 0) return;

    // Collect candidate collapsed containers by checking crossings for each changed node
    const candidateContainers = new Set<string>();

    // For each collapsed container, quickly filter by whether any changed node is inside it or adjacent by crossing
    for (const [containerId, container] of this.state._collections.containers.entries()) {
      if (!container.collapsed) continue;

      // Fast path: if any changed node resides inside this container, it may affect crossings
      let affects = false;
      for (const nodeId of changedNodes) {
        if (this.isNodeInContainerRecursive(nodeId, containerId)) {
          affects = true;
          break;
        }
      }

      // If not directly inside, check if any edge crossing involves a changed node
      if (!affects) {
        const crossings = this.findCrossingEdges(containerId);
        for (const edge of crossings) {
          if (changedNodes.has(edge.source) || changedNodes.has(edge.target)) {
            affects = true;
            break;
          }
        }
      }

      if (affects) candidateContainers.add(containerId);
    }

    // Now for each affected collapsed container, rebuild its hyperEdges
    for (const containerId of candidateContainers) {
      // Remove existing hyperEdges connected to this container
      const existingHyperEdges = Array.from(this.state._collections.hyperEdges.entries())
        .filter(([_, hyperEdge]) => hyperEdge.source === containerId || hyperEdge.target === containerId)
        .map(([hyperEdgeId, _]) => hyperEdgeId);

      for (const hyperEdgeId of existingHyperEdges) {
        this.state.removeHyperEdge(hyperEdgeId);
      }

      // Recreate based on current visibility/crossings
      this.createHyperEdgesForCollapsedContainer(containerId);
    }
  }
}
