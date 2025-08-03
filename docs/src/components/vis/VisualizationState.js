/**
 * Visualization State - Core Data Structure
 * 
 * Maintains the mutable state of the visualization including nodes, edges, containers, and hyperEdges.
 * Provides efficient access to visible/non-hidden elements through Maps and collections.
 */

// Style constants
export const NODE_STYLES = {
  DEFAULT: 'default',
  HIGHLIGHTED: 'highlighted',
  SELECTED: 'selected',
  WARNING: 'warning',
  ERROR: 'error'
};

export const EDGE_STYLES = {
  DEFAULT: 'default',
  HIGHLIGHTED: 'highlighted',
  DASHED: 'dashed',
  THICK: 'thick',
  WARNING: 'warning'
};

/**
 * Core visualization state class that manages all graph elements
 */
export class VisualizationState {
  constructor() {
    // Core graph elements
    this.graphNodes = new Map(); // id -> GraphNode
    this.graphEdges = new Map(); // id -> GraphEdge
    this.containers = new Map(); // id -> Container
    this.hyperEdges = new Map(); // id -> HyperEdge
    
    // Efficient access collections for visible elements
    this.visibleNodes = new Map(); // id -> GraphNode (non-hidden)
    this.visibleEdges = new Map(); // id -> GraphEdge (non-hidden)
    this.visibleContainers = new Map(); // id -> Container (non-hidden)
    this.expandedContainers = new Map(); // id -> Container (non-collapsed)
    
    // Container hierarchy tracking
    this.containerChildren = new Map(); // containerId -> Set of child node/container ids
    this.nodeContainers = new Map(); // nodeId -> containerId
  }

  // ============ Graph Nodes ============
  
  /**
   * Add or update a graph node
   */
  setGraphNode(id, { label, style = NODE_STYLES.DEFAULT, hidden = false, ...otherProps }) {
    const node = {
      id,
      label,
      style,
      hidden,
      ...otherProps
    };
    
    this.graphNodes.set(id, node);
    this._updateVisibleNodes(id, node);
    return node;
  }

  /**
   * Get a graph node by id
   */
  getGraphNode(id) {
    return this.graphNodes.get(id);
  }

  /**
   * Set hidden flag for a graph node
   */
  setNodeHidden(id, hidden) {
    const node = this.graphNodes.get(id);
    if (node) {
      node.hidden = hidden;
      this._updateVisibleNodes(id, node);
    }
  }

  /**
   * Get hidden flag for a graph node
   */
  getNodeHidden(id) {
    const node = this.graphNodes.get(id);
    return node ? node.hidden : undefined;
  }

  /**
   * Remove a graph node
   */
  removeGraphNode(id) {
    this.graphNodes.delete(id);
    this.visibleNodes.delete(id);
    this.nodeContainers.delete(id);
  }

  // ============ Graph Edges ============
  
  /**
   * Add or update a graph edge
   */
  setGraphEdge(id, { source, target, style = EDGE_STYLES.DEFAULT, hidden = false, ...otherProps }) {
    const edge = {
      id,
      source,
      target,
      style,
      hidden,
      ...otherProps
    };
    
    this.graphEdges.set(id, edge);
    this._updateVisibleEdges(id, edge);
    return edge;
  }

  /**
   * Get a graph edge by id
   */
  getGraphEdge(id) {
    return this.graphEdges.get(id);
  }

  /**
   * Set hidden flag for a graph edge
   */
  setEdgeHidden(id, hidden) {
    const edge = this.graphEdges.get(id);
    if (edge) {
      edge.hidden = hidden;
      this._updateVisibleEdges(id, edge);
    }
  }

  /**
   * Get hidden flag for a graph edge
   */
  getEdgeHidden(id) {
    const edge = this.graphEdges.get(id);
    return edge ? edge.hidden : undefined;
  }

  /**
   * Remove a graph edge
   */
  removeGraphEdge(id) {
    this.graphEdges.delete(id);
    this.visibleEdges.delete(id);
  }

  // ============ Containers ============
  
  /**
   * Add or update a container
   */
  setContainer(id, { 
    expandedDimensions = { width: 0, height: 0 }, 
    collapsed = false, 
    hidden = false,
    children = [],
    ...otherProps 
  }) {
    const container = {
      id,
      expandedDimensions,
      collapsed,
      hidden,
      children: new Set(children),
      ...otherProps
    };
    
    this.containers.set(id, container);
    this._updateVisibleContainers(id, container);
    this._updateExpandedContainers(id, container);
    
    // Update container hierarchy
    this.containerChildren.set(id, container.children);
    for (const childId of children) {
      this.nodeContainers.set(childId, id);
    }
    
    return container;
  }

