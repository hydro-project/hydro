/**
 * Visualization State - Core Data Structure
 * 
 * Maintains the mutable state of the visualization including nodes, edges, containers, and hyperEdges.
 * Provides efficient access to visible/non-hidden elements through Maps and collections.
 */

import {
  NODE_STYLES,
  EDGE_STYLES, 
  CONTAINER_STYLES,
  CreateNodeProps,
  CreateEdgeProps,
  CreateContainerProps
} from './constants.js';
import { ContainerCollapseExpandEngine } from './ContainerCollapseExpand.js';

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
<<<<<<< HEAD
 * Core visualization state class that manages all graph elements
 */
export class VisualizationState {
<<<<<<< HEAD:docs/src/components/vis/VisState.js
=======
=======
 * Read-only interface for container hierarchy information
 * Prevents external code from modifying the internal structure
 */
export interface ContainerHierarchyView {
  getContainerChildren(containerId: string): ReadonlySet<string>;
  getNodeContainer(nodeId: string): string | undefined;
}

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
 * - Runtime-enforced encapsulation for container hierarchy
 * 
 * @class VisualizationState
 * @example
 * ```javascript
 * // Modern idiomatic usage with getters
 * const state = createVisualizationState()
 *   .setGraphNode('n1', { label: 'Node 1' })
 *   .setGraphNode('n2', { label: 'Node 2' })
 *   .setGraphEdge('e1', { source: 'n1', target: 'n2' })
 *   .setContainer('c1', { children: ['n1', 'n2'] });
 * 
 * // Access data with TypeScript getters (no parentheses!)
 * console.log(state.visibleNodes);     // Array of visible nodes
 * console.log(state.expandedContainers); // Array of expanded containers
 * 
 * // Update properties idiomatically  
 * state.updateNode('n1', { hidden: true, style: 'highlighted' });
 * state.updateContainer('c1', { collapsed: true });
 * ```
 */
export class VisualizationState implements ContainerHierarchyView {
>>>>>>> 336632b29 (code review of VisState)
  // Core graph elements
  private readonly graphNodes: Map<string, any>;
  private readonly graphEdges: Map<string, any>;
  private readonly containers: Map<string, any>;
  private readonly hyperEdges: Map<string, any>;
  
  // Efficient access collections for visible elements (internal maps)
  private readonly _visibleNodes: Map<string, any>;
  private readonly _visibleEdges: Map<string, any>;
  private readonly _visibleContainers: Map<string, any>;
  private readonly _expandedContainers: Map<string, any>;
  
  // Collapsed container representations
  private readonly collapsedContainers: Map<string, any>;
  
  // Container hierarchy tracking (truly private with # syntax)
  readonly #containerChildren: Map<string, Set<string>>;
  readonly #nodeContainers: Map<string, string>;
  
  // Edge tracking for hyperEdge management
  private readonly nodeToEdges: Map<string, Set<string>>;

  // Container collapse/expand engine
  private readonly collapseExpandEngine: ContainerCollapseExpandEngine;

  /**
   * Create a new VisualizationState instance
   * @constructor
   */
>>>>>>> bddb2f97e (typescript port):docs/src/components/vis/VisState.ts
  constructor() {
    // Core graph elements
    this.graphNodes = new Map(); // id -> GraphNode
    this.graphEdges = new Map(); // id -> GraphEdge
    this.containers = new Map(); // id -> Container
    this.hyperEdges = new Map(); // id -> HyperEdge
    
    // Efficient access collections for visible elements
<<<<<<< HEAD
    this.visibleNodes = new Map(); // id -> GraphNode (non-hidden)
    this.visibleEdges = new Map(); // id -> GraphEdge (non-hidden)
    this.visibleContainers = new Map(); // id -> Container (non-hidden)
    this.expandedContainers = new Map(); // id -> Container (non-collapsed)
=======
    /** @type {Map<string, Object>} Non-hidden nodes for rendering */
    this._visibleNodes = new Map(); 
    /** @type {Map<string, Object>} Non-hidden edges for rendering */
    this._visibleEdges = new Map(); 
    /** @type {Map<string, Object>} Non-hidden containers for rendering */
    this._visibleContainers = new Map(); 
    /** @type {Map<string, Object>} Non-collapsed containers */
    this._expandedContainers = new Map(); 
>>>>>>> 336632b29 (code review of VisState)
    
    // Collapsed container representations
    this.collapsedContainers = new Map(); // id -> CollapsedContainer
    
    // Container hierarchy tracking
<<<<<<< HEAD
    this.containerChildren = new Map(); // containerId -> Set of child node/container ids
    this.nodeContainers = new Map(); // nodeId -> containerId
    
    // Edge tracking for hyperEdge management
    this.nodeToEdges = new Map(); // nodeId -> Set of edge ids connected to this node
=======
    /** @type {Map<string, Set<string>>} Container ID to Set of child IDs */
    this.#containerChildren = new Map(); 
    /** @type {Map<string, string>} Node ID to parent container ID */
    this.#nodeContainers = new Map(); 
    
    // Edge tracking for hyperEdge management
    /** @type {Map<string, Set<string>>} Node ID to Set of connected edge IDs */
    this.nodeToEdges = new Map(); 

    // Initialize container collapse/expand engine
    this.collapseExpandEngine = new ContainerCollapseExpandEngine(this);
>>>>>>> 336632b29 (code review of VisState)
  }

