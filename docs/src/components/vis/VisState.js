/**
 * Visualization State - Core Data Structure
 * 
 * Maintains the mutable state of the visualization including nodes, edges, containers, and hyperEdges.
 * Provides efficient access to visible/non-hidden elements through Maps and collections.
 */

import { NODE_STYLES, EDGE_STYLES } from './constants.js';

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
    
    // Collapsed container representations
    this.collapsedContainers = new Map(); // id -> CollapsedContainer
    
    // Container hierarchy tracking
    this.containerChildren = new Map(); // containerId -> Set of child node/container ids
    this.nodeContainers = new Map(); // nodeId -> containerId
    
    // Edge tracking for hyperEdge management
    this.nodeToEdges = new Map(); // nodeId -> Set of edge ids connected to this node
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
    
    // Maintain nodeToEdges mapping
    this._addEdgeToNodeMapping(id, source, target);
    
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
    const edge = this.graphEdges.get(id);
    if (edge) {
      this._removeEdgeFromNodeMapping(id, edge.source, edge.target);
    }
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
    this.collapsedContainers.clear();
    this.containerChildren.clear();
    this.nodeContainers.clear();
    this.nodeToEdges.clear();
  }

  // ============ Container Collapse/Expand Transitions ============
  
  /**
   * Collapse a container (depth-first, bottom-up with edge lifting)
   */
  collapseContainer(containerId) {
    const container = this.containers.get(containerId);
    if (!container || container.collapsed) {
      return; // Already collapsed or doesn't exist
    }
    
    // First, recursively collapse any child containers (bottom-up)
    const children = this.getContainerChildren(containerId);
    for (const childId of children) {
      if (this.containers.has(childId)) {
        this.collapseContainer(childId);
      }
    }
    
    // Now collapse this container and lift edges/hyperEdges to this level
    this._performContainerCollapse(containerId);
  }
  
  /**
   * Expand a container (depth-first, top-down restoration)
   */
  expandContainer(containerId) {
    const container = this.containers.get(containerId);
    if (!container || !container.collapsed) {
      return; // Already expanded or doesn't exist
    }
    
    // First expand this container
    this._performContainerExpansion(containerId);
    
    // Then recursively expand any child containers (top-down)
    const children = this.getContainerChildren(containerId);
    for (const childId of children) {
      if (this.containers.has(childId)) {
        this.expandContainer(childId);
      }
    }
  }

  /**
   * Perform the actual collapse operation for a single container
   * This includes lifting edges and hyperEdges from child containers
   */
  _performContainerCollapse(containerId) {
    const container = this.containers.get(containerId);
    
    // 1. Create collapsed container representation
    const collapsedContainer = {
      id: containerId,
      originalContainer: container,
      style: container.style || 'default'
    };
    this.collapsedContainers.set(containerId, collapsedContainer);
    
    // 2. Mark container as collapsed
    container.collapsed = true;
    this._updateExpandedContainers(containerId, container);
    
    // 3. Identify what we're collapsing
    const children = this.getContainerChildren(containerId);
    const containerNodes = new Set(); // Direct child nodes
    const childContainers = new Set(); // Direct child containers
    
    for (const childId of children) {
      if (this.graphNodes.has(childId)) {
        containerNodes.add(childId);
      } else if (this.containers.has(childId)) {
        childContainers.add(childId);
      }
    }
    
    // 4. Hide all direct child nodes
    for (const nodeId of containerNodes) {
      this.setNodeHidden(nodeId, true);
    }
    
    // 5. Lift edges and hyperEdges to this container level
    this._liftEdgesToContainer(containerId, containerNodes, childContainers);
  }

  /**
   * Lift edges and hyperEdges from nodes and child containers to the parent container
   */
  _liftEdgesToContainer(containerId, containerNodes, childContainers) {
    const liftedConnections = new Map(); // externalId -> {incoming: Set, outgoing: Set}
    
    // Process direct node edges
    this._liftNodeEdges(containerId, containerNodes, liftedConnections);
    
    // Process hyperEdges from child containers (lift them to this level)
    this._liftChildContainerHyperEdges(containerId, childContainers, liftedConnections);
    
    // Create new hyperEdges for all lifted connections
    this._createHyperEdgesFromConnections(containerId, liftedConnections);
  }

  /**
   * Lift edges from direct child nodes
   */
  _liftNodeEdges(containerId, containerNodes, liftedConnections) {
    const processedEdges = new Set();
    
    for (const nodeId of containerNodes) {
      const connectedEdges = this.nodeToEdges.get(nodeId) || new Set();
      
      for (const edgeId of connectedEdges) {
        if (processedEdges.has(edgeId)) continue;
        processedEdges.add(edgeId);
        
        const edge = this.graphEdges.get(edgeId);
        if (!edge) continue;
        
        const sourceInContainer = containerNodes.has(edge.source);
        const targetInContainer = containerNodes.has(edge.target);
        
        if (sourceInContainer && targetInContainer) {
          // Both endpoints in container - hide the edge (internal edge)
          this.setEdgeHidden(edgeId, true);
        } else if (sourceInContainer || targetInContainer) {
          // One endpoint in container, one external - check if external node is visible
          const externalId = sourceInContainer ? edge.target : edge.source;
          const internalId = sourceInContainer ? edge.source : edge.target;
          const externalNode = this.graphNodes.get(externalId);
          
          // Only create hyperEdge if the external endpoint is actually visible
          if (externalNode && !externalNode.hidden) {
            const isOutgoing = sourceInContainer; // container -> external
            this._addToLiftedConnections(liftedConnections, externalId, edge, isOutgoing, internalId);
          }
          
          // Hide the original edge regardless
          this.setEdgeHidden(edgeId, true);
        }
      }
    }
  }

  /**
   * Lift hyperEdges from child containers to this container level
   */
  _liftChildContainerHyperEdges(containerId, childContainers, liftedConnections) {
    const hyperEdgesToRemove = [];
    
    for (const [hyperEdgeId, hyperEdge] of this.hyperEdges) {
      const sourceIsChild = childContainers.has(hyperEdge.source);
      const targetIsChild = childContainers.has(hyperEdge.target);
      
      if (sourceIsChild || targetIsChild) {
        // This hyperEdge connects a child container to something external
        const externalId = sourceIsChild ? hyperEdge.target : hyperEdge.source;
        const isOutgoing = sourceIsChild; // child container -> external
        
        // Only lift if the external endpoint is visible
        let shouldLift = false;
        
        // Check if external endpoint is a visible node
        const externalNode = this.graphNodes.get(externalId);
        if (externalNode && !externalNode.hidden) {
          shouldLift = true;
        }
        
        // Check if external endpoint is a visible, collapsed container
        const externalContainer = this.containers.get(externalId);
        if (externalContainer && !externalContainer.hidden && externalContainer.collapsed) {
          shouldLift = true;
        }
        
        if (shouldLift) {
          // Lift all original edges from this hyperEdge
          if (hyperEdge.originalEdges) {
            for (const originalEdge of hyperEdge.originalEdges) {
              // Use the cached internal endpoint from the child hyperEdge, or derive it
              const childInternalEndpoint = hyperEdge.originalInternalEndpoint || 
                (sourceIsChild ? hyperEdge.source : hyperEdge.target);
              this._addToLiftedConnections(liftedConnections, externalId, originalEdge, isOutgoing, childInternalEndpoint);
            }
          }
        }
        
        hyperEdgesToRemove.push(hyperEdgeId);
      }
    }
    
    // Remove the child container hyperEdges
    for (const hyperEdgeId of hyperEdgesToRemove) {
      this.removeHyperEdge(hyperEdgeId);
    }
  }

  /**
   * Helper to add an edge to lifted connections with proper direction
   */
  _addToLiftedConnections(liftedConnections, externalId, edge, isOutgoing, internalEndpoint) {
    if (!liftedConnections.has(externalId)) {
      liftedConnections.set(externalId, { incoming: new Set(), outgoing: new Set() });
    }
    
    const direction = isOutgoing ? 'outgoing' : 'incoming';
    const connections = liftedConnections.get(externalId);
    
    // Store the edge with its original internal endpoint
    const edgeWithEndpoint = { 
      ...edge, 
      originalInternalEndpoint: internalEndpoint 
    };
    
    connections[direction].add(edgeWithEndpoint);
  }

  /**
   * Create hyperEdges from lifted connections
   */
  _createHyperEdgesFromConnections(containerId, liftedConnections) {
    for (const [externalId, connections] of liftedConnections) {
      if (connections.incoming.size > 0) {
        const hyperEdgeId = `hyper_${externalId}_to_${containerId}`;
        const edgesArray = Array.from(connections.incoming);
        this.setHyperEdge(hyperEdgeId, {
          source: externalId,
          target: containerId,
          style: this._aggregateEdgeStyles(edgesArray),
          originalEdges: edgesArray.map(e => ({ id: e.id, source: e.source, target: e.target, style: e.style })),
          // Use the cached internal endpoint from the first edge (they should all be the same for a given external endpoint)
          originalInternalEndpoint: edgesArray[0].originalInternalEndpoint || this._findOriginalInternalEndpoint(edgesArray, containerId)
        });
      }
      
      if (connections.outgoing.size > 0) {
        const hyperEdgeId = `hyper_${containerId}_to_${externalId}`;
        const edgesArray = Array.from(connections.outgoing);
        this.setHyperEdge(hyperEdgeId, {
          source: containerId,
          target: externalId,
          style: this._aggregateEdgeStyles(edgesArray),
          originalEdges: edgesArray.map(e => ({ id: e.id, source: e.source, target: e.target, style: e.style })),
          // Use the cached internal endpoint from the first edge (they should all be the same for a given external endpoint)
          originalInternalEndpoint: edgesArray[0].originalInternalEndpoint || this._findOriginalInternalEndpoint(edgesArray, containerId)
        });
      }
    }
  }

  /**
   * Find the original internal endpoint that should receive grounded connections
   * For multiple edges, prefer containers over nodes, then use the first one
   */
  _findOriginalInternalEndpoint(edges, containerId) {
    const children = this.getContainerChildren(containerId);
    const internalEndpoints = new Set();
    
    for (const edge of edges) {
      const internalEndpoint = children.has(edge.source) ? edge.source : edge.target;
      internalEndpoints.add(internalEndpoint);
    }
    
    // If multiple internal endpoints, prefer containers over nodes
    const containerEndpoints = Array.from(internalEndpoints).filter(id => this.containers.has(id));
    const nodeEndpoints = Array.from(internalEndpoints).filter(id => this.graphNodes.has(id));
    
    if (containerEndpoints.length > 0) {
      return containerEndpoints[0]; // Prefer containers
    } else if (nodeEndpoints.length > 0) {
      return nodeEndpoints[0]; // Fall back to nodes
    }
    
    return Array.from(internalEndpoints)[0]; // Fallback
  }
  
  /**
   * Perform the actual expansion operation for a single container
   * Mirror image of collapse: restore nodes, edges, and remove hyperEdges with grounding
   */
  _performContainerExpansion(containerId) {
    const container = this.containers.get(containerId);
    const collapsedContainer = this.collapsedContainers.get(containerId);
    
    if (!collapsedContainer) return;
    
    // 1. Mark container as expanded
    container.collapsed = false;
    this._updateExpandedContainers(containerId, container);
    
    // 2. Remove collapsed container representation
    this.collapsedContainers.delete(containerId);
    
    // 3. Show all direct children (nodes and containers)
    // Since we're going top-down in a tree, all direct children should become visible
    const children = this.getContainerChildren(containerId);
    const containerNodes = new Set();
    
    for (const childId of children) {
      if (this.graphNodes.has(childId)) {
        // Show child node only if its parent container is being expanded
        this.setNodeHidden(childId, false);
        containerNodes.add(childId);
      }
      // Child containers will be handled by recursive expansion call
      // but we need to ensure their children are properly handled
    }
    
    // 4. Ground hyperEdges connected to this container
    this._groundHyperEdges(containerId);
    
    // 5. Restore internal edges (edges between nodes in this container)
    // These are edges where both endpoints are now visible nodes
    for (const [edgeId, edge] of this.graphEdges) {
      if (!edge.hidden) continue; // Skip already visible edges
      
      const sourceNode = this.graphNodes.get(edge.source);
      const targetNode = this.graphNodes.get(edge.target);
      
      // Both endpoints must be nodes (not containers) and visible
      if (sourceNode && !sourceNode.hidden && targetNode && !targetNode.hidden) {
        this.setEdgeHidden(edgeId, false);
      }
    }
  }

  /**
   * Ground hyperEdges connected to the expanding container
   * This is the inverse of lifting: restore connections to the correct child endpoints
   */
  _groundHyperEdges(containerId) {
    const hyperEdgesToRemove = [];
    const children = this.getContainerChildren(containerId);
    
    for (const [hyperEdgeId, hyperEdge] of this.hyperEdges) {
      if (hyperEdge.source === containerId || hyperEdge.target === containerId) {
        hyperEdgesToRemove.push(hyperEdgeId);
        
        // Determine the external endpoint and internal endpoint
        const isSourceContainer = hyperEdge.source === containerId;
        const externalId = isSourceContainer ? hyperEdge.target : hyperEdge.source;
        const internalEndpoint = hyperEdge.originalInternalEndpoint;
        
        // Ground the connection based on the state of both endpoints
        this._groundConnection(externalId, internalEndpoint, hyperEdge, isSourceContainer);
      }
    }
    
    // Remove the grounded hyperEdges
    for (const hyperEdgeId of hyperEdgesToRemove) {
      this.removeHyperEdge(hyperEdgeId);
    }
  }

  /**
   * Ground a single connection between external and internal endpoints
   */
  _groundConnection(externalId, internalEndpoint, hyperEdge, isSourceContainer) {
    // Always restore original edges first, regardless of endpoint states
    if (hyperEdge.originalEdges) {
      for (const originalEdge of hyperEdge.originalEdges) {
        this.setEdgeHidden(originalEdge.id, false);
      }
    }
    
    // Then determine if we need to create new connections based on current states
    const externalNode = this.graphNodes.get(externalId);
    const externalContainer = this.containers.get(externalId);
    const internalNode = this.graphNodes.get(internalEndpoint);
    const internalContainer = this.containers.get(internalEndpoint);
    
    const externalIsNode = externalNode && !externalNode.hidden;
    const externalIsCollapsedContainer = externalContainer && !externalContainer.hidden && externalContainer.collapsed;
    const internalIsNode = internalNode && !internalNode.hidden;
    const internalIsCollapsedContainer = internalContainer && !internalContainer.hidden && internalContainer.collapsed;
    
    // Create new hyperEdge only if we have at least one collapsed container and both endpoints are valid
    if ((externalIsNode || externalIsCollapsedContainer) && 
        (internalIsNode || internalIsCollapsedContainer) &&
        (externalIsCollapsedContainer || internalIsCollapsedContainer)) {
      
      const newHyperEdgeId = `hyper_${isSourceContainer ? internalEndpoint : externalId}_to_${isSourceContainer ? externalId : internalEndpoint}`;
      
      this.setHyperEdge(newHyperEdgeId, {
        source: isSourceContainer ? internalEndpoint : externalId,
        target: isSourceContainer ? externalId : internalEndpoint,
        style: hyperEdge.style,
        originalEdges: hyperEdge.originalEdges,
        originalInternalEndpoint: internalEndpoint // Preserve for future operations
      });
    }
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

  /**
   * Add edge to node mapping for efficient edge lookup
   */
  _addEdgeToNodeMapping(edgeId, sourceId, targetId) {
    if (!this.nodeToEdges.has(sourceId)) {
      this.nodeToEdges.set(sourceId, new Set());
    }
    if (!this.nodeToEdges.has(targetId)) {
      this.nodeToEdges.set(targetId, new Set());
    }
    this.nodeToEdges.get(sourceId).add(edgeId);
    this.nodeToEdges.get(targetId).add(edgeId);
  }

  /**
   * Remove edge from node mapping
   */
  _removeEdgeFromNodeMapping(edgeId, sourceId, targetId) {
    const sourceEdges = this.nodeToEdges.get(sourceId);
    if (sourceEdges) {
      sourceEdges.delete(edgeId);
      if (sourceEdges.size === 0) {
        this.nodeToEdges.delete(sourceId);
      }
    }
    const targetEdges = this.nodeToEdges.get(targetId);
    if (targetEdges) {
      targetEdges.delete(edgeId);
      if (targetEdges.size === 0) {
        this.nodeToEdges.delete(targetId);
      }
    }
  }

  /**
   * Aggregate multiple edge styles into a single hyperEdge style
   */
  _aggregateEdgeStyles(edges) {
    // Priority order: ERROR > WARNING > THICK > HIGHLIGHTED > DEFAULT
    const stylePriority = {
      'error': 5,
      'warning': 4,
      'thick': 3,
      'highlighted': 2,
      'default': 1
    };
    
    let highestPriority = 0;
    let resultStyle = EDGE_STYLES.DEFAULT;
    
    for (const edge of edges) {
      const priority = stylePriority[edge.style] || 1;
      if (priority > highestPriority) {
        highestPriority = priority;
        resultStyle = edge.style;
      }
    }
    
    return resultStyle;
  }
}

/**
 * Create a new visualization state instance
 */
export function createVisualizationState() {
  return new VisualizationState();
}
