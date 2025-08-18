/**
 * Visualization State - Core Data Structure
 * 
 * Maintains the mutable state of the visualization including nodes, edges, containers, and hyperEdges.
 * Provides efficient access to visible/non-hidden elements through Maps and collections.
 */

import {
  NODE_STYLES,
  EDGE_STYLES, 
  CONTAINER_STYLES
} from './shared/constants';

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
 * Core visualization state class that manages all graph elements including nodes, edges, 
 * containers, and hyperEdges with efficient visibility tracking and hierarchy management.
 * 
 * Features:
 * - O(1) element lookups using Maps
 * - Automatic visibility management
 * - Hierarchical container support with collapse/expand
 * - Edge <-> hyperEdge conversion for collapse/expand
 * - Efficient update operations
 * 
 * @class VisualizationState
 * @example
 * ```javascript
 * const state = new VisualizationState();
 * 
 * // Add nodes
 * state.setGraphNode('n1', { label: 'Node 1' });
 * state.setGraphNode('n2', { label: 'Node 2' });
 * 
 * // Add edges
 * state.setGraphEdge('e1', { source: 'n1', target: 'n2' });
 * 
 * // Create container
 * state.setContainer('c1', { children: ['n1', 'n2'] });
 * 
 * // Collapse container (automatically creates hyperEdges)
 * state.collapseContainer('c1');
 * ```
 */
export class VisualizationState {
  // Core graph elements
  private readonly graphNodes: Map<string, any>;
  private readonly graphEdges: Map<string, any>;
  private readonly containers: Map<string, any>;
  private readonly hyperEdges: Map<string, any>;
  
  // Efficient access collections for visible elements
  private readonly visibleNodes: Map<string, any>;
  private readonly visibleEdges: Map<string, any>;
  private readonly visibleContainers: Map<string, any>;
  private readonly expandedContainers: Map<string, any>;
  
  // Collapsed container representations
  private readonly collapsedContainers: Map<string, any>;
  
  // Container hierarchy tracking
  private readonly containerChildren: Map<string, Set<string>>;
  private readonly nodeContainers: Map<string, string>;
  
  // Edge tracking for hyperEdge management
  private readonly nodeToEdges: Map<string, Set<string>>;

  /**
   * Create a new VisualizationState instance
   * @constructor
   */
  constructor() {
    // Core graph elements
    /** @type {Map<string, Object>} Map of node ID to GraphNode objects */
    this.graphNodes = new Map(); 
    /** @type {Map<string, Object>} Map of edge ID to GraphEdge objects */
    this.graphEdges = new Map(); 
    /** @type {Map<string, Object>} Map of container ID to Container objects */
    this.containers = new Map(); 
    /** @type {Map<string, Object>} Map of hyperEdge ID to HyperEdge objects */
    this.hyperEdges = new Map(); 
    
    // Efficient access collections for visible elements
    /** @type {Map<string, Object>} Non-hidden nodes for rendering */
    this.visibleNodes = new Map(); 
    /** @type {Map<string, Object>} Non-hidden edges for rendering */
    this.visibleEdges = new Map(); 
    /** @type {Map<string, Object>} Non-hidden containers for rendering */
    this.visibleContainers = new Map(); 
    /** @type {Map<string, Object>} Non-collapsed containers */
    this.expandedContainers = new Map(); 
    
    // Collapsed container representations
    /** @type {Map<string, Object>} Collapsed container representations */
    this.collapsedContainers = new Map(); 
    
    // Container hierarchy tracking
    /** @type {Map<string, Set<string>>} Container ID to Set of child IDs */
    this.containerChildren = new Map(); 
    /** @type {Map<string, string>} Node ID to parent container ID */
    this.nodeContainers = new Map(); 
    
    // Edge tracking for hyperEdge management
    /** @type {Map<string, Set<string>>} Node ID to Set of connected edge IDs */
    this.nodeToEdges = new Map(); 
  }

  // ============ Generic Entity Management ============

  /**
   * Validate that an entity exists and optionally check a condition
   * @param {string} entityType - The type of entity being validated
   * @param {string} id - The ID of the entity
   * @param {Object|null} entity - The entity object to validate
   * @param {string} operation - The operation being attempted
   * @param {Function} [conditionFn] - Optional condition function to check
   * @throws {Error} When entity doesn't exist or condition fails
   */
  _validateEntity(entityType, id, entity, operation, conditionFn = null) {
    if (!entity) {
      throw new Error(`Cannot ${operation}: ${entityType} '${id}' does not exist`);
    }
    if (conditionFn && !conditionFn(entity)) {
      throw new Error(`Cannot ${operation}: ${entityType} '${id}' does not support this operation`);
    }
    return true;
  }

