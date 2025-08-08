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
  CreateContainerProps,
  GraphNode,
  GraphEdge,
  Container
} from '../shared/types';
import { LAYOUT_CONSTANTS } from '../shared/config';
import { ContainerCollapseExpandEngine } from './ContainerCollapseExpand';

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
 * // // console.log(((state.visibleNodes)));     // Array of visible nodes
 * // // console.log(((state.expandedContainers))); // Array of expanded containers
 * 
 * // Update properties idiomatically  
 * state.updateNode('n1', { hidden: true, style: 'highlighted' });
 * state.updateContainer('c1', { collapsed: true });
 * ```
 */
export class VisualizationState implements ContainerHierarchyView {
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

  // Manual position overrides for user drag interactions
  // IMPORTANT: This is the ONLY place manual positions should be stored!
  // Do NOT add manual position state to React components, bridges, or other classes.
  private readonly manualPositions: Map<string, {x: number, y: number}>;

    // Container collapse/expand engine V2
  private readonly collapseExpandEngine: ContainerCollapseExpandEngine;

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
    this._visibleNodes = new Map(); 
    /** @type {Map<string, Object>} Non-hidden edges for rendering */
    this._visibleEdges = new Map(); 
    /** @type {Map<string, Object>} Non-hidden containers for rendering */
    this._visibleContainers = new Map(); 
    /** @type {Map<string, Object>} Non-collapsed containers */
    this._expandedContainers = new Map(); 
    
    // Collapsed container representations
    /** @type {Map<string, Object>} Collapsed container representations */
    this.collapsedContainers = new Map(); 
    
    // Container hierarchy tracking
    /** @type {Map<string, Set<string>>} Container ID to Set of child IDs */
    this.#containerChildren = new Map(); 
    /** @type {Map<string, string>} Node ID to parent container ID */
    this.#nodeContainers = new Map(); 
    
    // Edge tracking for hyperEdge management
    /** @type {Map<string, Set<string>>} Node ID to Set of connected edge IDs */
    this.nodeToEdges = new Map(); 

    // Manual position overrides for user drag interactions
    /** @type {Map<string, {x: number, y: number}>} Element ID to manual position override */
    this.manualPositions = new Map();

        // Initialize container collapse/expand engine V2
    this.collapseExpandEngine = new ContainerCollapseExpandEngine(this);
  }

  // ============ Generic Entity Management ============

  /**
   * Validate that an entity exists and optionally check a condition
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
  }

  /**
   * Generic method to get an entity from any collection
   */
  _getEntity(entityType: string, id: string): any {
    const collection = this._getEntityCollection(entityType);
    return collection.get(id);
  }

  /**
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
   * @param {string} id - The node ID to retrieve
   * @returns {Object|undefined} The node object or undefined if not found
   */
  getGraphNode(id: string): any {
    return this._getEntity(ENTITY_TYPES.NODE, id);
  }

  /**
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
  }

  /**
   * Remove a graph node
   * @param {string} id - The node ID to remove
   * @throws {Error} When node doesn't exist
   */
  removeGraphNode(id: string): void {
    if (!this.graphNodes.has(id)) {
      throw new Error(`Cannot remove node: node '${id}' does not exist`);
    }
    this._removeNodeFromAllStructures(id);
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
   * @param {string} id - The container ID to retrieve
   * @returns {Object|undefined} The container object or undefined if not found
   */
  getContainer(id: string): any {
    return this._getEntity(ENTITY_TYPES.CONTAINER, id);
  }

  /**
   * Get container dimensions adjusted for label positioning.
   * Automatically adds space for bottom-right labels to ensure they don't occlude content.
   * 
   * @param {string} id - The container ID
   * @returns {{width: number, height: number}} Adjusted dimensions including label space
   * @throws {Error} When container doesn't exist
   * @example
   * ```javascript
   * // Container with 300x200 base dimensions
   * state.setContainer('container1', { expandedDimensions: { width: 300, height: 200 } });
   * const dims = state.getContainerAdjustedDimensions('container1'); 
   * // Returns: { width: 300, height: 232 } (200 + 24 label height + 8 padding)
   * ```
   */
  getContainerAdjustedDimensions(id: string): { width: number; height: number } {
    this._validateRequiredString(id, 'Container ID');
    
    const container = this.getContainer(id);
    this._validateEntity(container);

    // Get base dimensions
    const baseWidth = container.expandedDimensions?.width || LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH;
    const baseHeight = container.expandedDimensions?.height || LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT;

    let result: { width: number; height: number };

    if (container.collapsed) {
      // Collapsed containers get fixed small dimensions (ignore expanded dimensions)
      const collapsedHeight = LAYOUT_CONSTANTS.CONTAINER_LABEL_HEIGHT + (LAYOUT_CONSTANTS.CONTAINER_LABEL_PADDING * 2);
      result = {
        width: LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH,
        height: Math.max(collapsedHeight, LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT)
      };
    } else {
      // Expanded containers get additional height for bottom-right label space
      const expandedHeight = baseHeight + LAYOUT_CONSTANTS.CONTAINER_LABEL_HEIGHT + LAYOUT_CONSTANTS.CONTAINER_LABEL_PADDING;
      result = {
        width: baseWidth,
        height: Math.max(expandedHeight, LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT)
      };
    }

    // CORE INVARIANT: Container dimensions must be consistent with collapse state
    if (container.collapsed && (result.width > 300 || result.height > 200)) {
      throw new Error(`Core invariant violation: Collapsed container ${id} has dimensions ${result.width}x${result.height} but collapsed containers must be small (≤300x200). This indicates a bug in getContainerAdjustedDimensions logic.`);
    }
    
    if (!container.collapsed && (result.width < LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH || result.height < LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT)) {
      throw new Error(`Core invariant violation: Expanded container ${id} has dimensions ${result.width}x${result.height} but expanded containers must be at least ${LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH}x${LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT}.`);
    }

    return result;
  }

  /**
   * Update a container's properties. More idiomatic than separate getters/setters.
   * @param {string} id - The container ID
   * @param {Partial<{collapsed: boolean, hidden: boolean, label: string}>} updates - Properties to update
   * @throws {Error} When container doesn't exist or parameters are invalid
   * @example
   * ```javascript
   * state.updateContainer('container1', { collapsed: true, hidden: false });
   * ```
   */
  updateContainer(id: string, updates: { collapsed?: boolean; hidden?: boolean; label?: string }) {
    this._validateRequiredString(id, 'Container ID');
    
    const container = this.getContainer(id);
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
  }

  /**
   * Add a child to a container
   * @param {string} containerId - The container ID
   * @param {string} childId - The child node/container ID to add
   * @throws {Error} When container doesn't exist or parameters are invalid
   */
  addContainerChild(containerId: string, childId: string): void {
    this._validateRequiredString(containerId, 'Container ID');
    this._validateRequiredString(childId, 'Child ID');
    
    const container = this.getContainer(containerId);
    this._validateEntity(container);
    
    // Validate tree hierarchy (no cycles/DAGs)
    this.collapseExpandEngine.validateTreeHierarchy(containerId, childId);
    
    // Use helper to maintain consistency
    this._addChildToContainerHierarchy(containerId, childId);
    
    // Update edge index
    this.collapseExpandEngine.rebuildEdgeIndex();
  }

  /**
   * Remove a child from a container
   * @param {string} containerId - The container ID
   * @param {string} childId - The child node/container ID to remove
   * @throws {Error} When container doesn't exist or parameters are invalid
   */
  removeContainerChild(containerId: string, childId: string): void {
    this._validateRequiredString(containerId, 'Container ID');
    this._validateRequiredString(childId, 'Child ID');
    
    const container = this.getContainer(containerId);
    this._validateEntity(container);
    
    // Use helper to maintain consistency
    this._removeChildFromContainerHierarchy(containerId, childId);
    
    // Update edge index
    this.collapseExpandEngine.rebuildEdgeIndex();
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
   * @param {boolean} [props.hidden=false] - Whether the hyperedge is hidden
   * @param {Set<string>} [props.aggregatedChildren] - IDs of (hyper)edges this aggregated
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
   *     aggregatedChildren: new Set(['edge1', 'edge2'])
   *   });
   * ```
   */
  setHyperEdge(id: string, { source, target, style = EDGE_STYLES.DEFAULT, hidden = false, ...otherProps }: { source: string; target: string; style?: string; hidden?: boolean; [key: string]: any }) {
    this._validateRequiredString(id, 'HyperEdge ID');
    this._validateRequiredString(source, 'HyperEdge source');
    this._validateRequiredString(target, 'HyperEdge target');
    this._validateStyle(style, EDGE_STYLES, 'HyperEdge');

    const hyperEdge = {
      id,
      source,
      target,
      style,
      hidden,
      ...otherProps
    };
    
    this._addHyperEdgeToAllStructures(id, hyperEdge);
    return this;
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
   * Get all visible (non-hidden) nodes with computed position/dimension properties
   */
  get visibleNodes() {
    return Array.from(this._visibleNodes.values()).map(node => {
      // Create a computed view that exposes layout data as direct properties
      const computedNode = {
        ...node,
        // Expose layout position as direct x, y properties (ELK updates these directly)
        x: node.x ?? node.layout?.position?.x ?? 0,
        y: node.y ?? node.layout?.position?.y ?? 0,
        // Expose layout dimensions as direct width, height properties
        width: node.width ?? node.layout?.dimensions?.width ?? 180, // Default ELK width
        height: node.height ?? node.layout?.dimensions?.height ?? 60  // Default ELK height
      };
      
      return computedNode;
    });
  }

  /**
   * Get all visible (non-hidden) edges, including hyperedges when appropriate
   * This provides a unified view of edges for external systems (ELK, ReactFlow)
   * Hyperedges are included when their corresponding containers are collapsed
   */
  get visibleEdges() {
    const regularEdges = Array.from(this._visibleEdges.values());
    
    // Include non-hidden hyperedges (these represent collapsed container connections)
    const activeHyperEdges = Array.from(this.hyperEdges.values()).filter(hyperEdge => !hyperEdge.hidden);
    
    // Return unified edge collection - external systems don't need to know about hyperedges
    return [...regularEdges, ...activeHyperEdges];
  }

  /**
   * Get all visible (non-hidden) containers with computed position/dimension properties
   * Dimensions are automatically adjusted to accommodate bottom-right label positioning.
   */
  get visibleContainers() {
    return Array.from(this._visibleContainers.values()).map(container => {
      // Get label-adjusted dimensions for this container
      const adjustedDims = this.getContainerAdjustedDimensions(container.id);
      
      // Create a computed view that exposes layout data as direct properties
      const computedContainer = {
        id: container.id,
        collapsed: container.collapsed,
        hidden: container.hidden,
        children: container.children,
        // Expose layout position as direct x, y properties
        x: container.layout?.position?.x ?? 0,
        y: container.layout?.position?.y ?? 0,
        // Use adjusted dimensions that include label space
        // ELK layout dimensions take precedence if available, otherwise use adjusted base dimensions
        width: container.layout?.dimensions?.width ?? adjustedDims.width,
        height: container.layout?.dimensions?.height ?? adjustedDims.height,
        // Copy any other custom properties but exclude internal ones
        ...Object.fromEntries(
          Object.entries(container).filter(([key]) => 
            !['layout', 'expandedDimensions', 'id', 'collapsed', 'hidden', 'children'].includes(key)
          )
        )
      };
      
      return computedContainer;
    });
  }

  /**
   * Get all expanded (non-collapsed) containers
   */
  get expandedContainers() {
    return Array.from(this._expandedContainers.values());
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

  // ============ Invariant Validation ============
  
  /**
   * Validate all internal data structure invariants
   * @param {string} [context=''] - Optional context string for error messages
   * @throws {Error} When any invariant is violated
   */
  validateAllInvariants(context: string = ''): void {
    this.validateHyperedgeInvariants(context);
    this.validateContainerVisibilityInvariant(context);
    this.validateEdgeVisibilityInvariant(context);
  }

  /**
   * Validate container visibility invariant: containers in _visibleContainers should not have collapsed ancestors
   * @param {string} [context=''] - Optional context string for error messages
   * @throws {Error} When container visibility invariant is violated
   */
  validateContainerVisibilityInvariant(context: string = ''): void {
    const contextPrefix = context ? `${context}: ` : '';
    
    for (const [containerId, container] of this._visibleContainers) {
      if (container.hidden) {
        throw new Error(`${contextPrefix}Container visibility invariant violated: hidden container ${containerId} found in _visibleContainers`);
      }
      
      if (this._hasCollapsedAncestor(containerId)) {
        throw new Error(`${contextPrefix}Container visibility invariant violated: container ${containerId} has collapsed ancestor but is in _visibleContainers`);
      }
    }
  }

  /**
   * Validate edge visibility invariant: edges adjacent to hidden nodes should be hidden
   * @param {string} [context=''] - Optional context string for error messages
   * @throws {Error} When edge visibility invariant is violated
   */
  validateEdgeVisibilityInvariant(context: string = ''): void {
    const contextPrefix = context ? `${context}: ` : '';
    
    for (const [edgeId, edge] of this.graphEdges) {
      const sourceNode = this.graphNodes.get(edge.source);
      const targetNode = this.graphNodes.get(edge.target);
      
      // Skip edges that don't connect nodes (might connect containers)
      if (!sourceNode || !targetNode) continue;
      
      const shouldBeHidden = sourceNode.hidden || targetNode.hidden;
      const isVisible = this._visibleEdges.has(edgeId);
      
      if (shouldBeHidden && isVisible) {
        throw new Error(`${contextPrefix}Edge visibility invariant violated: edge ${edgeId} connects hidden node(s) but is visible (source hidden: ${sourceNode.hidden}, target hidden: ${targetNode.hidden})`);
      }
      
      if (!shouldBeHidden && !edge.hidden && !isVisible) {
        throw new Error(`${contextPrefix}Edge visibility invariant violated: edge ${edgeId} connects visible nodes but is not visible`);
      }
    }
  }
  
  /**
   * Validate all internal hyperedge invariants
   * @param {string} [context=''] - Optional context string for error messages
   * @throws {Error} When any hyperedge invariant is violated
   * @example
   * ```javascript
   * state.validateHyperedgeInvariants('After container collapse');
   * ```
   */
  validateHyperedgeInvariants(context: string = ''): void {
    const contextPrefix = context ? `${context}: ` : '';
    
    // Get all active hyperedges (not hidden ones)
    const activeHyperEdges = Array.from(this.hyperEdges.values()).filter(he => !he.hidden);
    
    // Get visible nodes and containers for validation
    const visibleNodeIds = new Set(this.visibleNodes.map(n => n.id));
    const visibleContainers = this.visibleContainers;
    
    // Get ALL collapsed containers (not just visible ones)
    // This is important because containers can be collapsed but not visible due to collapsed ancestors
    const allCollapsedContainerIds = new Set();
    for (const [containerId, container] of this.containers) {
      if (container.collapsed) {
        allCollapsedContainerIds.add(containerId);
      }
    }
    
    for (const hyperEdge of activeHyperEdges) {
      // INVARIANT 1: HyperEdges must have at least one collapsed container endpoint
      const sourceIsCollapsedContainer = allCollapsedContainerIds.has(hyperEdge.source);
      const targetIsCollapsedContainer = allCollapsedContainerIds.has(hyperEdge.target);
      
      if (!sourceIsCollapsedContainer && !targetIsCollapsedContainer) {
        throw new Error(`${contextPrefix}HyperEdge ${hyperEdge.id} violates invariant: must have at least one collapsed container endpoint (source: ${hyperEdge.source}, target: ${hyperEdge.target})`);
      }
      
      // INVARIANT 2: All hyperedge endpoints must be either:
      // - Visible nodes, OR
      // - Visible containers, OR 
      // - Collapsed containers (even if not visible due to collapsed ancestors)
      const sourceIsVisibleNode = visibleNodeIds.has(hyperEdge.source);
      const targetIsVisibleNode = visibleNodeIds.has(hyperEdge.target);
      const sourceIsVisibleContainer = visibleContainers.some(c => c.id === hyperEdge.source);
      const targetIsVisibleContainer = visibleContainers.some(c => c.id === hyperEdge.target);
      const sourceIsCollapsedContainerAny = allCollapsedContainerIds.has(hyperEdge.source);
      const targetIsCollapsedContainerAny = allCollapsedContainerIds.has(hyperEdge.target);
      
      if (!(sourceIsVisibleNode || sourceIsVisibleContainer || sourceIsCollapsedContainerAny)) {
        throw new Error(`${contextPrefix}HyperEdge ${hyperEdge.id} violates invariant: source ${hyperEdge.source} must be a visible node, visible container, or collapsed container`);
      }
      
      if (!(targetIsVisibleNode || targetIsVisibleContainer || targetIsCollapsedContainerAny)) {
        throw new Error(`${contextPrefix}HyperEdge ${hyperEdge.id} violates invariant: target ${hyperEdge.target} must be a visible node, visible container, or collapsed container`);
      }
    }
    
    // INVARIANT 3: No hyperedges should leak into visibleEdges (encapsulation check)
    const visibleEdges = Array.from(this._visibleEdges.values());
    for (const edge of visibleEdges) {
      if (edge.id?.startsWith(HYPER_EDGE_PREFIX)) {
        throw new Error(`${contextPrefix}Encapsulation violation: hyperedge ${edge.id} found in visibleEdges - hyperedges should be internal only`);
      }
    }
  }

  // ============ Container Collapse/Expand Operations ============
  
  /**
   * Collapse a container (depth-first, bottom-up with edge lifting)
   * Uses optimized engine with tree hierarchy validation and edge indexing
   */
  collapseContainer(containerId: string): void {
    this.collapseExpandEngine.collapseContainer(containerId);
    
    // CRITICAL: Update hidden state of all descendant nodes when container is collapsed
    this._updateDescendantNodesOnCollapse(containerId);
    this._updateDescendantEdgesOnCollapse(containerId);
    
    // After collapse operation, refresh visibility for all containers
    // This ensures visibility invariants are maintained after the collapse engine modifies container.collapsed
    this._refreshAllContainerVisibility();
    
    // Validate all invariants after collapse operation
    this.validateAllInvariants(`After collapsing container ${containerId}`);
  }
  
  /**
   * Expand a container (depth-first, top-down with edge grounding)
   * SYMMETRIC INVERSE of collapseContainer()
   */
  expandContainer(containerId: string): void {
    this.collapseExpandEngine.expandContainer(containerId);
    
    // CRITICAL: Update hidden state of all descendant nodes when container is expanded
    this._updateDescendantNodesOnExpand(containerId);
    this._updateDescendantEdgesOnExpand(containerId);
    
    // After expand operation, refresh visibility for all containers
    // This ensures visibility invariants are maintained after the collapse engine modifies container.collapsed
    this._refreshAllContainerVisibility();
    
    // Validate all invariants after expand operation
    this.validateAllInvariants(`After expanding container ${containerId}`);
  }

  // ============ Bridge Support Methods (Business Logic) ============
  
  /**
   * Get collapsed containers converted to nodes for layout engines
   * This centralizes the business logic for how collapsed containers appear as nodes
   */
  getCollapsedContainersAsNodes(): GraphNode[] {
    const nodes: GraphNode[] = [];
    
    this.visibleContainers.forEach(container => {
      if (container.collapsed) {
        const containerAsNode: GraphNode = {
          id: container.id,
          label: container.id, // Use id as label since containers don't have a name property
          width: container.width || LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH,
          height: container.height || LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT,
          x: container.x || 0,
          y: container.y || 0,
          hidden: false,
          style: 'default'
        };
        nodes.push(containerAsNode);
      }
    });
    
    return nodes;
  }

  /**
   * Get parent-child relationship map for UI frameworks
   * This centralizes the business logic for which containers can have visible children
   */
  getParentChildMap(): Map<string, string> {
    const parentMap = new Map<string, string>();
    
    this.visibleContainers.forEach(container => {
      if (!container.collapsed) {
        // Only expanded containers can have children in UI frameworks
        container.children.forEach(childId => {
          parentMap.set(childId, container.id);
        });
      }
    });
    
    return parentMap;
  }

  /**
   * Get top-level nodes that don't belong to any container
   * This centralizes the business logic for determining node hierarchy
   */
  getTopLevelNodes(): GraphNode[] {
    const collapsedContainerIds = new Set(
      this.visibleContainers.filter(c => c.collapsed).map(c => c.id)
    );
    
    return this.visibleNodes.filter(node => {
      // Node is top-level if:
      // 1. It's not inside any expanded container
      // 2. It's not a collapsed container (those are handled separately)
      const parentContainer = this.getNodeContainer(node.id);
      const isInExpandedContainer = parentContainer && 
        this.visibleContainers.some(c => c.id === parentContainer && !c.collapsed);
      
      return !isInExpandedContainer && !collapsedContainerIds.has(node.id);
    });
  }

  /**
   * Get handle configuration for an edge
   * This centralizes the business logic for edge handle assignments
   */
  getEdgeHandles(edgeId: string): { sourceHandle?: string; targetHandle?: string } {
    const edge = this.getGraphEdge(edgeId);
    if (!edge) return {};
    
    return {
      sourceHandle: edge.sourceHandle || 'default-out',
      targetHandle: edge.targetHandle || 'default-in'
    };
  }

  /**
   * Ensure all nodes and containers have valid dimensions
   * This centralizes the business logic for default dimensions
   */
  validateAndFixDimensions(): void {
    // Fix node dimensions
    for (const [nodeId, node] of this.graphNodes) {
      if (!node.width || node.width <= 0) {
        node.width = LAYOUT_CONSTANTS.DEFAULT_NODE_WIDTH;
      }
      if (!node.height || node.height <= 0) {
        node.height = LAYOUT_CONSTANTS.DEFAULT_NODE_HEIGHT;
      }
    }
    
    // Fix container dimensions
    for (const [containerId, container] of this.containers) {
      if (!container.width || container.width <= 0) {
        container.width = LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH;
      }
      if (!container.height || container.height <= 0) {
        container.height = LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT;
      }
    }
  }

  // ============ Private Helper Methods ============
  
  /**
   * Check if a container has any collapsed ancestor
   * @param {string} containerId - The container to check
   * @returns {boolean} True if any ancestor is collapsed
   * @private
   */
  private _hasCollapsedAncestor(containerId: string): boolean {
    const parentId = this.getNodeContainer(containerId);
    if (!parentId) {
      return false; // No parent, so no collapsed ancestor
    }
    
    const parentContainer = this.getContainer(parentId);
    if (!parentContainer) {
      return false; // Parent is not a container
    }
    
    if (parentContainer.collapsed) {
      return true; // Direct parent is collapsed
    }
    
    // Recursively check further up the hierarchy
    return this._hasCollapsedAncestor(parentId);
  }

  /**
   * Update visibility of all descendant containers when an ancestor's collapsed state changes
   * @param {string} ancestorId - The container whose descendants need visibility updates
   * @private
   */
  private _updateDescendantContainerVisibility(ancestorId: string): void {
    const children = this.getContainerChildren(ancestorId);
    
    for (const childId of children) {
      const childContainer = this.getContainer(childId);
      if (childContainer) {
        // Recursively update this child container's visibility
        this._updateVisibleContainers(childId, childContainer);
        
        // Recursively update this child's descendants
        this._updateDescendantContainerVisibility(childId);
      }
    }
  }

  /**
   * Refresh visibility for all containers based on current collapsed state
   * This is needed after collapse/expand operations that directly modify container.collapsed
   * @private
   */
  private _refreshAllContainerVisibility(): void {
    for (const [containerId, container] of this.containers) {
      const shouldBeVisible = !container.hidden && !this._hasCollapsedAncestor(containerId);
      
      if (shouldBeVisible) {
        this._visibleContainers.set(containerId, container);
      } else {
        this._visibleContainers.delete(containerId);
      }
    }
  }
  
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
    // A node is visible only if it's not hidden AND not inside a collapsed container
    const isInCollapsedContainer = this._isNodeInCollapsedContainer(id);
    const shouldBeVisible = !node.hidden && !isInCollapsedContainer;
    
    if (shouldBeVisible) {
      this._visibleNodes.set(id, node);
    } else {
      this._visibleNodes.delete(id);
    }
    
    // When a node's visibility changes, update adjacent edges
    if ('hidden' in node || node.hidden !== undefined) {
      this._updateAdjacentEdgesVisibility(id, node.hidden);
    }
  }

  /**
   * Update visibility of edges adjacent to a node
   * @param {string} nodeId - The node whose adjacent edges should be updated
   * @param {boolean} nodeHidden - Whether the node is hidden
   * @private
   */
  _updateAdjacentEdgesVisibility(nodeId: string, nodeHidden: boolean): void {
    const adjacentEdgeIds = this.nodeToEdges.get(nodeId);
    if (!adjacentEdgeIds) return;

    for (const edgeId of adjacentEdgeIds) {
      const edge = this.graphEdges.get(edgeId);
      if (!edge) continue;

      // An edge should be hidden if either endpoint is hidden
      const sourceNode = this.graphNodes.get(edge.source);
      const targetNode = this.graphNodes.get(edge.target);
      
      const shouldHideEdge = (sourceNode?.hidden) || (targetNode?.hidden);
      
      // Only update if visibility actually changed
      if (edge.hidden !== shouldHideEdge) {
        edge.hidden = shouldHideEdge;
        this._updateVisibleEdges(edgeId, edge);
      }
    }
  }

  /**
   * Update hidden state of all descendant nodes when a container is collapsed
   */
  _updateDescendantNodesOnCollapse(containerId) {
    const container = this.containers.get(containerId);
    if (!container) return;
    
    // Recursively find all descendant nodes and mark them as hidden
    const allDescendantNodes = this._getAllDescendantNodes(containerId);
    for (const nodeId of allDescendantNodes) {
      const node = this.graphNodes.get(nodeId);
      if (node && !node.hidden) {
        node.hidden = true;
        this._updateVisibleNodes(nodeId, node);
      }
    }
    
    // Also hide all descendant containers
    const allDescendantContainers = this._getAllDescendantContainers(containerId);
    for (const containerIdChild of allDescendantContainers) {
      const childContainer = this.containers.get(containerIdChild);
      if (childContainer && !childContainer.hidden) {
        childContainer.hidden = true;
        this._updateVisibleContainers(containerIdChild, childContainer);
      }
    }
  }

  /**
   * Update hidden state of all descendant nodes when a container is expanded
   */
  _updateDescendantNodesOnExpand(containerId) {
    const container = this.containers.get(containerId);
    if (!container) return;
    
    // Only make direct children visible - let nested containers control their own children
    for (const childId of container.children) {
      const childNode = this.graphNodes.get(childId);
      if (childNode) {
        // Only unhide if the node is not in another collapsed container
        const shouldBeVisible = !this._isNodeInCollapsedContainer(childId);
        if (shouldBeVisible && childNode.hidden) {
          childNode.hidden = false;
          this._updateVisibleNodes(childId, childNode);
        }
      }
      
      // Also handle child containers
      const childContainer = this.containers.get(childId);
      if (childContainer) {
        // Only unhide if the container is not in another collapsed container
        const shouldBeVisible = !this._hasCollapsedAncestor(childId);
        if (shouldBeVisible && childContainer.hidden) {
          childContainer.hidden = false;
          this._updateVisibleContainers(childId, childContainer);
        }
      }
    }
  }

  /**
   * Update hidden state of all descendant edges when a container is collapsed
   */
  _updateDescendantEdgesOnCollapse(containerId) {
    // Find all edges that have endpoints inside the collapsed container
    const allDescendantNodes = this._getAllDescendantNodes(containerId);
    const descendantNodeSet = new Set(allDescendantNodes);
    
    for (const [edgeId, edge] of this.graphEdges) {
      const sourceInContainer = descendantNodeSet.has(edge.source);
      const targetInContainer = descendantNodeSet.has(edge.target);
      
      // Hide edge if at least one endpoint is in the collapsed container
      if ((sourceInContainer || targetInContainer) && !edge.hidden) {
        edge.hidden = true;
        this._updateVisibleEdges(edgeId, edge);
      }
    }
  }

  /**
   * Update hidden state of all descendant edges when a container is expanded
   */
  _updateDescendantEdgesOnExpand(containerId) {
    const container = this.containers.get(containerId);
    if (!container) return;
    
    // Re-evaluate all edges to see if they should become visible
    for (const [edgeId, edge] of this.graphEdges) {
      if (edge.hidden) {
        const sourceNode = this.graphNodes.get(edge.source);
        const targetNode = this.graphNodes.get(edge.target);
        
        // Edge should be visible if both endpoints are visible
        const shouldBeVisible = sourceNode && !sourceNode.hidden && targetNode && !targetNode.hidden;
        if (shouldBeVisible) {
          edge.hidden = false;
          this._updateVisibleEdges(edgeId, edge);
        }
      }
    }
  }

  /**
   * Get all descendant nodes of a container (recursively)
   */
  _getAllDescendantNodes(containerId) {
    const result = [];
    const container = this.containers.get(containerId);
    if (!container) return result;
    
    for (const childId of container.children) {
      const childContainer = this.containers.get(childId);
      if (childContainer) {
        // Child is a container - recurse into it
        result.push(...this._getAllDescendantNodes(childId));
      } else {
        // Child is a node
        result.push(childId);
      }
    }
    
    return result;
  }

  /**
   * Get all descendant containers of a container (recursively)
   */
  _getAllDescendantContainers(containerId) {
    const result = [];
    const container = this.containers.get(containerId);
    if (!container) return result;
    
    for (const childId of container.children) {
      const childContainer = this.containers.get(childId);
      if (childContainer) {
        // Child is a container - add it and recurse
        result.push(childId);
        result.push(...this._getAllDescendantContainers(childId));
      }
    }
    
    return result;
  }

  /**
   * Check if a node is inside a collapsed container (anywhere in the hierarchy)
   */
  _isNodeInCollapsedContainer(nodeId) {
    // Check if this node is a direct child of any collapsed container
    for (const [containerId, container] of this.containers) {
      if (container.collapsed && container.children.has(nodeId)) {
        return true;
      }
    }
    
    // Check if this node is inside any container that has a collapsed ancestor
    for (const [containerId, container] of this.containers) {
      if (container.children.has(nodeId)) {
        // This node is in this container, check if the container has collapsed ancestors
        if (this._hasCollapsedAncestor(containerId)) {
          return true;
        }
      }
    }
    
    return false;
  }

  _updateVisibleEdges(id, edge) {
    this._updateVisibilityMap(this._visibleEdges, id, edge);
  }

  _updateVisibleContainers(id, container) {
    // A container is visible only if it's not hidden AND none of its ancestors are collapsed
    const shouldBeVisible = !container.hidden && !this._hasCollapsedAncestor(id);
    
    if (shouldBeVisible) {
      this._visibleContainers.set(id, container);
    } else {
      this._visibleContainers.delete(id);
    }
    
    // When a container's collapsed state changes, update visibility of all descendants
    // This must happen AFTER updating this container's visibility
    if ('collapsed' in container || container.collapsed !== undefined) {
      this._updateDescendantContainerVisibility(id);
    }
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
}

/**
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
 * // // console.log(((state.getGraphNode('node1')))); // { id: 'node1', label: 'My First Node', ... }
 * ```
 */
export function createVisualizationState() {
  return new VisualizationState();
}

// Add interface compatibility methods
declare module './VisState.js' {
  interface VisualizationState {
    // Node interface methods
    setNodeHidden(id: string, hidden: boolean): void;
    getNodeHidden(id: string): boolean | undefined;
    
    // Edge interface methods  
    setEdgeHidden(id: string, hidden: boolean): void;
    getEdgeHidden(id: string): boolean | undefined;
    
    // Container interface methods
    setContainerCollapsed(id: string, collapsed: boolean): void;
    getContainerCollapsed(id: string): boolean | undefined;
    setContainerHidden(id: string, hidden: boolean): void;
    getContainerHidden(id: string): boolean | undefined;

    // Layout interface methods - CENTRALIZED LAYOUT STATE
    setNodeLayout(id: string, layout: Partial<import('../shared/types').LayoutState>): void;
    getNodeLayout(id: string): import('../shared/types').LayoutState | undefined;
    setEdgeLayout(id: string, layout: Partial<import('../shared/types').LayoutState>): void;
    getEdgeLayout(id: string): import('../shared/types').LayoutState | undefined;
    setContainerLayout(id: string, layout: Partial<import('../shared/types').LayoutState>): void;
    getContainerLayout(id: string): import('../shared/types').LayoutState | undefined;

    // ELK integration methods
    setContainerELKFixed(id: string, fixed: boolean): void;
    getContainerELKFixed(id: string): boolean | undefined;
    getContainersRequiringLayout(changedContainerId?: string): import('../shared/types').Container[];

    // Manual position management methods
    setManualPosition(elementId: string, x: number, y: number): void;
    getManualPosition(elementId: string): {x: number, y: number} | null;
    hasManualPosition(elementId: string): boolean;
    clearManualPosition(elementId: string): void;
    clearAllManualPositions(): void;
    getAllManualPositions(): Map<string, {x: number, y: number}>;
    hasAnyManualPositions(): boolean;

    // Enhanced collapse/expand with layout state management
    collapseContainer(containerId: string): void;
    expandContainer(containerId: string): void;
  }
}

// Implement the interface methods
Object.assign(VisualizationState.prototype, {
  setNodeHidden(id: string, hidden: boolean): void {
    this.updateNode(id, { hidden });
  },
  
  getNodeHidden(id: string): boolean | undefined {
    return this.getGraphNode(id)?.hidden;
  },
  
  setEdgeHidden(id: string, hidden: boolean): void {
    this.updateEdge(id, { hidden });
  },
  
  getEdgeHidden(id: string): boolean | undefined {
    return this.getGraphEdge(id)?.hidden;
  },
  
  setContainerCollapsed(id: string, collapsed: boolean): void {
    this.updateContainer(id, { collapsed });
  },
  
  getContainerCollapsed(id: string): boolean | undefined {
    return this.getContainer(id)?.collapsed;
  },
  
  setContainerHidden(id: string, hidden: boolean): void {
    this.updateContainer(id, { hidden });
  },
  
  getContainerHidden(id: string): boolean | undefined {
    return this.getContainer(id)?.hidden;
  }
});

// ============ CENTRALIZED LAYOUT STATE MANAGEMENT ============
// ALL layout information flows through VisState - ELK and ReactFlow get data from here

Object.assign(VisualizationState.prototype, {
  // Node layout methods
  setNodeLayout(id: string, layout: Partial<any>): void {
    const node = this.getGraphNode(id);
    this._validateEntity(node);
    
    if (!node.layout) {
      node.layout = {};
    }
    Object.assign(node.layout, layout);
  },

  getNodeLayout(id: string): any {
    return this.getGraphNode(id)?.layout;
  },

  // Edge layout methods
  setEdgeLayout(id: string, layout: Partial<any>): void {
    const edge = this.getGraphEdge(id);
    this._validateEntity(edge);
    
    if (!edge.layout) {
      edge.layout = {};
    }
    Object.assign(edge.layout, layout);
  },

  getEdgeLayout(id: string): any {
    return this.getGraphEdge(id)?.layout;
  },

  // Container layout methods
  setContainerLayout(id: string, layout: Partial<any>): void {
    const container = this.getContainer(id);
    this._validateEntity(container);
    
    if (!container.layout) {
      container.layout = {};
    }
    Object.assign(container.layout, layout);
    
    // IMPORTANT: When layout dimensions are updated, automatically update expandedDimensions
    // This encapsulates the expandedDimensions management within VisState
    if (layout.dimensions) {
      if (layout.dimensions.width !== undefined || layout.dimensions.height !== undefined) {
        container.expandedDimensions = {
          width: layout.dimensions.width ?? container.expandedDimensions.width,
          height: layout.dimensions.height ?? container.expandedDimensions.height
        };
        // // console.log(((`[VisState] 📏 Auto-updated expandedDimensions for ${id}: ${container.expandedDimensions.width}x${container.expandedDimensions.height}`)));
      }
    }
  },

  getContainerLayout(id: string): any {
    return this.getContainer(id)?.layout;
  },

  // ELK position fixing methods
  setContainerELKFixed(id: string, fixed: boolean): void {
    this.setContainerLayout(id, { elkFixed: fixed });
  },

  getContainerELKFixed(id: string): boolean | undefined {
    const layout = this.getContainerLayout(id);
    return layout?.elkFixed ?? false; // Default to false if not set
  },

  // Get containers requiring layout with position fixing logic
  getContainersRequiringLayout(changedContainerId?: string): any[] {
    const containers = this.visibleContainers;
    
    // Apply CENTRALIZED position fixing logic: 
    // Everything FIXED except the container that changed
    return containers.map(container => {
      const shouldBeFixed = changedContainerId && container.id !== changedContainerId;
      
      // Ensure elkFixed is set in VisState
      this.setContainerELKFixed(container.id, shouldBeFixed);
      
      return container;
    });
  },

  // ============ Manual Position Management ============
  // ARCHITECTURAL NOTE: Manual positions are stored ONLY in VisState to ensure:
  // 1. Clean resets: new VisState = no manual positions
  // 2. Single source of truth: no scattered React state
  // 3. Atomic updates: positions and graph structure stay in sync
  // 
  // DO NOT add manual position state to:
  // - React components (FlowGraph, etc.)
  // - Bridge classes (ReactFlowBridge, etc.) 
  // - Layout engines (ELK, etc.)
  // This prevents state pollution and reset bugs.

  /**
   * Set a manual position override for a node or container
   * @param {string} elementId - ID of the element to position
   * @param {number} x - X coordinate
   * @param {number} y - Y coordinate
   */
  setManualPosition(elementId: string, x: number, y: number): void {
    this._validateRequiredString(elementId, 'Element ID');
    if (typeof x !== 'number' || typeof y !== 'number') {
      throw new Error('Position coordinates must be numbers');
    }
    
    this.manualPositions.set(elementId, { x, y });
  },

  /**
   * Get manual position override for an element
   * @param {string} elementId - ID of the element
   * @returns {{x: number, y: number} | null} Manual position or null if not set
   */
  getManualPosition(elementId: string): {x: number, y: number} | null {
    return this.manualPositions.get(elementId) || null;
  },

  /**
   * Check if an element has a manual position override
   * @param {string} elementId - ID of the element
   * @returns {boolean} True if element has manual position
   */
  hasManualPosition(elementId: string): boolean {
    return this.manualPositions.has(elementId);
  },

  /**
   * Remove manual position override for an element
   * @param {string} elementId - ID of the element
   */
  clearManualPosition(elementId: string): void {
    this.manualPositions.delete(elementId);
  },

  /**
   * Clear all manual position overrides
   * Called during resets to ensure clean state
   */
  clearAllManualPositions(): void {
    this.manualPositions.clear();
  },

  /**
   * Get all manual positions as a Map
   * @returns {Map<string, {x: number, y: number}>} Copy of manual positions
   */
  getAllManualPositions(): Map<string, {x: number, y: number}> {
    return new Map(this.manualPositions);
  },

  /**
   * Check if any manual positions exist
   * @returns {boolean} True if any manual positions are set
   */
  hasAnyManualPositions(): boolean {
    return this.manualPositions.size > 0;
  }
});
