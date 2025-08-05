/**
 * Visualization State - Core Data Structure
 *
 * Maintains the mutable state of the visualization including nodes, edges, containers, and hyperEdges.
 * Provides efficient access to visible/non-hidden elements through Maps and collections.
 */
var __classPrivateFieldSet = (this && this.__classPrivateFieldSet) || function (receiver, state, value, kind, f) {
    if (kind === "m") throw new TypeError("Private method is not writable");
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a setter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot write private member to an object whose class did not declare it");
    return (kind === "a" ? f.call(receiver, value) : f ? f.value = value : state.set(receiver, value)), value;
};
var __classPrivateFieldGet = (this && this.__classPrivateFieldGet) || function (receiver, state, kind, f) {
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a getter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
    return kind === "m" ? f : kind === "a" ? f.call(receiver) : f ? f.value : state.get(receiver);
};
var _VisualizationState_containerChildren, _VisualizationState_nodeContainers;
import { NODE_STYLES, EDGE_STYLES } from '../shared/types';
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
export class VisualizationState {
    /**
     * Create a new VisualizationState instance
     * @constructor
     */
    constructor() {
        // Container hierarchy tracking (truly private with # syntax)
        _VisualizationState_containerChildren.set(this, void 0);
        _VisualizationState_nodeContainers.set(this, void 0);
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
        __classPrivateFieldSet(this, _VisualizationState_containerChildren, new Map(), "f");
        /** @type {Map<string, string>} Node ID to parent container ID */
        __classPrivateFieldSet(this, _VisualizationState_nodeContainers, new Map(), "f");
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
    _getEntity(entityType, id) {
        const collection = this._getEntityCollection(entityType);
        return collection.get(id);
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
    setGraphNode(id, props) {
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
    getGraphNode(id) {
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
    updateNode(id, updates) {
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
    removeGraphNode(id) {
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
    setGraphEdge(id, props) {
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
    getGraphEdge(id) {
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
    updateEdge(id, updates) {
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
    removeGraphEdge(id) {
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
    setContainer(id, props) {
        const { expandedDimensions = { width: 0, height: 0 }, collapsed = false, hidden = false, children = [], ...otherProps } = props;
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
    getContainer(id) {
        return this._getEntity(ENTITY_TYPES.CONTAINER, id);
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
    updateContainer(id, updates) {
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
    addContainerChild(containerId, childId) {
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
    removeContainerChild(containerId, childId) {
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
    removeContainer(id) {
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
    setHyperEdge(id, { source, target, style = EDGE_STYLES.DEFAULT, hidden = false, ...otherProps }) {
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
    getHyperEdge(id) {
        return this._getEntity(ENTITY_TYPES.HYPER_EDGE, id);
    }
    /**
     * Remove a hyper edge
     * @param {string} id - The hyperEdge ID to remove
     * @throws {Error} When hyperEdge doesn't exist
     */
    removeHyperEdge(id) {
        if (!this.hyperEdges.has(id)) {
            throw new Error(`Cannot remove hyperEdge: hyperEdge '${id}' does not exist`);
        }
        this._removeHyperEdgeFromAllStructures(id);
    }
    // ============ Computed Properties (Idiomatic TypeScript Getters) ============
    /**
     * Get all containers
     */
    get allContainers() {
        return Array.from(this.containers.values());
    }
    /**
     * Get all visible (non-hidden) nodes with computed position/dimension properties
     */
    get visibleNodes() {
        return Array.from(this._visibleNodes.values());
    }
    /**
     * Get all visible (non-hidden) edges, including hyperedges when appropriate
     * This provides a unified view of edges for external systems (ELK, ReactFlow)
     * Hyperedges are included when their corresponding containers are collapsed
     */
    get visibleEdges() {
        return Array.from(this._visibleEdges.values());
    }
    /**
     * Get all visible (non-hidden) containers with computed position/dimension properties
     */
    get visibleContainers() {
        return Array.from(this._visibleContainers.values()).map(container => {
            // Create a computed view that exposes layout data as direct properties
            const computedContainer = {
                id: container.id,
                collapsed: container.collapsed,
                hidden: container.hidden,
                children: container.children,
                // Expose layout position as direct x, y properties
                x: container.layout?.position?.x ?? 0,
                y: container.layout?.position?.y ?? 0,
                // Expose layout dimensions as direct width, height properties
                // Priority: layout dimensions (from ELK) > expandedDimensions (internal) > default
                width: container.layout?.dimensions?.width ?? container.expandedDimensions?.width ?? 0,
                height: container.layout?.dimensions?.height ?? container.expandedDimensions?.height ?? 0,
                // Copy any other custom properties but exclude internal ones
                ...Object.fromEntries(Object.entries(container).filter(([key]) => !['layout', 'expandedDimensions', 'id', 'collapsed', 'hidden', 'children'].includes(key)))
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
     * Get all collapsed containers
     */
    get collapsedContainerNodes() {
        return Array.from(this.collapsedContainers.values());
    }
    /**
     * Get all hyper-edges that are currently visible
     */
    get visibleHyperEdges() {
        const regularEdges = Array.from(this._visibleEdges.values());
        // Include non-hidden hyperedges (these represent collapsed container connections)
        const activeHyperEdges = Array.from(this.hyperEdges.values()).filter(hyperEdge => !hyperEdge.hidden);
        // Return unified edge collection - external systems don't need to know about hyperedges
        return [...regularEdges, ...activeHyperEdges];
    }
    /**
     * Get all edges connected to a node
     */
    getEdgesForNode(nodeId) {
        return this.nodeToEdges.get(nodeId) || new Set();
    }
    /**
     * Clear all data
     */
    clear() {
        this._clearAllDataStructures();
    }
    // ============ Invariant Validation ============
    /**
     * Validate all internal hyperedge invariants
     * @param {string} [context=''] - Optional context string for error messages
     * @throws {Error} When any hyperedge invariant is violated
     * @example
     * ```javascript
     * state.validateHyperedgeInvariants('After container collapse');
     * ```
     */
    validateHyperedgeInvariants(context = '') {
        const contextPrefix = context ? `${context}: ` : '';
        // Get all active hyperedges (not hidden ones)
        const activeHyperEdges = Array.from(this.hyperEdges.values()).filter(he => !he.hidden);
        // Get visible nodes and containers for validation
        const visibleNodeIds = new Set(this.visibleNodes.map(n => n.id));
        const visibleContainers = this.visibleContainers;
        const collapsedContainerIds = new Set(visibleContainers.filter(c => c.collapsed).map(c => c.id));
        for (const hyperEdge of activeHyperEdges) {
            // INVARIANT 1: HyperEdges must have at least one collapsed container endpoint
            const sourceIsCollapsedContainer = collapsedContainerIds.has(hyperEdge.source);
            const targetIsCollapsedContainer = collapsedContainerIds.has(hyperEdge.target);
            if (!sourceIsCollapsedContainer && !targetIsCollapsedContainer) {
                throw new Error(`${contextPrefix}HyperEdge ${hyperEdge.id} violates invariant: must have at least one collapsed container endpoint (source: ${hyperEdge.source}, target: ${hyperEdge.target})`);
            }
            // INVARIANT 2: All hyperedge endpoints must be visible (either visible nodes or visible containers)
            const sourceIsVisibleNode = visibleNodeIds.has(hyperEdge.source);
            const targetIsVisibleNode = visibleNodeIds.has(hyperEdge.target);
            const sourceIsVisibleContainer = visibleContainers.some(c => c.id === hyperEdge.source);
            const targetIsVisibleContainer = visibleContainers.some(c => c.id === hyperEdge.target);
            if (!(sourceIsVisibleNode || sourceIsVisibleContainer)) {
                throw new Error(`${contextPrefix}HyperEdge ${hyperEdge.id} violates invariant: source ${hyperEdge.source} must be a visible node or container`);
            }
            if (!(targetIsVisibleNode || targetIsVisibleContainer)) {
                throw new Error(`${contextPrefix}HyperEdge ${hyperEdge.id} violates invariant: target ${hyperEdge.target} must be a visible node or container`);
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
    collapseContainer(containerId) {
        this.collapseExpandEngine.collapseContainer(containerId);
    }
    /**
     * Expand a container (depth-first, top-down with edge grounding)
     * SYMMETRIC INVERSE of collapseContainer()
     */
    expandContainer(containerId) {
        this.collapseExpandEngine.expandContainer(containerId);
    }
    /**
     * Get the children of a container
     */
    getContainerChildren(containerId) {
        return __classPrivateFieldGet(this, _VisualizationState_containerChildren, "f").get(containerId) || new Set();
    }
    /**
     * Get the parent container of a node or container
     */
    getParentContainer(childId) {
        return __classPrivateFieldGet(this, _VisualizationState_nodeContainers, "f").get(childId);
    }
    /**
     * Get a specific graph node by ID
     */
    getGraphNode(id) {
        return this.graphNodes.get(id);
    }
    /**
     * Get a specific graph edge by ID
     */
    getGraphEdge(id) {
        return this.graphEdges.get(id);
    }
    /**
     * Get a specific container by ID
     */
    getContainer(id) {
        return this.containers.get(id);
    }
    /**
     * Get a specific hyper-edge by ID
     */
    getHyperEdge(id) {
        return this.hyperEdges.get(id);
    }
    /**
     * Get a specific node, edge, or container by ID
     */
    getEntity(id) {
        const node = this.getGraphNode(id);
        if (node) {
            return node;
        }
        const edge = this.getGraphEdge(id);
        if (edge) {
            return edge;
        }
        const container = this.getContainer(id);
        if (container) {
            return container;
        }
        return this.getHyperEdge(id);
    }
    /**
     * Add a node to the graph
     */
    addNode(node) {
        this._validateEntity(node, ['id']);
        this.graphNodes.set(node.id, node);
        this._updateVisibility(node.id, node);
    }
    /**
     * Add an edge to the graph
     */
    addEdge(edge) {
        this._validateEntity(edge, ['id', 'source', 'target']);
        this.graphEdges.set(edge.id, edge);
        this._updateVisibility(edge.id, edge);
    }
    /**
     * Add a container to the graph
     */
    addContainer(container) {
        this._validateEntity(container, ['id']);
        this.containers.set(container.id, container);
        this._updateVisibility(container.id, container);
    }
    /**
     * Add a hyper-edge to the graph
     */
    addHyperEdge(hyperEdge) {
        this._validateEntity(hyperEdge, ['id']);
        this.hyperEdges.set(hyperEdge.id, hyperEdge);
        this._updateVisibility(hyperEdge.id, hyperEdge);
    }
    /**
     * Update a node's properties
     */
    updateNode(id, updates) {
        const node = this.getGraphNode(id);
        this._validateEntity(node);
        Object.assign(node, updates);
        this._updateVisibility(id, node);
    }
    /**
     * Update an edge's properties
     */
    updateEdge(id, updates) {
        const edge = this.getGraphEdge(id);
        this._validateEntity(edge);
        Object.assign(edge, updates);
        this._updateVisibility(id, edge);
    }
    /**
     * Update a container's properties
     */
    updateContainer(id, updates) {
        const container = this.getContainer(id);
        this._validateEntity(container);
        Object.assign(container, updates);
        this._updateVisibility(id, container);
    }
    /**
     * Update a hyper-edge's properties
     */
    updateHyperEdge(id, updates) {
        const hyperEdge = this.getHyperEdge(id);
        this._validateEntity(hyperEdge);
        Object.assign(hyperEdge, updates);
        this._updateVisibility(id, hyperEdge);
    }
    /**
     * Remove a node from the graph
     */
    removeNode(id) {
        if (!this.graphNodes.has(id)) {
            throw new Error(`Cannot remove node: node '${id}' does not exist`);
        }
        this._removeNodeFromAllStructures(id);
    }
    /**
     * Remove an edge from the graph
     */
    removeEdge(id) {
        this.graphEdges.delete(id);
        this._updateVisibility(id, undefined);
    }
    /**
     * Remove a container from the graph
     */
    removeContainer(id) {
        if (!this.containers.has(id)) {
            throw new Error(`Cannot remove container: container '${id}' does not exist`);
        }
        this._removeContainerFromAllStructures(id);
    }
    /**
     * Remove a hyper-edge from the graph
     */
    removeHyperEdge(id) {
        if (!this.hyperEdges.has(id)) {
            throw new Error(`Cannot remove hyperEdge: hyperEdge '${id}' does not exist`);
        }
        this._removeHyperEdgeFromAllStructures(id);
    }
    /**
     * Clear the entire graph
     */
    clearAll() {
        this.graphNodes.clear();
        this.graphEdges.clear();
        this.containers.clear();
        this.hyperEdges.clear();
        this.manualPositions.clear();
        this.collapsedContainers.clear();
        this._visibleNodes.clear();
        this._visibleEdges.clear();
        this._visibleContainers.clear();
        this._expandedContainers.clear();
    }
    /**
     * Get all manual positions
     * @returns {Map<string, {x: number, y: number}>} Copy of manual positions
     */
    get allManualPositions() {
        return new Map(this.manualPositions);
    }
    /**
     * Set a manual position for a node or container
     * @param {string} id - The element ID
     * @param {{x: number, y: number}} position - The position object
     */
    setManualPosition(id, position) {
        this._validateRequiredString(id, 'Element ID');
        this.manualPositions.set(id, position);
    }
    /**
     * Get manual position for a node or container
     * @param {string} id - The element ID
     * @returns {{x: number, y: number} | undefined} The position object or undefined if not set
     */
    getManualPosition(id) {
        return this.manualPositions.get(id);
    }
    /**
     * Clear manual position for a node or container
     * @param {string} id - The element ID
     */
    clearManualPosition(id) {
        this.manualPositions.delete(id);
    }
    /**
     * Clear all manual position overrides
     * Called during resets to ensure clean state
     */
    clearAllManualPositions() {
        this.manualPositions.clear();
    }
    /**
     * Get all visible (non-hidden) nodes with computed position/dimension properties
     */
    get visibleNodes() {
        return Array.from(this._visibleNodes.values());
    }
    /**
     * Get all visible (non-hidden) edges, including hyperedges when appropriate
     * This provides a unified view of edges for external systems (ELK, ReactFlow)
     * Hyperedges are included when their corresponding containers are collapsed
     */
    get visibleEdges() {
        return Array.from(this._visibleEdges.values());
    }
    /**
     * Get all visible (non-hidden) containers with computed position/dimension properties
     */
    get visibleContainers() {
        return Array.from(this._visibleContainers.values()).map(container => {
            // Create a computed view that exposes layout data as direct properties
            const computedContainer = {
                id: container.id,
                collapsed: container.collapsed,
                hidden: container.hidden,
                children: container.children,
                // Expose layout position as direct x, y properties
                x: container.layout?.position?.x ?? 0,
                y: container.layout?.position?.y ?? 0,
                // Expose layout dimensions as direct width, height properties
                // Priority: layout dimensions (from ELK) > expandedDimensions (internal) > default
                width: container.layout?.dimensions?.width ?? container.expandedDimensions?.width ?? 0,
                height: container.layout?.dimensions?.height ?? container.expandedDimensions?.height ?? 0,
                // Copy any other custom properties but exclude internal ones
                ...Object.fromEntries(Object.entries(container).filter(([key]) => !['layout', 'expandedDimensions', 'id', 'collapsed', 'hidden', 'children'].includes(key)))
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
     * Get all collapsed containers
     */
    get collapsedContainerNodes() {
        return Array.from(this.collapsedContainers.values());
    }
    /**
     * Get all hyper-edges that are currently visible
     */
    get visibleHyperEdges() {
        const regularEdges = Array.from(this._visibleEdges.values());
        // Include non-hidden hyperedges (these represent collapsed container connections)
        const activeHyperEdges = Array.from(this.hyperEdges.values()).filter(hyperEdge => !hyperEdge.hidden);
        // Return unified edge collection - external systems don't need to know about hyperedges
        return [...regularEdges, ...activeHyperEdges];
    }
    /**
     * Get all edges connected to a node
     */
    getEdgesForNode(nodeId) {
        return this.nodeToEdges.get(nodeId) || new Set();
    }
    /**
     * Clear all data
     */
    clear() {
        this._clearAllDataStructures();
    }
    // ============ Invariant Validation ============
    /**
     * Validate all internal hyperedge invariants
     * @param {string} [context=''] - Optional context string for error messages
     * @throws {Error} When any hyperedge invariant is violated
     * @example
     * ```javascript
     * state.validateHyperedgeInvariants('After container collapse');
     * ```
     */
    validateHyperedgeInvariants(context = '') {
        const contextPrefix = context ? `${context}: ` : '';
        // Get all active hyperedges (not hidden ones)
        const activeHyperEdges = Array.from(this.hyperEdges.values()).filter(he => !he.hidden);
        // Get visible nodes and containers for validation
        const visibleNodeIds = new Set(this.visibleNodes.map(n => n.id));
        const visibleContainers = this.visibleContainers;
        const collapsedContainerIds = new Set(visibleContainers.filter(c => c.collapsed).map(c => c.id));
        for (const hyperEdge of activeHyperEdges) {
            // INVARIANT 1: HyperEdges must have at least one collapsed container endpoint
            const sourceIsCollapsedContainer = collapsedContainerIds.has(hyperEdge.source);
            const targetIsCollapsedContainer = collapsedContainerIds.has(hyperEdge.target);
            if (!sourceIsCollapsedContainer && !targetIsCollapsedContainer) {
                throw new Error(`${contextPrefix}HyperEdge ${hyperEdge.id} violates invariant: must have at least one collapsed container endpoint (source: ${hyperEdge.source}, target: ${hyperEdge.target})`);
            }
            // INVARIANT 2: All hyperedge endpoints must be visible (either visible nodes or visible containers)
            const sourceIsVisibleNode = visibleNodeIds.has(hyperEdge.source);
            const targetIsVisibleNode = visibleNodeIds.has(hyperEdge.target);
            const sourceIsVisibleContainer = visibleContainers.some(c => c.id === hyperEdge.source);
            const targetIsVisibleContainer = visibleContainers.some(c => c.id === hyperEdge.target);
            if (!(sourceIsVisibleNode || sourceIsVisibleContainer)) {
                throw new Error(`${contextPrefix}HyperEdge ${hyperEdge.id} violates invariant: source ${hyperEdge.source} must be a visible node or container`);
            }
            if (!(targetIsVisibleNode || targetIsVisibleContainer)) {
                throw new Error(`${contextPrefix}HyperEdge ${hyperEdge.id} violates invariant: target ${hyperEdge.target} must be a visible node or container`);
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
    collapseContainer(containerId) {
        this.collapseExpandEngine.collapseContainer(containerId);
    }
    /**
     * Expand a container (depth-first, top-down with edge grounding)
     * SYMMETRIC INVERSE of collapseContainer()
     */
    expandContainer(containerId) {
        this.collapseExpandEngine.expandContainer(containerId);
    }
    // ============ Private Helper Methods ============
    // ============ Entity Creation Helpers ============
    /**
     * Add a node to all related data structures
     * @param {string} id - The node ID
     * @param {Object} node - The node object
     * @private
     */
    _addNodeToAllStructures(id, node) {
        // Add to main collection
        this.graphNodes.set(id, node);
        // Update visibility collection
        this._updateVisibleNodes(id, node);
    }
    /**
     * Add edge to node mapping for tracking
     * @private
     */
    _addEdgeToNodeMapping(edgeId, source, target) {
        // Add edge to source node's edge set
        if (!this.nodeToEdges.has(source)) {
            this.nodeToEdges.set(source, new Set());
        }
        this.nodeToEdges.get(source).add(edgeId);
        // Add edge to target node's edge set
        if (!this.nodeToEdges.has(target)) {
            this.nodeToEdges.set(target, new Set());
        }
        this.nodeToEdges.get(target).add(edgeId);
    }
    /**
     * Remove edge from node mapping
     * @private
     */
    _removeEdgeFromNodeMapping(edgeId, source, target) {
        // Remove edge from source node's edge set
        this.nodeToEdges.get(source)?.delete(edgeId);
        // Remove edge from target node's edge set
        this.nodeToEdges.get(target)?.delete(edgeId);
    }
    /**
     * Update expanded containers visibility
     * @private
     */
    _updateExpandedContainers(id, container) {
        this._updateVisibilityMap(this._expandedContainers, id, container && !container.collapsed ? container : undefined);
    }
    /**
     * Add an edge to all related data structures
     * @param {string} id - The edge ID
     * @param {Object} edge - The edge object
     * @param {string} source - The source node/container ID
     * @param {string} target - The target node/container ID
     * @private
     */
    _addEdgeToAllStructures(id, edge, source, target) {
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
    _addContainerToAllStructures(id, container) {
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
        const parentId = __classPrivateFieldGet(this, _VisualizationState_nodeContainers, "f").get(id);
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
        __classPrivateFieldGet(this, _VisualizationState_containerChildren, "f").clear();
        __classPrivateFieldGet(this, _VisualizationState_nodeContainers, "f").clear();
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
            __classPrivateFieldGet(this, _VisualizationState_containerChildren, "f").set(containerId, container.children);
            __classPrivateFieldGet(this, _VisualizationState_nodeContainers, "f").set(childId, containerId);
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
            __classPrivateFieldGet(this, _VisualizationState_containerChildren, "f").set(containerId, container.children);
            __classPrivateFieldGet(this, _VisualizationState_nodeContainers, "f").delete(childId);
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
        __classPrivateFieldGet(this, _VisualizationState_containerChildren, "f").set(containerId, children);
        // Add each child to the nodeContainers mapping
        for (const childId of children) {
            __classPrivateFieldGet(this, _VisualizationState_nodeContainers, "f").set(childId, containerId);
        }
    }
    /**
     * Clean up container hierarchy when removing a container
     * @param {string} containerId - The container ID being removed
     * @private
     */
    _cleanupContainerHierarchy(containerId) {
        // First, remove this container from its parent's children list
        const parentId = __classPrivateFieldGet(this, _VisualizationState_nodeContainers, "f").get(containerId);
        if (parentId) {
            this._removeChildFromContainerHierarchy(parentId, containerId);
        }
        // Then clean up this container's children using encapsulated method
        const children = __classPrivateFieldGet(this, _VisualizationState_containerChildren, "f").get(containerId);
        if (children) {
            // Create a copy to avoid modification during iteration
            const childrenArray = Array.from(children);
            for (const childId of childrenArray) {
                this._removeChildFromContainerHierarchy(containerId, childId);
            }
        }
        // Final cleanup of the container's own mapping
        __classPrivateFieldGet(this, _VisualizationState_containerChildren, "f").delete(containerId);
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
            }
            else if (this.containers.has(childId)) {
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
    _updateVisibility(id, entity) {
        if (entity.hidden) {
            this._visibleNodes.delete(id);
            this._visibleEdges.delete(id);
            this._visibleContainers.delete(id);
            this._expandedContainers.delete(id);
        }
        else if (this.graphNodes.has(id)) {
            this._updateVisibleNodes(id, entity);
        }
        else if (this.graphEdges.has(id)) {
            this._updateVisibleEdges(id, entity);
        }
        else if (this.containers.has(id)) {
            this._updateVisibleContainers(id, entity);
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
        this._updateVisibilityMap(this._expandedContainers, id, container && !container.collapsed ? container : undefined);
    }
    _updateVisibilityMap(map, id, entity) {
        if (entity.hidden) {
            map.delete(id);
        }
        else {
            map.set(id, entity);
        }
    }
    /**
     * Get the layout of a node
     */
    setNodeLayout(id, layout) {
        const node = this.getGraphNode(id);
        this._validateEntity(node);
        if (!node.layout) {
            node.layout = {};
        }
        Object.assign(node.layout, layout);
    }
    getNodeLayout(id) {
        return this.getGraphNode(id)?.layout;
    }
    setEdgeLayout(id, layout) {
        const edge = this.getGraphEdge(id);
        this._validateEntity(edge);
        if (!edge.layout) {
            edge.layout = {};
        }
        Object.assign(edge.layout, layout);
    }
    getEdgeLayout(id) {
        return this.getGraphEdge(id)?.layout;
    }
    setContainerLayout(id, layout) {
        const container = this.getContainer(id);
        this._validateEntity(container);
        if (!container.layout) {
            container.layout = {};
        }
        Object.assign(container.layout, layout);
    }
    getContainerLayout(id) {
        return this.getContainer(id)?.layout;
    }
    getContainerCollapsed(id) {
        return this.getContainer(id)?.collapsed;
    }
    setContainerCollapsed(id, collapsed) {
        this.updateContainer(id, { collapsed });
    }
    getContainersRequiringLayout(changedContainerId) {
        // For now, return all containers. In the future, optimize this.
        return Array.from(this.containers.values());
    }
    /**
     * Get all nodes that are currently visible and require layouting
     */
    getNodesRequiringLayout(changedNodeId) {
        const nodes = this.visibleNodes;
        // Apply CENTRALIZED position fixing logic: 
        // Everything FIXED except the node that changed
        return nodes.map(node => {
            const shouldBeFixed = changedNodeId && node.id !== changedNodeId;
            // Ensure elkFixed is set in VisState
            this.setContainerELKFixed(node.id, shouldBeFixed);
            return node;
        });
    }
    /**
     * Get all edges that are currently visible and require layouting
     */
    getEdgesRequiringLayout(changedEdgeId) {
        // For now, return all visible edges. In the future, optimize this.
        return this.visibleEdges;
    }
    /**
     * Get all hyper-edges that are currently visible and require layouting
     */
    getHyperEdgesRequiringLayout(changedHyperEdgeId) {
        // For now, return all visible hyperEdges. In the future, optimize this.
        return Array.from(this.hyperEdges.values()).filter(he => !he.hidden);
    }
    /**
     * Get all entities that are currently visible and require layouting
     */
    getEntitiesRequiringLayout(changedEntityId) {
        return {
            nodes: this.getNodesRequiringLayout(changedEntityId),
            edges: this.getEdgesRequiringLayout(changedEntityId),
            containers: this.getContainersRequiringLayout(changedEntityId),
            hyperEdges: this.getHyperEdgesRequiringLayout(changedEntityId)
        };
    }
    /**
     * Validate that an entity exists
     */
    _validateEntity(entity, requiredFields = []) {
        if (!entity) {
            throw new Error(`Entity does not exist`);
        }
        for (const field of requiredFields) {
            if (entity[field] === undefined) {
                throw new Error(`Entity is missing required field: ${field}`);
            }
        }
    }
}
_VisualizationState_containerChildren = new WeakMap(), _VisualizationState_nodeContainers = new WeakMap();
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
 * console.log(state.getGraphNode('node1')); // { id: 'node1', label: 'My First Node', ... }
 * ```
 */
export function createVisualizationState() {
    return new VisualizationState();
}
// Implement the interface methods
Object.assign(VisualizationState.prototype, {
    setNodeHidden(id, hidden) {
        this.updateNode(id, { hidden });
    },
    getNodeHidden(id) {
        return this.getGraphNode(id)?.hidden;
    },
    setEdgeHidden(id, hidden) {
        this.updateEdge(id, { hidden });
    },
    getEdgeHidden(id) {
        return this.getGraphEdge(id)?.hidden;
    },
    setContainerCollapsed(id, collapsed) {
        this.updateContainer(id, { collapsed });
    },
    getContainerCollapsed(id) {
        return this.getContainer(id)?.collapsed;
    },
    setContainerHidden(id, hidden) {
        this.updateContainer(id, { hidden });
    },
    getContainerHidden(id) {
        return this.getContainer(id)?.hidden;
    }
});
// ============ CENTRALIZED LAYOUT STATE MANAGEMENT ============
// ALL layout information flows through VisState - ELK and ReactFlow get data from here
Object.assign(VisualizationState.prototype, {
    // Node layout methods
    setNodeLayout(id, layout) {
        const node = this.getGraphNode(id);
        this._validateEntity(node);
        if (!node.layout) {
            node.layout = {};
        }
        Object.assign(node.layout, layout);
    },
    getNodeLayout(id) {
        return this.getGraphNode(id)?.layout;
    },
    // Edge layout methods
    setEdgeLayout(id, layout) {
        const edge = this.getGraphEdge(id);
        this._validateEntity(edge);
        if (!edge.layout) {
            edge.layout = {};
        }
        Object.assign(edge.layout, layout);
    },
    getEdgeLayout(id) {
        return this.getGraphEdge(id)?.layout;
    },
    // Container layout methods
    setContainerLayout(id, layout) {
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
                console.log(`[VisState] 📏 Auto-updated expandedDimensions for ${id}: ${container.expandedDimensions.width}x${container.expandedDimensions.height}`);
            }
        }
    },
    getContainerLayout(id) {
        return this.getContainer(id)?.layout;
    },
    // ELK position fixing methods
    setContainerELKFixed(id, fixed) {
        this.setContainerLayout(id, { elkFixed: fixed });
    },
    getContainerELKFixed(id) {
        return this.getContainerLayout(id)?.elkFixed;
    },
    getContainersRequiringLayout(changedContainerId) {
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
    /**
     * Set a manual position for a node or container
     * @param {string} id - The element ID
     * @param {{x: number, y: number}} position - The position object
     */
    setManualPosition(id, position) {
        this._validateRequiredString(id, 'Element ID');
        this.manualPositions.set(id, position);
    }
});
//# sourceMappingURL=VisState.js.map