  /**
   * Validate that an entity exists and optionally check a condition (non-throwing version)
   * @param {Object|null} entity - The entity object to validate
   * @param {Function} [conditionFn] - Optional condition function to check
   * @returns {boolean} True if entity exists and passes condition
   */
  _validateEntitySafe(entity, conditionFn = null) {
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
   * @param {string} entityType - The type of entity
   * @param {string} id - The entity ID
   * @param {boolean} hidden - Whether the entity should be hidden
   * @throws {Error} When entity doesn't exist or doesn't support hiding
   */
  _setEntityHidden(entityType, id, hidden) {
    const entity = this._getEntity(entityType, id);
    this._validateEntity(entityType, id, entity, 'set hidden flag', e => 'hidden' in e);
    entity.hidden = hidden;
    this._updateVisibilityCollection(entityType, id, entity);
  }

  /**
   * Generic method to get hidden flag for any entity type that supports it
   * @param {string} entityType - The type of entity
   * @param {string} id - The entity ID
   * @returns {boolean|undefined} The hidden flag or undefined if entity doesn't exist
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
   * @param {string} id - Unique identifier for the node
   * @param {Object} props - Node properties
   * @param {string} props.label - Display label for the node
   * @param {string} [props.style=NODE_STYLES.DEFAULT] - Visual style identifier
   * @param {boolean} [props.hidden=false] - Whether the node is hidden
   * @param {Object} [props.otherProps] - Additional custom properties
   * @returns {Object} The created/updated node object
   * @throws {Error} When required properties are missing
   * @example
   * ```javascript
   * const node = state.setGraphNode('node1', {
   *   label: 'My Node',
   *   style: NODE_STYLES.HIGHLIGHTED,
   *   customData: { type: 'processor' }
   * });
   * ```
   */
  setGraphNode(id: string, { label, style = NODE_STYLES.DEFAULT as any, hidden = false, ...otherProps }: any) {
    if (!id || typeof id !== 'string') {
      throw new Error('Node ID must be a non-empty string');
    }
    if (!label || typeof label !== 'string') {
      throw new Error('Node label must be a non-empty string');
    }

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
   * @param {string} id - The node ID to retrieve
   * @returns {Object|undefined} The node object or undefined if not found
   */
  getGraphNode(id) {
    return this._getEntity(ENTITY_TYPES.NODE, id);
  }

  /**
   * Set hidden flag for a graph node
   * @param {string} id - The node ID
   * @param {boolean} hidden - Whether the node should be hidden
   * @throws {Error} When node doesn't exist
   */
  setNodeHidden(id, hidden) {
    this._setEntityHidden(ENTITY_TYPES.NODE, id, hidden);
  }

  /**
   * Get hidden flag for a graph node
   * @param {string} id - The node ID
   * @returns {boolean|undefined} The hidden flag or undefined if node doesn't exist
   */
  getNodeHidden(id) {
    return this._getEntityHidden(ENTITY_TYPES.NODE, id);
  }

  /**
   * Remove a graph node
   * @param {string} id - The node ID to remove
   * @throws {Error} When node doesn't exist
   */
  removeGraphNode(id) {
    if (!this.graphNodes.has(id)) {
      throw new Error(`Cannot remove node: node '${id}' does not exist`);
    }
    this.graphNodes.delete(id);
    this.visibleNodes.delete(id);
    this.nodeContainers.delete(id);
  }

  // ============ Graph Edges ============
  
  /**
   * Add or update a graph edge
   */
  setGraphEdge(id: string, { source, target, style = EDGE_STYLES.DEFAULT as any, hidden = false, ...otherProps }: any) {
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
   * @param {string} id - Unique identifier for the container
   * @param {Object} props - Container properties
   * @param {Object} [props.expandedDimensions={width: 0, height: 0}] - Dimensions when expanded
   * @param {number} props.expandedDimensions.width - Width in pixels
   * @param {number} props.expandedDimensions.height - Height in pixels
   * @param {boolean} [props.collapsed=false] - Whether the container is collapsed
   * @param {boolean} [props.hidden=false] - Whether the container is hidden
   * @param {Array<string>} [props.children=[]] - Array of child node/container IDs
   * @param {Object} [props.otherProps] - Additional custom properties
   * @returns {Object} The created/updated container object
   * @throws {Error} When required properties are missing or invalid
   * @example
   * ```javascript
   * const container = state.setContainer('container1', {
   *   expandedDimensions: { width: 200, height: 150 },
   *   children: ['node1', 'node2'],
   *   label: 'My Container'
   * });
   * ```
   */
  setContainer(id, { 
    expandedDimensions = { width: 0, height: 0 }, 
    collapsed = false, 
    hidden = false,
    children = [],
    ...otherProps 
  }) {
    if (!id || typeof id !== 'string') {
      throw new Error('Container ID must be a non-empty string');
    }
    if (!Array.isArray(children)) {
      throw new Error('Container children must be an array');
    }

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
   * @param {string} id - The container ID to retrieve
   * @returns {Object|undefined} The container object or undefined if not found
   */
  getContainer(id) {
    return this._getEntity(ENTITY_TYPES.CONTAINER, id);
  }

  /**
   * Set collapsed flag for a container
   * @param {string} id - The container ID
   * @param {boolean} collapsed - Whether the container should be collapsed
   * @throws {Error} When container doesn't exist
   */
  setContainerCollapsed(id, collapsed) {
    const container = this.getContainer(id);
    this._validateEntity(ENTITY_TYPES.CONTAINER, id, container, 'set collapsed flag');
    container.collapsed = collapsed;
    this._updateExpandedContainers(id, container);
  }

  /**
   * Get collapsed flag for a container
   * @param {string} id - The container ID
   * @returns {boolean|undefined} The collapsed flag or undefined if container doesn't exist
   */
  getContainerCollapsed(id) {
    const container = this.getContainer(id);
    return container ? container.collapsed : undefined;
  }

  /**
   * Set hidden flag for a container
   * @param {string} id - The container ID
   * @param {boolean} hidden - Whether the container should be hidden
   * @throws {Error} When container doesn't exist
   */
  setContainerHidden(id, hidden) {
    this._setEntityHidden(ENTITY_TYPES.CONTAINER, id, hidden);
  }

  /**
   * Get hidden flag for a container
   * @param {string} id - The container ID
   * @returns {boolean|undefined} The hidden flag or undefined if container doesn't exist
   */
  getContainerHidden(id) {
    return this._getEntityHidden(ENTITY_TYPES.CONTAINER, id);
  }

  /**
   * Add a child to a container
   * @param {string} containerId - The container ID
   * @param {string} childId - The child node/container ID to add
   * @throws {Error} When container doesn't exist
   */
  addContainerChild(containerId, childId) {
    const container = this.getContainer(containerId);
    this._validateEntity(ENTITY_TYPES.CONTAINER, containerId, container, 'add child');
    container.children.add(childId);
    this.containerChildren.set(containerId, container.children);
    this.nodeContainers.set(childId, containerId);
  }

  /**
   * Remove a child from a container
   * @param {string} containerId - The container ID
   * @param {string} childId - The child node/container ID to remove
   * @throws {Error} When container doesn't exist
   */
  removeContainerChild(containerId, childId) {
    const container = this.getContainer(containerId);
    this._validateEntity(ENTITY_TYPES.CONTAINER, containerId, container, 'remove child');
    container.children.delete(childId);
    this.containerChildren.set(containerId, container.children);
    this.nodeContainers.delete(childId);
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
    // Allow collapsing containers even if they're hidden by parent containers
    // Just check that the container exists and is not already explicitly collapsed
    if (!container) {
      throw new Error(`Cannot collapse container: container '${containerId}' does not exist`);
    }
    if (container.collapsed) {
      return; // Already collapsed
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
    // Allow expanding containers even if they're hidden by parent containers
    // Just check that the container exists and is currently collapsed
    if (!container) {
      throw new Error(`Cannot expand container: container '${containerId}' does not exist`);
    }
    if (!container.collapsed) {
      return; // Already expanded
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
    const containerEndpoints = Array.from(internalEndpoints).filter(id => this.containers.has(id as string));
    const nodeEndpoints = Array.from(internalEndpoints).filter(id => this.graphNodes.has(id as string));
    
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
 * Factory function to create a new VisualizationState instance.
 * Preferred over direct constructor usage for consistency and potential future initialization logic.
 * 
 * @function createVisualizationState
 * @returns {VisualizationState} A new, empty visualization state instance
 * @example
 * ```javascript
 * // Preferred approach
 * const state = createVisualizationState();
 * 
 * // Instead of direct constructor
 * // const state = new VisualizationState(); // works but not recommended
 * 
 * // Add some data
 * state.setGraphNode('node1', { label: 'My First Node' });
 * console.log(state.getGraphNode('node1')); // { id: 'node1', label: 'My First Node', ... }
 * ```
 */
export function createVisualizationState() {
  return new VisualizationState();
}
