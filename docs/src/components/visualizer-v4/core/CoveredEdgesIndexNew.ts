/**
 * CoveredEdgesIndex - Maintains an index of which GraphEdges are "covered" by each container
 * 
 * This replaces the complex aggregatedEdges tracking in hyperEdges with a simpler approach:
 * - Each container tracks all GraphEdges that are recursively underneath it
 * - HyperEdges become simple connection representations without embedded edge data
 * - Aggregated edges are computed on-demand using this index
 * 
 * The index is maintained incrementally as the hierarchy changes.
 */

import { GraphEdge, Container } from './types';

export class CoveredEdgesIndex {
  private containerToEdges = new Map<string, Set<string>>(); // container -> edge IDs
  private edgeToContainers = new Map<string, Set<string>>(); // edge ID -> container IDs that cover it
  
  /**
   * Initialize the index from the current state
   */
  initialize(
    containers: Map<string, Container>,
    edges: Map<string, GraphEdge>,
    containerChildren: Map<string, Set<string>>,
    nodeContainers: Map<string, string>
  ): void {
    this.containerToEdges.clear();
    this.edgeToContainers.clear();
    
    // Initialize empty sets for all containers
    for (const containerId of containers.keys()) {
      this.containerToEdges.set(containerId, new Set());
    }
    
    // For each edge, determine which containers cover it
    for (const [edgeId, edge] of edges.entries()) {
      const sourceContainers = this.getContainerAncestors(edge.source, nodeContainers);
      const targetContainers = this.getContainerAncestors(edge.target, nodeContainers);
      
      // An edge is covered by a container if both its endpoints are descendants of that container
      const coveringContainers = this.intersectSets(sourceContainers, targetContainers);
      
      for (const containerId of coveringContainers) {
        this.addEdgeToContainer(edgeId, containerId);
      }
    }
  }
  
  /**
   * Get all edges covered by a container (including those in descendant containers)
   */
  getCoveredEdges(containerId: string): ReadonlySet<string> {
    return this.containerToEdges.get(containerId) || new Set();
  }
  
  /**
   * Add a new edge to the index
   */
  addEdge(
    edgeId: string,
    edge: GraphEdge,
    nodeContainers: Map<string, string>
  ): void {
    const sourceContainers = this.getContainerAncestors(edge.source, nodeContainers);
    const targetContainers = this.getContainerAncestors(edge.target, nodeContainers);
    const coveringContainers = this.intersectSets(sourceContainers, targetContainers);
    
    for (const containerId of coveringContainers) {
      this.addEdgeToContainer(edgeId, containerId);
    }
  }
  
  /**
   * Remove an edge from the index
   */
  removeEdge(edgeId: string): void {
    const containers = this.edgeToContainers.get(edgeId);
    if (containers) {
      for (const containerId of containers) {
        this.removeEdgeFromContainer(edgeId, containerId);
      }
    }
  }
  
  /**
   * Add a new container to the index
   */
  addContainer(containerId: string): void {
    if (!this.containerToEdges.has(containerId)) {
      this.containerToEdges.set(containerId, new Set());
    }
  }
  
  /**
   * Remove a container from the index
   */
  removeContainer(containerId: string): void {
    const edges = this.containerToEdges.get(containerId);
    if (edges) {
      for (const edgeId of edges) {
        this.removeEdgeFromContainer(edgeId, containerId);
      }
    }
    this.containerToEdges.delete(containerId);
  }
  
  /**
   * Update the index when a node moves between containers
   */
  moveNode(
    nodeId: string,
    edges: Map<string, GraphEdge>,
    nodeContainers: Map<string, string>
  ): void {
    // Find all edges connected to this node
    const connectedEdges: GraphEdge[] = [];
    for (const edge of edges.values()) {
      if (edge.source === nodeId || edge.target === nodeId) {
        connectedEdges.push(edge);
      }
    }
    
    // Update coverage for each connected edge
    for (const edge of connectedEdges) {
      this.removeEdge(edge.id);
      this.addEdge(edge.id, edge, nodeContainers);
    }
  }
  
  /**
   * Private helper methods
   */
  
  private addEdgeToContainer(edgeId: string, containerId: string): void {
    // Add to container -> edges mapping
    let edges = this.containerToEdges.get(containerId);
    if (!edges) {
      edges = new Set();
      this.containerToEdges.set(containerId, edges);
    }
    edges.add(edgeId);
    
    // Add to edge -> containers mapping
    let containers = this.edgeToContainers.get(edgeId);
    if (!containers) {
      containers = new Set();
      this.edgeToContainers.set(edgeId, containers);
    }
    containers.add(containerId);
  }
  
  private removeEdgeFromContainer(edgeId: string, containerId: string): void {
    // Remove from container -> edges mapping
    const edges = this.containerToEdges.get(containerId);
    if (edges) {
      edges.delete(edgeId);
    }
    
    // Remove from edge -> containers mapping
    const containers = this.edgeToContainers.get(edgeId);
    if (containers) {
      containers.delete(containerId);
      if (containers.size === 0) {
        this.edgeToContainers.delete(edgeId);
      }
    }
  }
  
  private getContainerAncestors(
    nodeId: string,
    nodeContainers: Map<string, string>
  ): Set<string> {
    const ancestors = new Set<string>();
    let currentContainer = nodeContainers.get(nodeId);
    
    while (currentContainer) {
      ancestors.add(currentContainer);
      // Find parent of current container
      currentContainer = nodeContainers.get(currentContainer);
    }
    
    return ancestors;
  }
  
  private intersectSets<T>(set1: Set<T>, set2: Set<T>): Set<T> {
    const intersection = new Set<T>();
    for (const item of set1) {
      if (set2.has(item)) {
        intersection.add(item);
      }
    }
    return intersection;
  }
  
  /**
   * Debug method to get statistics about the index
   */
  getStats(): { containers: number; totalEdges: number; averageEdgesPerContainer: number } {
    const containers = this.containerToEdges.size;
    let totalEdges = 0;
    for (const edges of this.containerToEdges.values()) {
      totalEdges += edges.size;
    }
    return {
      containers,
      totalEdges,
      averageEdgesPerContainer: containers > 0 ? totalEdges / containers : 0
    };
  }
}