  // ============ Generic Entity Management ============

  /**
   * Validate that an entity exists and optionally check a condition
<<<<<<< HEAD
   */
  _validateEntity(entity, conditionFn = null) {
    if (!entity) return false;
    return conditionFn ? conditionFn(entity) : true;
=======
   * @param {Object|null} entity - The entity object to validate
   * @param {Function} [conditionFn] - Optional condition function to check
   * @throws {Error} When entity doesn't exist or condition fails
   */
  _validateEntity(entity, conditionFn = null) {
    if (!entity) {
      throw new Error(`Entity does not exist`);
    }
    if (conditionFn && !conditionFn(entity)) {
      throw new Error(`Entity '${entity.id}' does not support this operation`);
    }
    return true;
  }

  /**
   * Validate required string parameter
   * @param {any} value - The value to validate
   * @param {string} fieldName - Name of the field for error messages
   * @throws {Error} When value is not a non-empty string
   */
  _validateRequiredString(value, fieldName) {
    if (!value || typeof value !== 'string') {
      throw new Error(`${fieldName} must be a non-empty string`);
    }
  }

  /**
   * Validate style parameter against allowed values
   * @param {any} style - The style value to validate
   * @param {Object} allowedStyles - Object containing allowed style values
   * @param {string} entityType - Type of entity for error messages
   * @throws {Error} When style is not in allowed values
   */
  _validateStyle(style, allowedStyles, entityType) {
    const validStyles = Object.values(allowedStyles);
    if (!validStyles.includes(style)) {
      throw new Error(`${entityType} style must be one of: ${validStyles.join(', ')}`);
    }
>>>>>>> 336632b29 (code review of VisState)
  }

  /**
   * Generic method to get an entity from any collection
   */
  _getEntity(entityType: string, id: string): any {
    const collection = this._getEntityCollection(entityType);
    return collection.get(id);
  }

  /**
<<<<<<< HEAD
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
=======
>>>>>>> 336632b29 (code review of VisState)
   * Get the main collection for an entity type
   */
  _getEntityCollection(entityType: string): Map<string, any> {
    switch (entityType) {
      case ENTITY_TYPES.NODE: return this.graphNodes;
      case ENTITY_TYPES.EDGE: return this.graphEdges;
      case ENTITY_TYPES.CONTAINER: return this.containers;
      case ENTITY_TYPES.HYPER_EDGE: return this.hyperEdges;
      default: throw new Error(`Unknown entity type: ${entityType}`);
    }
  }

  // ============ Graph Nodes ============
  
  /**
   * Add or update a graph node
<<<<<<< HEAD
   */
<<<<<<< HEAD:docs/src/components/vis/VisState.js
  setGraphNode(id, { label, style = NODE_STYLES.DEFAULT, hidden = false, ...otherProps }) {
=======
  setGraphNode(id: string, { label, style = NODE_STYLES.DEFAULT as any, hidden = false, ...otherProps }: any) {
    if (!id || typeof id !== 'string') {
      throw new Error('Node ID must be a non-empty string');
    }
    if (!label || typeof label !== 'string') {
      throw new Error('Node label must be a non-empty string');
    }
=======
   * @param {string} id - Unique identifier for the node
   * @param {Object} props - Node properties
   * @param {string} props.label - Display label for the node
   * @param {string} [props.style=NODE_STYLES.DEFAULT] - Visual style identifier
   * @param {boolean} [props.hidden=false] - Whether the node is hidden
   * @param {Object} [props.otherProps] - Additional custom properties
   * @returns {VisualizationState} This instance for method chaining
   * @throws {Error} When required properties are missing
   * @example
   * ```javascript
   * const state = createVisualizationState()
   *   .setGraphNode('node1', {
   *     label: 'My Node',
   *     style: NODE_STYLES.HIGHLIGHTED,
   *     customData: { type: 'processor' }
   *   })
   *   .setGraphNode('node2', { label: 'Another Node' });
   * ```
   */
  setGraphNode(id: string, props: CreateNodeProps) {
    const { label, style = NODE_STYLES.DEFAULT, hidden = false, ...otherProps } = props;
    
    this._validateRequiredString(id, 'Node ID');
    this._validateRequiredString(label, 'Node label');
    this._validateStyle(style, NODE_STYLES, 'Node');
>>>>>>> 336632b29 (code review of VisState)

>>>>>>> bddb2f97e (typescript port):docs/src/components/vis/VisState.ts
    const node = {
      id,
      label,
      style,
      hidden,
      ...otherProps
    };
    
    this._addNodeToAllStructures(id, node);
    return this;
  }

  /**
   * Get a graph node by id
   */
  getGraphNode(id: string): any {
    return this._getEntity(ENTITY_TYPES.NODE, id);
  }

  /**
<<<<<<< HEAD
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
=======
   * Update a node's properties. More idiomatic than separate getters/setters.
   * @param {string} id - The node ID
   * @param {Partial<{hidden: boolean, style: string, label: string}>} updates - Properties to update
   * @throws {Error} When node doesn't exist
   * @example
   * ```javascript
   * state.updateNode('node1', { hidden: true, style: 'highlighted' });
   * ```
   */
  updateNode(id: string, updates: { hidden?: boolean; style?: string; label?: string }) {
    const node = this.getGraphNode(id);
    this._validateEntity(node);
    
    // Apply updates
    Object.assign(node, updates);
    
    // Update visibility if hidden changed
    if ('hidden' in updates) {
      this._updateVisibleNodes(id, node);
    }
    
    return this;
>>>>>>> 336632b29 (code review of VisState)
  }

  /**
   * Remove a graph node
   */
<<<<<<< HEAD
  removeGraphNode(id) {
    this.graphNodes.delete(id);
    this.visibleNodes.delete(id);
    this.nodeContainers.delete(id);
=======
  removeGraphNode(id: string): void {
    if (!this.graphNodes.has(id)) {
      throw new Error(`Cannot remove node: node '${id}' does not exist`);
    }
    this._removeNodeFromAllStructures(id);
>>>>>>> 336632b29 (code review of VisState)
  }

  // ============ Graph Edges ============
  
  /**
   * Add or update a graph edge
   * @param {string} id - Unique identifier for the edge
   * @param {Object} props - Edge properties
   * @param {string} props.source - Source node/container ID
   * @param {string} props.target - Target node/container ID
   * @param {string} [props.style=EDGE_STYLES.DEFAULT] - Visual style identifier
   * @param {boolean} [props.hidden=false] - Whether the edge is hidden
   * @param {Object} [props.otherProps] - Additional custom properties
   * @returns {VisualizationState} This instance for method chaining
   * @throws {Error} When required properties are missing or invalid
   * @example
   * ```javascript
   * const state = createVisualizationState()
   *   .setGraphEdge('edge1', {
   *     source: 'node1',
   *     target: 'node2',
   *     style: EDGE_STYLES.HIGHLIGHTED,
   *     weight: 5
   *   })
   *   .setGraphEdge('edge2', { source: 'node2', target: 'node3' });
   * ```
   */
  setGraphEdge(id: string, props: CreateEdgeProps) {
    const { source, target, style = EDGE_STYLES.DEFAULT, hidden = false, ...otherProps } = props;
    
    this._validateRequiredString(id, 'Edge ID');
    this._validateRequiredString(source, 'Edge source');
    this._validateRequiredString(target, 'Edge target');
    this._validateStyle(style, EDGE_STYLES, 'Edge');

    const edge = {
      id,
      source,
      target,
      style,
      hidden,
      ...otherProps
    };
    
    this._addEdgeToAllStructures(id, edge, source, target);
    return this;
  }

  /**
   * Get a graph edge by id
   * @param {string} id - The edge ID to retrieve
   * @returns {Object|undefined} The edge object or undefined if not found
   */
  getGraphEdge(id: string): any {
    return this._getEntity(ENTITY_TYPES.EDGE, id);
  }

  /**
   * Update an edge's properties. More idiomatic than separate getters/setters.
   * @param {string} id - The edge ID
   * @param {Partial<{hidden: boolean, style: string}>} updates - Properties to update
   * @throws {Error} When edge doesn't exist
   * @example
   * ```javascript
   * state.updateEdge('edge1', { hidden: true, style: 'highlighted' });
   * ```
   */
  updateEdge(id: string, updates: { hidden?: boolean; style?: string }) {
    const edge = this.getGraphEdge(id);
    this._validateEntity(edge);
    
    // Apply updates
    Object.assign(edge, updates);
    
    // Update visibility if hidden changed
    if ('hidden' in updates) {
      this._updateVisibleEdges(id, edge);
    }
    
    return this;
  }

  /**
   * Remove a graph edge
   * @param {string} id - The edge ID to remove
   * @throws {Error} When edge doesn't exist
   */
  removeGraphEdge(id: string): void {
    if (!this.graphEdges.has(id)) {
      throw new Error(`Cannot remove edge: edge '${id}' does not exist`);
    }
    this._removeEdgeFromAllStructures(id);
  }

  // ============ Containers ============
  
  /**
   * Add or update a container
<<<<<<< HEAD
   */
  setContainer(id, { 
    expandedDimensions = { width: 0, height: 0 }, 
    collapsed = false, 
    hidden = false,
    children = [],
    ...otherProps 
  }) {
=======
   * @param {string} id - Unique identifier for the container
   * @param {Object} props - Container properties
   * @param {Object} [props.expandedDimensions={width: 0, height: 0}] - Dimensions when expanded
   * @param {number} props.expandedDimensions.width - Width in pixels
   * @param {number} props.expandedDimensions.height - Height in pixels
   * @param {boolean} [props.collapsed=false] - Whether the container is collapsed
   * @param {boolean} [props.hidden=false] - Whether the container is hidden
   * @param {Array<string>} [props.children=[]] - Array of child node/container IDs
   * @param {string} [props.label] - Display label for the container
   * @param {Object} [props.otherProps] - Additional custom properties
   * @returns {VisualizationState} This instance for method chaining
   * @throws {Error} When required properties are missing or invalid
   * @example
   * ```javascript
   * const state = createVisualizationState()
   *   .setContainer('container1', {
   *     expandedDimensions: { width: 200, height: 150 },
   *     children: ['node1', 'node2'],
   *     label: 'My Container'
   *   });
   * ```
   */
  setContainer(id: string, props: CreateContainerProps) {
    const { 
      expandedDimensions = { width: 0, height: 0 }, 
      collapsed = false, 
      hidden = false,
      children = [],
      ...otherProps 
    } = props;

    this._validateRequiredString(id, 'Container ID');
    
    if (!Array.isArray(children)) {
      throw new Error('Container children must be an array');
    }

>>>>>>> 336632b29 (code review of VisState)
    const container = {
      id,
      expandedDimensions,
      collapsed,
      hidden,
      children: new Set(children),
      ...otherProps
    };
    
    this._addContainerToAllStructures(id, container);
    return this;
  }

  /**
   * Get a container by id
   */
  getContainer(id: string): any {
    return this._getEntity(ENTITY_TYPES.CONTAINER, id);
  }

  /**
<<<<<<< HEAD
   * Set collapsed flag for a container
=======
   * Update a container's properties. More idiomatic than separate getters/setters.
   * @param {string} id - The container ID
   * @param {Partial<{collapsed: boolean, hidden: boolean, label: string}>} updates - Properties to update
   * @throws {Error} When container doesn't exist or parameters are invalid
   * @example
   * ```javascript
   * state.updateContainer('container1', { collapsed: true, hidden: false });
   * ```
>>>>>>> 336632b29 (code review of VisState)
   */
  updateContainer(id: string, updates: { collapsed?: boolean; hidden?: boolean; label?: string }) {
    this._validateRequiredString(id, 'Container ID');
    
    const container = this.getContainer(id);
<<<<<<< HEAD
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
=======
    this._validateEntity(container);
    
    // Apply updates
    Object.assign(container, updates);
    
    // Update expanded containers if collapsed changed
    if ('collapsed' in updates) {
      this._updateExpandedContainers(id, container);
    }
    
    // Update visibility if hidden changed  
    if ('hidden' in updates) {
      this._updateVisibleContainers(id, container);
    }
    
    return this;
>>>>>>> 336632b29 (code review of VisState)
  }

  /**
   * Add a child to a container
<<<<<<< HEAD
=======
   * @param {string} containerId - The container ID
   * @param {string} childId - The child node/container ID to add
   * @throws {Error} When container doesn't exist or parameters are invalid
>>>>>>> 336632b29 (code review of VisState)
   */
  addContainerChild(containerId: string, childId: string): void {
    this._validateRequiredString(containerId, 'Container ID');
    this._validateRequiredString(childId, 'Child ID');
    
    const container = this.getContainer(containerId);
<<<<<<< HEAD
    if (this._validateEntity(container)) {
      container.children.add(childId);
      this.containerChildren.set(containerId, container.children);
      this.nodeContainers.set(childId, containerId);
    }
=======
    this._validateEntity(container);
    
    // Validate tree hierarchy (no cycles/DAGs)
    this.collapseExpandEngine.validateTreeHierarchy(containerId, childId);
    
    // Use helper to maintain consistency
    this._addChildToContainerHierarchy(containerId, childId);
    
    // Update edge index
    this.collapseExpandEngine.rebuildEdgeIndex();
>>>>>>> 336632b29 (code review of VisState)
  }

  /**
   * Remove a child from a container
<<<<<<< HEAD
=======
   * @param {string} containerId - The container ID
   * @param {string} childId - The child node/container ID to remove
   * @throws {Error} When container doesn't exist or parameters are invalid
>>>>>>> 336632b29 (code review of VisState)
   */
  removeContainerChild(containerId: string, childId: string): void {
    this._validateRequiredString(containerId, 'Container ID');
    this._validateRequiredString(childId, 'Child ID');
    
    const container = this.getContainer(containerId);
<<<<<<< HEAD
    if (this._validateEntity(container)) {
      container.children.delete(childId);
      this.containerChildren.set(containerId, container.children);
      this.nodeContainers.delete(childId);
    }
=======
    this._validateEntity(container);
    
    // Use helper to maintain consistency
    this._removeChildFromContainerHierarchy(containerId, childId);
    
    // Update edge index
    this.collapseExpandEngine.rebuildEdgeIndex();
>>>>>>> 336632b29 (code review of VisState)
  }

  /**
   * Remove a container
   * @param {string} id - The container ID to remove
   * @throws {Error} When container doesn't exist
   */
  removeContainer(id: string): void {
    if (!this.containers.has(id)) {
      throw new Error(`Cannot remove container: container '${id}' does not exist`);
    }
    this._removeContainerFromAllStructures(id);
  }

  // ============ Hyper Edges ============
  
  /**
   * Add or update a hyper edge
   * @param {string} id - Unique identifier for the hyperEdge
   * @param {Object} props - HyperEdge properties
   * @param {string} props.source - Source node/container ID
   * @param {string} props.target - Target node/container ID
   * @param {string} [props.style=EDGE_STYLES.DEFAULT] - Visual style identifier
   * @param {Array<Object>} [props.originalEdges] - Original edges aggregated into this hyperEdge
   * @param {string} [props.originalInternalEndpoint] - Original internal endpoint for grounding
   * @param {Object} [props.otherProps] - Additional custom properties
   * @returns {VisualizationState} This instance for method chaining
   * @throws {Error} When required properties are missing or invalid
   * @example
   * ```javascript
   * const state = createVisualizationState()
   *   .setHyperEdge('hyper1', {
   *     source: 'container1',
   *     target: 'node3',
   *     style: EDGE_STYLES.THICK,
   *     originalEdges: [edge1, edge2]
   *   });
   * ```
   */
  setHyperEdge(id: string, { source, target, style = EDGE_STYLES.DEFAULT, ...otherProps }: { source: string; target: string; style?: string; [key: string]: any }) {
    this._validateRequiredString(id, 'HyperEdge ID');
    this._validateRequiredString(source, 'HyperEdge source');
    this._validateRequiredString(target, 'HyperEdge target');
    this._validateStyle(style, EDGE_STYLES, 'HyperEdge');

    const hyperEdge = {
      id,
      source,
      target,
      style,
      ...otherProps
    };
    
<<<<<<< HEAD
    this.hyperEdges.set(id, hyperEdge);
    
    // Check invariants after adding hyperEdge
    this._checkHyperEdgeInvariants();
    
    return hyperEdge;
=======
    this._addHyperEdgeToAllStructures(id, hyperEdge);
    return this;
>>>>>>> 336632b29 (code review of VisState)
  }

  /**
   * Get a hyper edge by id
   * @param {string} id - The hyperEdge ID to retrieve
   * @returns {Object|undefined} The hyperEdge object or undefined if not found
   */
  getHyperEdge(id: string): any {
    return this._getEntity(ENTITY_TYPES.HYPER_EDGE, id);
  }

  /**
   * Remove a hyper edge
   * @param {string} id - The hyperEdge ID to remove
   * @throws {Error} When hyperEdge doesn't exist
   */
  removeHyperEdge(id: string): void {
    if (!this.hyperEdges.has(id)) {
      throw new Error(`Cannot remove hyperEdge: hyperEdge '${id}' does not exist`);
    }
    this._removeHyperEdgeFromAllStructures(id);
  }

  // ============ Computed Properties (Idiomatic TypeScript Getters) ============
  
  /**
   * Get all visible (non-hidden) nodes
   */
  get visibleNodes() {
    return Array.from(this._visibleNodes.values());
  }

  /**
   * Get all visible (non-hidden) edges  
   */
  get visibleEdges() {
    return Array.from(this._visibleEdges.values());
  }

  /**
   * Get all visible (non-hidden) containers
   */
  get visibleContainers() {
    return Array.from(this._visibleContainers.values());
  }

  /**
   * Get all expanded (non-collapsed) containers
   */
  get expandedContainers() {
    return Array.from(this._expandedContainers.values());
  }

  /**
   * Get all hyper edges
   */
  get allHyperEdges() {
    return Array.from(this.hyperEdges.values());
  }

  /**
   * Get container children for a container id
   * Returns a readonly Set to prevent external modification
   */
  getContainerChildren(containerId: string): ReadonlySet<string> {
    return this.#containerChildren.get(containerId) || new Set();
  }

  /**
   * Get the container that contains a given node
   */
  getNodeContainer(nodeId: string): string | undefined {
    return this.#nodeContainers.get(nodeId);
  }

  /**
   * Clear all data
   */
  clear(): void {
    this._clearAllDataStructures();
  }

  // ============ Container Collapse/Expand Operations ============
  
  /**
   * Collapse a container (depth-first, bottom-up with edge lifting)
   * Uses optimized engine with tree hierarchy validation and edge indexing
   */
<<<<<<< HEAD
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
    
    // Check invariants after collapse operation
    this._checkHyperEdgeInvariants();
=======
  collapseContainer(containerId: string): void {
    this.collapseExpandEngine.collapseContainer(containerId);
>>>>>>> 336632b29 (code review of VisState)
  }
  
  /**
   * Expand a container (depth-first, top-down with edge grounding)
   * SYMMETRIC INVERSE of collapseContainer()
   */
<<<<<<< HEAD
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
    
    // Check invariants after expand operation
    this._checkHyperEdgeInvariants();
=======
  expandContainer(containerId: string): void {
    this.collapseExpandEngine.expandContainer(containerId);
>>>>>>> 336632b29 (code review of VisState)
  }

  // ============ Private Helper Methods ============
  
  // ============ Entity Creation Helpers ============
  
  /**
   * Add a node to all related data structures
   * @param {string} id - The node ID
   * @param {Object} node - The node object
   * @private
   */
  _addNodeToAllStructures(id: string, node: any): void {
    // Add to main collection
    this.graphNodes.set(id, node);
    
    // Update visibility collection
    this._updateVisibleNodes(id, node);
  }

  /**
   * Add an edge to all related data structures
   * @param {string} id - The edge ID
   * @param {Object} edge - The edge object
   * @param {string} source - The source node/container ID
   * @param {string} target - The target node/container ID
   * @private
   */
  _addEdgeToAllStructures(id: string, edge: any, source: string, target: string): void {
    // Add to main collection
    this.graphEdges.set(id, edge);
    
    // Update visibility collection
    this._updateVisibleEdges(id, edge);
    
    // Maintain nodeToEdges mapping
    this._addEdgeToNodeMapping(id, source, target);
    
    // Update collapse/expand engine edge index
    this.collapseExpandEngine.rebuildEdgeIndex();
  }

  /**
   * Add a container to all related data structures
   * @param {string} id - The container ID
   * @param {Object} container - The container object
   * @private
   */
  _addContainerToAllStructures(id: string, container: any): void {
    // Add to main collection
    this.containers.set(id, container);
    
    // Update visibility collections
    this._updateVisibleContainers(id, container);
    this._updateExpandedContainers(id, container);
    
    // Update container hierarchy using helper
    this._initializeContainerHierarchy(id, container.children);
    
    // Update collapse/expand engine edge index (containers affect edge routing)
    this.collapseExpandEngine.rebuildEdgeIndex();
  }

  /**
   * Add a hyperEdge to all related data structures
   * @param {string} id - The hyperEdge ID
   * @param {Object} hyperEdge - The hyperEdge object
   * @private
   */
  _addHyperEdgeToAllStructures(id, hyperEdge) {
    // Add to main collection (hyperEdges don't have other index structures)
    this.hyperEdges.set(id, hyperEdge);
  }

  // ============ Entity Removal Helpers ============
  
  /**
   * Remove a node from all related data structures
   * @param {string} id - The node ID to remove
   * @private
   */
  _removeNodeFromAllStructures(id) {
    // Remove from main collection
    this.graphNodes.delete(id);
    
    // Remove from visibility collection
    this._visibleNodes.delete(id);
    
    // Remove from container hierarchy using encapsulated method
    const parentId = this.#nodeContainers.get(id);
    if (parentId) {
      this._removeChildFromContainerHierarchy(parentId, id);
    }
    
    // Update collapse/expand engine edge index (node removal affects edge routing)
    this.collapseExpandEngine.rebuildEdgeIndex();
    
    // Note: nodeToEdges cleanup handled by edge removal
  }

  /**
   * Remove an edge from all related data structures
   * @param {string} id - The edge ID to remove
   * @private
   */
  _removeEdgeFromAllStructures(id) {
    const edge = this.graphEdges.get(id);
    
    // Remove from node-edge mapping first (needs edge data)
    if (edge) {
      this._removeEdgeFromNodeMapping(id, edge.source, edge.target);
    }
    
    // Remove from main collection
    this.graphEdges.delete(id);
    
    // Remove from visibility collection
    this._visibleEdges.delete(id);
    
    // Update collapse/expand engine edge index
    this.collapseExpandEngine.rebuildEdgeIndex();
  }

  /**
   * Remove a container from all related data structures
   * @param {string} id - The container ID to remove
   * @private
   */
  _removeContainerFromAllStructures(id) {
    // Clean up container hierarchy first (needs container data)
    this._cleanupContainerHierarchy(id);
    
    // Remove from main collection
    this.containers.delete(id);
    
    // Remove from visibility collections
    this._visibleContainers.delete(id);
    this._expandedContainers.delete(id);
    
    // Remove from collapsed representations if present
    this.collapsedContainers.delete(id);
    
    // Update collapse/expand engine edge index (containers affect edge routing)
    this.collapseExpandEngine.rebuildEdgeIndex();
  }

  /**
   * Remove a hyperEdge from all related data structures
   * @param {string} id - The hyperEdge ID to remove
   * @private
   */
  _removeHyperEdgeFromAllStructures(id) {
    // Remove from main collection (hyperEdges don't have other index structures)
    this.hyperEdges.delete(id);
  }

  // ============ Bulk Operations Helpers ============
  
  /**
   * Clear all data structures in the correct order
   * @private
   */
  _clearAllDataStructures() {
    // Clear main entity collections
    this.graphNodes.clear();
    this.graphEdges.clear();
    this.containers.clear();
    this.hyperEdges.clear();
    
    // Clear visibility collections
    this._visibleNodes.clear();
    this._visibleEdges.clear();
    this._visibleContainers.clear();
    this._expandedContainers.clear();
    
    // Clear specialized collections
    this.collapsedContainers.clear();
    
    // Clear index structures
    this.#containerChildren.clear();
    this.#nodeContainers.clear();
    this.nodeToEdges.clear();
  }

  // ============ Container Hierarchy Helpers ============
  
  /**
   * Add a child to container hierarchy and maintain all indexes
   * @param {string} containerId - The container ID
   * @param {string} childId - The child node/container ID to add
   * @private
   */
  _addChildToContainerHierarchy(containerId, childId) {
    const container = this.containers.get(containerId);
    if (container) {
      container.children.add(childId);
      this.#containerChildren.set(containerId, container.children);
      this.#nodeContainers.set(childId, containerId);
    }
  }

  /**
   * Remove a child from container hierarchy and maintain all indexes
   * @param {string} containerId - The container ID
   * @param {string} childId - The child node/container ID to remove
   * @private
   */
  _removeChildFromContainerHierarchy(containerId, childId) {
    const container = this.containers.get(containerId);
    if (container) {
      container.children.delete(childId);
      this.#containerChildren.set(containerId, container.children);
      this.#nodeContainers.delete(childId);
    }
  }

  /**
   * Initialize container hierarchy for a new container with children
   * @param {string} containerId - The container ID
   * @param {Set<string>} children - The Set of child IDs
   * @private
   */
  _initializeContainerHierarchy(containerId, children) {
    // Sync the containerChildren map with the container's children Set
    this.#containerChildren.set(containerId, children);
    
    // Add each child to the nodeContainers mapping
    for (const childId of children) {
      this.#nodeContainers.set(childId, containerId);
    }
  }

  /**
   * Clean up container hierarchy when removing a container
   * @param {string} containerId - The container ID being removed
   * @private
   */
  _cleanupContainerHierarchy(containerId) {
    // First, remove this container from its parent's children list
    const parentId = this.#nodeContainers.get(containerId);
    if (parentId) {
      this._removeChildFromContainerHierarchy(parentId, containerId);
    }
    
    // Then clean up this container's children using encapsulated method
    const children = this.#containerChildren.get(containerId);
    if (children) {
      // Create a copy to avoid modification during iteration
      const childrenArray = Array.from(children);
      for (const childId of childrenArray) {
        this._removeChildFromContainerHierarchy(containerId, childId);
      }
    }
    
    // Final cleanup of the container's own mapping
    this.#containerChildren.delete(containerId);
  }
  
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
      this.updateNode(nodeId, { hidden });
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
    this._updateVisibilityMap(this._visibleNodes, id, node);
  }

  _updateVisibleEdges(id, edge) {
    this._updateVisibilityMap(this._visibleEdges, id, edge);
  }

  _updateVisibleContainers(id, container) {
    this._updateVisibilityMap(this._visibleContainers, id, container);
  }

  _updateExpandedContainers(id, container) {
    if (container.collapsed) {
      this._expandedContainers.delete(id);
    } else {
      this._expandedContainers.set(id, container);
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

  // ============ Internal Invariant Checking ============

  /**
   * Check all internal hyperedge invariants
   * This is called after operations that affect containers or edges
   * @private
   */
  _checkHyperEdgeInvariants() {
    this._checkHyperEdgeExistence();
    this._checkHyperEdgeEndpoints();
    this._checkHyperEdgeVisibilityEncapsulation();
  }

  /**
   * Invariant: HyperEdges exist only for visible, collapsed containers
   * @private
   */
  _checkHyperEdgeExistence() {
    const visibleCollapsedContainers = new Set();
    for (const container of this.visibleContainers.values()) {
      if (container.collapsed) {
        visibleCollapsedContainers.add(container.id);
      }
    }

    for (const hyperEdge of this.hyperEdges.values()) {
      const hasCollapsedContainer = 
        visibleCollapsedContainers.has(hyperEdge.source) || 
        visibleCollapsedContainers.has(hyperEdge.target);
      
      if (!hasCollapsedContainer) {
        throw new Error(`HyperEdge invariant violation: ${hyperEdge.id} exists but neither endpoint is a visible collapsed container`);
      }
    }
  }

  /**
   * Invariant: HyperEdges connect to valid visible endpoints (collapsed containers or visible nodes)
   * @private
   */
  _checkHyperEdgeEndpoints() {
    const visibleNodeIds = new Set(this.visibleNodes.keys());
    const visibleContainerIds = new Set(this.visibleContainers.keys());

    for (const hyperEdge of this.hyperEdges.values()) {
      // Check source endpoint
      const sourceIsValidNode = visibleNodeIds.has(hyperEdge.source);
      const sourceIsValidContainer = visibleContainerIds.has(hyperEdge.source);
      
      if (!sourceIsValidNode && !sourceIsValidContainer) {
        throw new Error(`HyperEdge invariant violation: ${hyperEdge.id} source ${hyperEdge.source} is not a visible node or container`);
      }

      // Check target endpoint
      const targetIsValidNode = visibleNodeIds.has(hyperEdge.target);
      const targetIsValidContainer = visibleContainerIds.has(hyperEdge.target);
      
      if (!targetIsValidNode && !targetIsValidContainer) {
        throw new Error(`HyperEdge invariant violation: ${hyperEdge.id} target ${hyperEdge.target} is not a visible node or container`);
      }

      // At least one endpoint must be a collapsed container
      const sourceIsCollapsedContainer = visibleContainerIds.has(hyperEdge.source) && 
        this.getContainer(hyperEdge.source)?.collapsed;
      const targetIsCollapsedContainer = visibleContainerIds.has(hyperEdge.target) && 
        this.getContainer(hyperEdge.target)?.collapsed;

      if (!sourceIsCollapsedContainer && !targetIsCollapsedContainer) {
        throw new Error(`HyperEdge invariant violation: ${hyperEdge.id} must have at least one collapsed container endpoint`);
      }
    }
  }

  /**
   * Invariant: HyperEdges are completely encapsulated and never appear in visibleEdges
   * @private
   */
  _checkHyperEdgeVisibilityEncapsulation() {
    for (const edge of this.visibleEdges.values()) {
      if (edge.id && edge.id.startsWith(HYPER_EDGE_PREFIX)) {
        throw new Error(`HyperEdge encapsulation violation: HyperEdge ${edge.id} found in visibleEdges - hyperedges should be internal only`);
      }
    }
  }
}

/**
<<<<<<< HEAD
 * Create a new visualization state instance
=======
 * Factory function to create a new VisualizationState instance.
 * Preferred over direct constructor usage for consistency and potential future initialization logic.
 * 
 * @function createVisualizationState
 * @returns {VisualizationState} A new, empty visualization state instance
 * @example
 * ```javascript
 * // Fluent interface with method chaining
 * const state = createVisualizationState()
 *   .setGraphNode('node1', { label: 'My First Node' })
 *   .setGraphNode('node2', { label: 'My Second Node' })
 *   .setGraphEdge('edge1', { source: 'node1', target: 'node2' });
 * 
 * console.log(state.getGraphNode('node1')); // { id: 'node1', label: 'My First Node', ... }
 * ```
>>>>>>> 336632b29 (code review of VisState)
 */
export function createVisualizationState() {
  return new VisualizationState();
}