  /**
   * Get a container by id
   */
  getContainer(id) {
    return this.containers.get(id);
  }

  /**
   * Set collapsed flag for a container
   */
  setContainerCollapsed(id, collapsed) {
    const container = this.containers.get(id);
    if (container) {
      container.collapsed = collapsed;
      this._updateExpandedContainers(id, container);
    }
  }

  /**
   * Get collapsed flag for a container
   */
  getContainerCollapsed(id) {
    const container = this.containers.get(id);
    return container ? container.collapsed : undefined;
  }

  /**
   * Set hidden flag for a container
   */
  setContainerHidden(id, hidden) {
    const container = this.containers.get(id);
    if (container) {
      container.hidden = hidden;
      this._updateVisibleContainers(id, container);
    }
  }

  /**
   * Get hidden flag for a container
   */
  getContainerHidden(id) {
    const container = this.containers.get(id);
    return container ? container.hidden : undefined;
  }

  /**
   * Add a child to a container
   */
  addContainerChild(containerId, childId) {
    const container = this.containers.get(containerId);
    if (container) {
      container.children.add(childId);
      this.containerChildren.set(containerId, container.children);
      this.nodeContainers.set(childId, containerId);
    }
  }

  /**
   * Remove a child from a container
   */
  removeContainerChild(containerId, childId) {
    const container = this.containers.get(containerId);
    if (container) {
      container.children.delete(childId);
      this.containerChildren.set(containerId, container.children);
      this.nodeContainers.delete(childId);
    }
  }

  /**
   * Remove a container
   */
  removeContainer(id) {
    this.containers.delete(id);
    this.visibleContainers.delete(id);
    this.expandedContainers.delete(id);
    this.containerChildren.delete(id);
  }

  // ============ Hyper Edges ============
  
  /**
   * Add or update a hyper edge
   */
  setHyperEdge(id, { source, target, style = EDGE_STYLES.DEFAULT, ...otherProps }) {
    const hyperEdge = {
      id,
      source,
      target,
      style,
      ...otherProps
    };
    
    this.hyperEdges.set(id, hyperEdge);
    return hyperEdge;
  }

  /**
   * Get a hyper edge by id
   */
  getHyperEdge(id) {
    return this.hyperEdges.get(id);
  }

  /**
   * Remove a hyper edge
   */
  removeHyperEdge(id) {
    this.hyperEdges.delete(id);
  }

  // ============ Bulk Operations ============
  
  /**
   * Get all visible (non-hidden) nodes
   */
  getVisibleNodes() {
    return Array.from(this.visibleNodes.values());
  }

  /**
   * Get all visible (non-hidden) edges
   */
  getVisibleEdges() {
    return Array.from(this.visibleEdges.values());
  }

  /**
   * Get all visible (non-hidden) containers
   */
  getVisibleContainers() {
    return Array.from(this.visibleContainers.values());
  }

  /**
   * Get all expanded (non-collapsed) containers
   */
  getExpandedContainers() {
    return Array.from(this.expandedContainers.values());
  }

  /**
   * Get all hyper edges
   */
  getHyperEdges() {
    return Array.from(this.hyperEdges.values());
  }

  /**
   * Get container children for a container id
   */
  getContainerChildren(containerId) {
    return this.containerChildren.get(containerId) || new Set();
  }

  /**
   * Get the container that contains a given node
   */
  getNodeContainer(nodeId) {
    return this.nodeContainers.get(nodeId);
  }

  /**
   * Clear all data
   */
  clear() {
    this.graphNodes.clear();
    this.graphEdges.clear();
    this.containers.clear();
    this.hyperEdges.clear();
    this.visibleNodes.clear();
    this.visibleEdges.clear();
    this.visibleContainers.clear();
    this.expandedContainers.clear();
    this.containerChildren.clear();
    this.nodeContainers.clear();
  }

  // ============ Private Helper Methods ============
  
  _updateVisibleNodes(id, node) {
    if (node.hidden) {
      this.visibleNodes.delete(id);
    } else {
      this.visibleNodes.set(id, node);
    }
  }

  _updateVisibleEdges(id, edge) {
    if (edge.hidden) {
      this.visibleEdges.delete(id);
    } else {
      this.visibleEdges.set(id, edge);
    }
  }

  _updateVisibleContainers(id, container) {
    if (container.hidden) {
      this.visibleContainers.delete(id);
    } else {
      this.visibleContainers.set(id, container);
    }
  }

  _updateExpandedContainers(id, container) {
    if (container.collapsed) {
      this.expandedContainers.delete(id);
    } else {
      this.expandedContainers.set(id, container);
    }
  }
}

/**
 * Create a new visualization state instance
 */
export function createVisualizationState() {
  return new VisualizationState();
}
