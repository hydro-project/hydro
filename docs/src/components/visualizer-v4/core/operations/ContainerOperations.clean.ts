import { VisualizationState } from '../VisualizationState';
import { GraphEdge, HyperEdge, isGraphEdge } from '../types';
import { createHyperEdge } from '../EdgeFactory';

/**
 * Container Operations - Handles container collapse/expand with the new CoveredEdgesIndex architecture
 */
export class ContainerOperations {
  constructor(private state: VisualizationState) {}

  /**
   * Handle container collapse with recursive child collapse and hyperEdge creation
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
    this.deleteAdjacentEdges(new Set(children));
    
    // Step 3: Create hyperEdges based on external connections
    this.createHyperEdgesForCollapsedContainer(containerId);
    
    // Step 4: Mark container as collapsed
    this.state.setContainerCollapsed(containerId, true);
  }

  /**
   * Handle container expansion by removing hyperEdges and restoring GraphEdge visibility
   */
  handleContainerExpansion(containerId: string): void {
    // 1. Find all hyperEdges connected to this container and remove them
    const hyperEdgesToRemove: string[] = [];
    
    for (const hyperEdge of this.state.visibleHyperEdges) {
      const he = hyperEdge as HyperEdge;
      if (he.source === containerId || he.target === containerId) {
        hyperEdgesToRemove.push(he.id);
      }
    }

    // 2. Remove the hyperEdges (they are no longer needed)
    for (const hyperEdgeId of hyperEdgesToRemove) {
      this.state.removeHyperEdge(hyperEdgeId);
    }

    // 3. Mark container as expanded
    this.state.setContainerCollapsed(containerId, false);

    // 4. Restore visibility of GraphEdges using CoveredEdgesIndex
    const coveredEdgeIds = this.state.getAggregatedEdges(containerId);
    for (const edgeId of coveredEdgeIds) {
      const edge = this.state.getGraphEdge(edgeId);
      if (edge) {
        // Check if both endpoints are now visible
        const sourceVisible = this.isNodeOrContainerVisible(edge.source);
        const targetVisible = this.isNodeOrContainerVisible(edge.target);
        if (sourceVisible && targetVisible) {
          this.state.setEdgeVisibility(edge.id, true);
        }
      }
    }
  }

  /**
   * Recursively expand container and all its child containers
   */
  handleContainerExpansionRecursive(containerId: string): void {
    // First expand this container
    this.handleContainerExpansion(containerId);

    // Then recursively expand all child containers
    const children = this.state.getContainerChildren(containerId) || new Set();
    for (const childId of Array.from(children)) {
      const childContainer = this.state.getContainer(childId);
      if (childContainer && this.state.getContainerCollapsed(childId)) {
        this.handleContainerExpansionRecursive(childId);
      }
    }
  }

  /**
   * Get crossing edges for a container (GraphEdges only, not hyperEdges)
   */
  getCrossingEdges(containerId: string): GraphEdge[] {
    const children = this.state.getContainerChildren(containerId) || new Set();
    const crossingEdges: GraphEdge[] = [];

    // Look through all visible edges
    for (const edge of this.state.visibleEdges) {
      if (isGraphEdge(edge)) {
        const sourceInContainer = this.isNodeInContainerRecursive(edge.source, containerId);
        const targetInContainer = this.isNodeInContainerRecursive(edge.target, containerId);

        // Edge crosses the container boundary if exactly one endpoint is inside
        if (sourceInContainer !== targetInContainer) {
          crossingEdges.push(edge);
        }
      }
    }

    return crossingEdges;
  }

  // Private helper methods

  /**
   * Delete all GraphEdges that are adjacent to the given nodes/containers
   */
  private deleteAdjacentEdges(nodeIds: Set<string>): void {
    const edgesToDelete: string[] = [];
    
    // Find all GraphEdges adjacent to the nodes being collapsed
    for (const edge of this.state.visibleEdges) {
      if (isGraphEdge(edge)) {
        if (nodeIds.has(edge.source) || nodeIds.has(edge.target)) {
          edgesToDelete.push(edge.id);
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
    // Get all edge IDs that would be covered by this container
    const coveredEdgeIds = this.state.getAggregatedEdges(containerId);
    
    // Group external connections by endpoint
    const externalConnections = new Map<string, { incoming: GraphEdge[], outgoing: GraphEdge[] }>();
    
    for (const edgeId of coveredEdgeIds) {
      const edge = this.state.getGraphEdge(edgeId);
      if (!edge) continue;
      
      const sourceInContainer = this.isNodeInContainerRecursive(edge.source, containerId);
      const targetInContainer = this.isNodeInContainerRecursive(edge.target, containerId);
      
      // Skip internal edges (both endpoints in container)
      if (sourceInContainer && targetInContainer) {
        continue;
      }
      
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
      // Create incoming hyperEdge (external -> container)
      if (group.incoming.length > 0) {
        const hyperEdgeId = `hyper_${externalEndpoint}_to_${containerId}`;
        const hyperEdge = createHyperEdge({
          id: hyperEdgeId,
          source: externalEndpoint,
          target: containerId
        });
        this.state.setHyperEdge(hyperEdge.id, hyperEdge);
      }
      
      // Create outgoing hyperEdge (container -> external)
      if (group.outgoing.length > 0) {
        const hyperEdgeId = `hyper_${containerId}_to_${externalEndpoint}`;
        const hyperEdge = createHyperEdge({
          id: hyperEdgeId,
          source: containerId,
          target: externalEndpoint
        });
        this.state.setHyperEdge(hyperEdge.id, hyperEdge);
      }
    }
  }

  /**
   * Check if a node is recursively contained within a container
   */
  private isNodeInContainerRecursive(nodeId: string, containerId: string): boolean {
    const children = this.state.getContainerChildren(containerId) || new Set();
    
    // Direct child
    if (children.has(nodeId)) {
      return true;
    }
    
    // Check nested containers
    for (const childId of children) {
      const childContainer = this.state.getContainer(childId);
      if (childContainer && this.isNodeInContainerRecursive(nodeId, childId)) {
        return true;
      }
    }
    
    return false;
  }

  /**
   * Find the lowest visible ancestor of a node
   */
  private findLowestVisibleAncestor(nodeId: string): string {
    // If the node itself is visible, return it
    if (this.isNodeOrContainerVisible(nodeId)) {
      return nodeId;
    }
    
    // Find the container that contains this node
    const parentContainer = this.state.getNodeContainer(nodeId);
    if (parentContainer) {
      return this.findLowestVisibleAncestor(parentContainer);
    }
    
    // Node has no parent, return itself
    return nodeId;
  }

  /**
   * Check if a node or container is visible
   */
  private isNodeOrContainerVisible(nodeId: string): boolean {
    // Check if it's a visible node
    const node = this.state.getGraphNode(nodeId);
    if (node && !node.hidden) {
      return true;
    }
    
    // Check if it's a visible container
    const container = this.state.getContainer(nodeId);
    if (container && !container.hidden) {
      return true;
    }
    
    return false;
  }

  /**
   * Handle container expansion by removing hyperEdges and restoring GraphEdge visibility
   */
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
    if (this.state._collections._visibleNodes.has(entityId)) return true;
    if (this.state._collections._visibleContainers.has(entityId)) return true;
    return false;
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
}
