/**
 * Visualization State - Core Data Structure
 * 
 * Maintains the mutable state of the visualization including nodes, edges, containers, and hyperEdges.
 * Provides efficient access to visible/non-hidden elements through Maps and collections.
 */

import { NODE_STYLES, EDGE_STYLES } from './constants.js';

// Constants for consistent string literals
const HYPER_EDGE_PREFIX = 'hyper_';
const DEFAULT_STYLE = 'default';

// Entity types for generic operations
const ENTITY_TYPES = {
  NODE: 'node',
  EDGE: 'edge',
  CONTAINER: 'container',
  HYPER_EDGE: 'hyperEdge'
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
    
    // Collapsed container representations
    this.collapsedContainers = new Map(); // id -> CollapsedContainer
    
    // Container hierarchy tracking
    this.containerChildren = new Map(); // containerId -> Set of child node/container ids
    this.nodeContainers = new Map(); // nodeId -> containerId
    
    // Edge tracking for hyperEdge management
    this.nodeToEdges = new Map(); // nodeId -> Set of edge ids connected to this node
  }

  // ============ Generic Entity Management ============

  /**
   * Validate that an entity exists and optionally check a condition
   */
  _validateEntity(entity, conditionFn = null) {
    if (!entity) return false;
    return conditionFn ? conditionFn(entity) : true;
  }

  /**
   * Generic method to get an entity from any collection
   */
  _getEntity(entityType, id) {
    const collection = this._getEntityCollection(entityType);
    return collection.get(id);
  }

  /**
   * Generic method to set hidden flag for any entity type that supports it
   */
  _setEntityHidden(entityType, id, hidden) {
    const entity = this._getEntity(entityType, id);
    if (this._validateEntity(entity, e => 'hidden' in e)) {
      entity.hidden = hidden;
      this._updateVisibilityCollection(entityType, id, entity);
    }
  }

  /**
   * Generic method to get hidden flag for any entity type that supports it
   */
  _getEntityHidden(entityType, id) {
    const entity = this._getEntity(entityType, id);
    return entity && 'hidden' in entity ? entity.hidden : undefined;
  }

  /**
   * Get the main collection for an entity type
   */
  _getEntityCollection(entityType) {
    switch (entityType) {
      case ENTITY_TYPES.NODE: return this.graphNodes;
      case ENTITY_TYPES.EDGE: return this.graphEdges;
      case ENTITY_TYPES.CONTAINER: return this.containers;
      case ENTITY_TYPES.HYPER_EDGE: return this.hyperEdges;
      default: throw new Error(`Unknown entity type: ${entityType}`);
    }
  }

  /**
   * Update visibility collections based on entity type and hidden state
   */
  _updateVisibilityCollection(entityType, id, entity) {
    switch (entityType) {
      case ENTITY_TYPES.NODE:
        this._updateVisibleNodes(id, entity);
        break;
      case ENTITY_TYPES.EDGE:
        this._updateVisibleEdges(id, entity);
        break;
      case ENTITY_TYPES.CONTAINER:
        this._updateVisibleContainers(id, entity);
        break;
      // HyperEdges don't have visibility collections
    }
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
    return this._getEntity(ENTITY_TYPES.NODE, id);
  }

  /**
   * Set hidden flag for a graph node
   */
  setNodeHidden(id, hidden) {
    this._setEntityHidden(ENTITY_TYPES.NODE, id, hidden);
  }

  /**
   * Get hidden flag for a graph node
   */
  getNodeHidden(id) {
    return this._getEntityHidden(ENTITY_TYPES.NODE, id);
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
    return this._getEntity(ENTITY_TYPES.EDGE, id);
  }

  /**
   * Set hidden flag for a graph edge
   */
  setEdgeHidden(id, hidden) {
    this._setEntityHidden(ENTITY_TYPES.EDGE, id, hidden);
  }

  /**
   * Get hidden flag for a graph edge
   */
  getEdgeHidden(id) {
    return this._getEntityHidden(ENTITY_TYPES.EDGE, id);
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
    return this._getEntity(ENTITY_TYPES.CONTAINER, id);
  }

  /**
   * Set collapsed flag for a container
   */
  setContainerCollapsed(id, collapsed) {
    const container = this.getContainer(id);
    if (this._validateEntity(container)) {
      container.collapsed = collapsed;
      this._updateExpandedContainers(id, container);
    }
  }

  /**
   * Get collapsed flag for a container
   */
  getContainerCollapsed(id) {
    const container = this.getContainer(id);
    return container ? container.collapsed : undefined;
  }

  /**
   * Set hidden flag for a container
   */
  setContainerHidden(id, hidden) {
    this._setEntityHidden(ENTITY_TYPES.CONTAINER, id, hidden);
  }

  /**
   * Get hidden flag for a container
   */
  getContainerHidden(id) {
    return this._getEntityHidden(ENTITY_TYPES.CONTAINER, id);
  }

  /**
   * Add a child to a container
   */
  addContainerChild(containerId, childId) {
    const container = this.getContainer(containerId);
    if (this._validateEntity(container)) {
      container.children.add(childId);
      this.containerChildren.set(containerId, container.children);
      this.nodeContainers.set(childId, containerId);
    }
  }

  /**
   * Remove a child from a container
   */
  removeContainerChild(containerId, childId) {
    const container = this.getContainer(containerId);
    if (this._validateEntity(container)) {
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
    return this._getEntity(ENTITY_TYPES.HYPER_EDGE, id);
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

  // ============ Container Collapse/Expand Symmetric Operations ============
  
  /**
   * Collapse a container (depth-first, bottom-up with edge lifting)
   */
  collapseContainer(containerId) {
    const container = this.getContainer(containerId);
    if (!this._validateEntity(container, c => !c.collapsed)) {
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
    this._performCollapseWithLift(containerId);
  }
  
  /**
   * Expand a container (depth-first, top-down with edge grounding)
   * SYMMETRIC INVERSE of collapseContainer()
   */
  expandContainer(containerId) {
    const container = this.getContainer(containerId);
    if (!this._validateEntity(container, c => c.collapsed)) {
      return; // Already expanded or doesn't exist
    }
    
    // First expand this container and ground edges/hyperEdges to child level
    this._performExpandWithGround(containerId);
    
    // Then recursively expand any child containers (top-down)
    const children = this.getContainerChildren(containerId);
    for (const childId of children) {
      if (this.containers.has(childId)) {
        this.expandContainer(childId);
      }
    }
  }

  // ============ Collapse/Expand Core Implementation (Symmetric Pair) ============

  /**
   * Perform the actual collapse operation for a single container
   * This includes lifting edges and hyperEdges from child containers
   */
  _performCollapseWithLift(containerId) {
    const container = this.getContainer(containerId);
    
    // 1. Create collapsed container representation
    this._createCollapsedContainerRepresentation(containerId, container);
    
    // 2. Mark container as collapsed
    this._markContainerAsCollapsed(containerId, container);
    
    // 3. Get and categorize children
    const children = this.getContainerChildren(containerId);
    const { containerNodes, childContainers } = this._categorizeChildren(children);
    
    // 4. Hide child nodes and handle edge rerouting
    this._hideChildNodesAndRerouteEdges(containerId, containerNodes);
    
    // 5. Lift edges and hyperEdges to this container level
    this._liftEdgesToContainer(containerId, containerNodes, childContainers);
  }

  /**
   * Create collapsed container representation
   */
  _createCollapsedContainerRepresentation(containerId, container) {
    this.collapsedContainers.set(containerId, {
      id: containerId,
      originalContainer: container,
      style: container.style || DEFAULT_STYLE
    });
  }

  /**
   * Mark container as collapsed and update tracking
   */
  _markContainerAsCollapsed(containerId, container) {
    container.collapsed = true;
    this._updateExpandedContainers(containerId, container);
  }

  /**
   * Hide child nodes and reroute existing hyperEdges
   */
  _hideChildNodesAndRerouteEdges(containerId, containerNodes) {
    // Hide all direct child nodes
    this._setNodesVisibility(containerNodes, true);
    
    // Handle existing hyperEdges that point to nodes we're hiding
    this._rerouteHyperEdgesToCollapsedContainer(containerId, containerNodes);
  }

  /**
   * Perform the actual expansion operation for a single container
   * This includes grounding edges and hyperEdges to child containers
   * SYMMETRIC INVERSE of _performCollapseWithLift()
   */
  _performExpandWithGround(containerId) {
    const container = this.getContainer(containerId);
    const collapsedContainer = this.collapsedContainers.get(containerId);
    
    if (!collapsedContainer) return;
    
    // 1. Mark container as expanded and cleanup
    this._markContainerAsExpandedAndCleanup(containerId, container);
    
    // 2. Show child nodes
    this._showChildNodes(containerId);
    
    // 3. Ground hyperEdges and edges from this container to child level
    this._groundEdgesFromContainer(containerId);
  }

  /**
   * Mark container as expanded and remove collapsed representation
   */
  _markContainerAsExpandedAndCleanup(containerId, container) {
    // Mark container as expanded
    container.collapsed = false;
    this._updateExpandedContainers(containerId, container);
    
    // Remove collapsed container representation
    this.collapsedContainers.delete(containerId);
  }

  /**
   * Show all direct child nodes
   */
  _showChildNodes(containerId) {
    const children = this.getContainerChildren(containerId);
    const { containerNodes } = this._categorizeChildren(children);
    this._setNodesVisibility(containerNodes, false);
  }

  /**
   * Reroute existing hyperEdges that point to nodes we're about to hide
   * when collapsing a container
   */
  _rerouteHyperEdgesToCollapsedContainer(containerId, containerNodes) {
    const hyperEdgesToUpdate = [];
    
    // Find hyperEdges that need rerouting
    for (const [hyperEdgeId, hyperEdge] of this.hyperEdges) {
      const update = this._calculateHyperEdgeReroute(hyperEdge, containerNodes, containerId);
      if (update) {
        hyperEdgesToUpdate.push({ id: hyperEdgeId, originalHyperEdge: hyperEdge, ...update });
      }
    }
    
    // Apply the updates
    for (const update of hyperEdgesToUpdate) {
      this.removeHyperEdge(update.id);
      
      // Only create a new hyperEdge if source and target are different
      if (update.newSource !== update.newTarget) {
        const newHyperEdgeId = `${HYPER_EDGE_PREFIX}${update.newSource}_to_${update.newTarget}`;
        this.setHyperEdge(newHyperEdgeId, {
          source: update.newSource,
          target: update.newTarget,
          style: update.originalHyperEdge.style,
          originalEdges: update.originalHyperEdge.originalEdges,
          originalInternalEndpoint: update.originalHyperEdge.originalInternalEndpoint
        });
      }
    }
  }

  /**
   * Calculate if a hyperEdge needs rerouting and return the new endpoints
   */
  _calculateHyperEdgeReroute(hyperEdge, containerNodes, containerId) {
    let needsUpdate = false;
    let newSource = hyperEdge.source;
    let newTarget = hyperEdge.target;
    
    // Check if source is a node we're hiding
    if (containerNodes.has(hyperEdge.source)) {
      newSource = containerId;
      needsUpdate = true;
    }
    
    // Check if target is a node we're hiding
    if (containerNodes.has(hyperEdge.target)) {
      newTarget = containerId;
      needsUpdate = true;
    }
    
    return needsUpdate ? { newSource, newTarget } : null;
  }

  // ============ Edge Lifting/Grounding Coordination (Symmetric Pair) ============

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
    this._createHyperEdgesFromLiftedConnections(containerId, liftedConnections);
  }

  /**
   * Ground hyperEdges and edges connected to the expanding container
   * This is the inverse of lifting: restore connections to the correct child endpoints
   * SYMMETRIC INVERSE of _liftEdgesToContainer()
   */
  _groundEdgesFromContainer(containerId) {
    const children = this.getContainerChildren(containerId);
    
    // Process hyperEdges connected to this container
    this._groundContainerHyperEdges(containerId);
    
    // Process direct node edges that were hidden during collapse
    this._groundNodeEdges(containerId, children);
  }

  // ============ Node Edge Processing (Symmetric Pair) ============

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
        
        this._processNodeEdge(edge, containerNodes, liftedConnections);
      }
    }
  }

  /**
   * Process a single node edge during lifting
   */
  _processNodeEdge(edge, containerNodes, liftedConnections) {
    const sourceInContainer = containerNodes.has(edge.source);
    const targetInContainer = containerNodes.has(edge.target);
    
    if (sourceInContainer && targetInContainer) {
      // Both endpoints in container - hide the edge (internal edge)
      this.setEdgeHidden(edge.id, true);
    } else if (sourceInContainer || targetInContainer) {
      // One endpoint in container, one external
      const externalId = sourceInContainer ? edge.target : edge.source;
      const internalId = sourceInContainer ? edge.source : edge.target;
      
      // Only create hyperEdge if the external endpoint should be connected
      if (this._isEndpointConnectable(externalId)) {
        const isOutgoing = sourceInContainer; // container -> external
        this._addToLiftedConnections(liftedConnections, externalId, edge, isOutgoing, internalId);
      }
      
      // Hide the original edge regardless
      this.setEdgeHidden(edge.id, true);
    }
  }

  /**
   * Ground edges from direct child nodes
   * SYMMETRIC INVERSE of _liftNodeEdges()
   */
  _groundNodeEdges(containerId, children) {
    // Restore internal edges (edges between nodes in this container)
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

  // ============ Container HyperEdge Processing (Symmetric Pair) ============

  /**
   * Lift hyperEdges from child containers to this container level
   */
  _liftChildContainerHyperEdges(containerId, childContainers, liftedConnections) {
    this._processHyperEdges(
      (hyperEdge) => childContainers.has(hyperEdge.source) || childContainers.has(hyperEdge.target),
      (hyperEdge) => this._liftChildContainerHyperEdge(hyperEdge, childContainers, liftedConnections)
    );
  }

  /**
   * Lift a single child container hyperEdge
   */
  _liftChildContainerHyperEdge(hyperEdge, childContainers, liftedConnections) {
    const sourceIsChild = childContainers.has(hyperEdge.source);
    const targetIsChild = childContainers.has(hyperEdge.target);
    
    if (sourceIsChild || targetIsChild) {
      const externalId = sourceIsChild ? hyperEdge.target : hyperEdge.source;
      const isOutgoing = sourceIsChild; // child container -> external
      
      // Only lift if the external endpoint is connectable
      if (this._isEndpointConnectable(externalId) && hyperEdge.originalEdges) {
        for (const originalEdge of hyperEdge.originalEdges) {
          const childInternalEndpoint = hyperEdge.originalInternalEndpoint || 
            (sourceIsChild ? hyperEdge.source : hyperEdge.target);
          this._addToLiftedConnections(liftedConnections, externalId, originalEdge, isOutgoing, childInternalEndpoint);
        }
      }
    }
  }

  /**
   * Ground hyperEdges connected to the expanding container
   * SYMMETRIC INVERSE of _liftChildContainerHyperEdges()
   */
  _groundContainerHyperEdges(containerId) {
    this._processHyperEdges(
      (hyperEdge) => hyperEdge.source === containerId || hyperEdge.target === containerId,
      (hyperEdge) => this._groundSingleContainerHyperEdge(hyperEdge, containerId)
    );
  }

  /**
   * Ground a single container hyperEdge
   */
  _groundSingleContainerHyperEdge(hyperEdge, containerId) {
    const isSourceContainer = hyperEdge.source === containerId;
    const externalId = isSourceContainer ? hyperEdge.target : hyperEdge.source;
    const internalEndpoint = hyperEdge.originalInternalEndpoint;
    
    this._groundConnection(externalId, internalEndpoint, hyperEdge, isSourceContainer);
  }

  // ============ Helper Functions (Symmetric Pairs) ============

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
   * Ground a single connection during container expansion
   * SYMMETRIC INVERSE of _addToLiftedConnections()
   */
  _groundConnection(externalId, internalEndpoint, hyperEdge, isSourceContainer) {
    if (hyperEdge.originalEdges) {
      // Restore original edges only if both endpoints are visible
      for (const originalEdge of hyperEdge.originalEdges) {
        const sourceNode = this.graphNodes.get(originalEdge.source);
        const targetNode = this.graphNodes.get(originalEdge.target);
        
        // Only restore edge if both endpoints are visible nodes
        if (sourceNode && !sourceNode.hidden && targetNode && !targetNode.hidden) {
          this.setEdgeHidden(originalEdge.id, false);
        }
      }
    }
  }

  /**
   * Create hyperEdges from lifted connections
   */
  _createHyperEdgesFromLiftedConnections(containerId, liftedConnections) {
    for (const [externalId, connections] of liftedConnections) {
      this._createDirectionalHyperEdges(containerId, externalId, connections);
    }
  }

  /**
   * Create hyperEdges for both directions (incoming and outgoing)
   */
  _createDirectionalHyperEdges(containerId, externalId, connections) {
    if (connections.incoming.size > 0) {
      this._createHyperEdge(externalId, containerId, Array.from(connections.incoming));
    }
    
    if (connections.outgoing.size > 0) {
      this._createHyperEdge(containerId, externalId, Array.from(connections.outgoing));
    }
  }

  /**
   * Create a single hyperEdge with proper metadata
   */
  _createHyperEdge(sourceId, targetId, edgesArray) {
    const hyperEdgeId = `${HYPER_EDGE_PREFIX}${sourceId}_to_${targetId}`;
    this.setHyperEdge(hyperEdgeId, {
      source: sourceId,
      target: targetId,
      style: this._aggregateEdgeStyles(edgesArray),
      originalEdges: edgesArray.map(e => ({ id: e.id, source: e.source, target: e.target, style: e.style })),
      originalInternalEndpoint: edgesArray[0].originalInternalEndpoint || 
        this._findOriginalInternalEndpoint(edgesArray, targetId === sourceId ? sourceId : targetId)
    });
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



  // ============ Private Helper Methods ============
  
  /**
   * Check if an endpoint (node or container) is visible and should be connected to
   */
  _isEndpointConnectable(endpointId) {
    // Check if endpoint is a visible node
    const node = this.graphNodes.get(endpointId);
    if (node && !node.hidden) {
      return true;
    }
    
    // Check if endpoint is a visible, collapsed container
    const container = this.containers.get(endpointId);
    if (container && !container.hidden && container.collapsed) {
      return true;
    }
    
    return false;
  }

  /**
   * Categorize children into nodes and containers
   */
  _categorizeChildren(children) {
    const containerNodes = new Set();
    const childContainers = new Set();
    
    for (const childId of children) {
      if (this.graphNodes.has(childId)) {
        containerNodes.add(childId);
      } else if (this.containers.has(childId)) {
        childContainers.add(childId);
      }
    }
    
    return { containerNodes, childContainers };
  }

  /**
   * Apply visibility changes to a set of nodes
   */
  _setNodesVisibility(nodeIds, hidden) {
    for (const nodeId of nodeIds) {
      this.setNodeHidden(nodeId, hidden);
    }
  }

  /**
   * Process hyperEdges by predicate and apply update function
   */
  _processHyperEdges(predicate, updateFn) {
    const hyperEdgesToRemove = [];
    
    for (const [hyperEdgeId, hyperEdge] of this.hyperEdges) {
      if (predicate(hyperEdge)) {
        hyperEdgesToRemove.push(hyperEdgeId);
        if (updateFn) {
          updateFn(hyperEdge, hyperEdgeId);
        }
      }
    }
    
    // Remove processed hyperEdges
    for (const hyperEdgeId of hyperEdgesToRemove) {
      this.removeHyperEdge(hyperEdgeId);
    }
  }

  /**
   * Generic visibility update method - consolidates _updateVisibleNodes, _updateVisibleEdges, _updateVisibleContainers
   */
  _updateVisibilityMap(visibilityMap, id, entity) {
    if (entity.hidden) {
      visibilityMap.delete(id);
    } else {
      visibilityMap.set(id, entity);
    }
  }

  _updateVisibleNodes(id, node) {
    this._updateVisibilityMap(this.visibleNodes, id, node);
  }

  _updateVisibleEdges(id, edge) {
    this._updateVisibilityMap(this.visibleEdges, id, edge);
  }

  _updateVisibleContainers(id, container) {
    this._updateVisibilityMap(this.visibleContainers, id, container);
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
