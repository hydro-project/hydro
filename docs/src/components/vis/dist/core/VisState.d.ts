/**
 * Visualization State - Core Data Structure
 *
 * Maintains the mutable state of the visualization including nodes, edges, containers, and hyperEdges.
 * Provides efficient access to visible/non-hidden elements through Maps and collections.
 */
import { CreateNodeProps, CreateEdgeProps, CreateContainerProps } from '../shared/types';
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
 * console.log(state.visibleNodes);     // Array of visible nodes
 * console.log(state.expandedContainers); // Array of expanded containers
 *
 * // Update properties idiomatically
 * state.updateNode('n1', { hidden: true, style: 'highlighted' });
 * state.updateContainer('c1', { collapsed: true });
 * ```
 */
export declare class VisualizationState implements ContainerHierarchyView {
    #private;
    private readonly graphNodes;
    private readonly graphEdges;
    private readonly containers;
    private readonly hyperEdges;
    private readonly _visibleNodes;
    private readonly _visibleEdges;
    private readonly _visibleContainers;
    private readonly _expandedContainers;
    private readonly collapsedContainers;
    private readonly nodeToEdges;
    private readonly collapseExpandEngine;
    /**
     * Create a new VisualizationState instance
     * @constructor
     */
    constructor();
    /**
     * Validate that an entity exists and optionally check a condition
     * @param {Object|null} entity - The entity object to validate
     * @param {Function} [conditionFn] - Optional condition function to check
     * @throws {Error} When entity doesn't exist or condition fails
     */
    _validateEntity(entity: any, conditionFn?: any): boolean;
    /**
     * Validate required string parameter
     * @param {any} value - The value to validate
     * @param {string} fieldName - Name of the field for error messages
     * @throws {Error} When value is not a non-empty string
     */
    _validateRequiredString(value: any, fieldName: any): void;
    /**
     * Validate style parameter against allowed values
     * @param {any} style - The style value to validate
     * @param {Object} allowedStyles - Object containing allowed style values
     * @param {string} entityType - Type of entity for error messages
     * @throws {Error} When style is not in allowed values
     */
    _validateStyle(style: any, allowedStyles: any, entityType: any): void;
    /**
     * Generic method to get an entity from any collection
     */
    _getEntity(entityType: string, id: string): any;
    /**
     * Get the main collection for an entity type
     */
    _getEntityCollection(entityType: string): Map<string, any>;
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
    setGraphNode(id: string, props: CreateNodeProps): this;
    /**
     * Get a graph node by id
     * @param {string} id - The node ID to retrieve
     * @returns {Object|undefined} The node object or undefined if not found
     */
    getGraphNode(id: string): any;
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
    updateNode(id: string, updates: {
        hidden?: boolean;
        style?: string;
        label?: string;
    }): this;
    /**
     * Remove a graph node
     * @param {string} id - The node ID to remove
     * @throws {Error} When node doesn't exist
     */
    removeGraphNode(id: string): void;
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
    setGraphEdge(id: string, props: CreateEdgeProps): this;
    /**
     * Get a graph edge by id
     * @param {string} id - The edge ID to retrieve
     * @returns {Object|undefined} The edge object or undefined if not found
     */
    getGraphEdge(id: string): any;
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
    updateEdge(id: string, updates: {
        hidden?: boolean;
        style?: string;
    }): this;
    /**
     * Remove a graph edge
     * @param {string} id - The edge ID to remove
     * @throws {Error} When edge doesn't exist
     */
    removeGraphEdge(id: string): void;
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
    setContainer(id: string, props: CreateContainerProps): this;
    /**
     * Get a container by id
     * @param {string} id - The container ID to retrieve
     * @returns {Object|undefined} The container object or undefined if not found
     */
    getContainer(id: string): any;
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
    updateContainer(id: string, updates: {
        collapsed?: boolean;
        hidden?: boolean;
        label?: string;
    }): this;
    /**
     * Add a child to a container
     * @param {string} containerId - The container ID
     * @param {string} childId - The child node/container ID to add
     * @throws {Error} When container doesn't exist or parameters are invalid
     */
    addContainerChild(containerId: string, childId: string): void;
    /**
     * Remove a child from a container
     * @param {string} containerId - The container ID
     * @param {string} childId - The child node/container ID to remove
     * @throws {Error} When container doesn't exist or parameters are invalid
     */
    removeContainerChild(containerId: string, childId: string): void;
    /**
     * Remove a container
     * @param {string} id - The container ID to remove
     * @throws {Error} When container doesn't exist
     */
    removeContainer(id: string): void;
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
    setHyperEdge(id: string, { source, target, style, ...otherProps }: {
        source: string;
        target: string;
        style?: string;
        [key: string]: any;
    }): this;
    /**
     * Get a hyper edge by id
     * @param {string} id - The hyperEdge ID to retrieve
     * @returns {Object|undefined} The hyperEdge object or undefined if not found
     */
    getHyperEdge(id: string): any;
    /**
     * Remove a hyper edge
     * @param {string} id - The hyperEdge ID to remove
     * @throws {Error} When hyperEdge doesn't exist
     */
    removeHyperEdge(id: string): void;
    /**
     * Get all visible (non-hidden) nodes
     */
    get visibleNodes(): any[];
    /**
     * Get all visible (non-hidden) edges
     */
    get visibleEdges(): any[];
    /**
     * Get all visible (non-hidden) containers
     */
    get visibleContainers(): any[];
    /**
     * Get all expanded (non-collapsed) containers
     */
    get expandedContainers(): any[];
    /**
     * Get all hyper edges
     */
    get allHyperEdges(): any[];
    /**
     * Get container children for a container id
     * Returns a readonly Set to prevent external modification
     */
    getContainerChildren(containerId: string): ReadonlySet<string>;
    /**
     * Get the container that contains a given node
     */
    getNodeContainer(nodeId: string): string | undefined;
    /**
     * Clear all data
     */
    clear(): void;
    /**
     * Collapse a container (depth-first, bottom-up with edge lifting)
     * Uses optimized engine with tree hierarchy validation and edge indexing
     */
    collapseContainer(containerId: string): void;
    /**
     * Expand a container (depth-first, top-down with edge grounding)
     * SYMMETRIC INVERSE of collapseContainer()
     */
    expandContainer(containerId: string): void;
    /**
     * Add a node to all related data structures
     * @param {string} id - The node ID
     * @param {Object} node - The node object
     * @private
     */
    _addNodeToAllStructures(id: string, node: any): void;
    /**
     * Add an edge to all related data structures
     * @param {string} id - The edge ID
     * @param {Object} edge - The edge object
     * @param {string} source - The source node/container ID
     * @param {string} target - The target node/container ID
     * @private
     */
    _addEdgeToAllStructures(id: string, edge: any, source: string, target: string): void;
    /**
     * Add a container to all related data structures
     * @param {string} id - The container ID
     * @param {Object} container - The container object
     * @private
     */
    _addContainerToAllStructures(id: string, container: any): void;
    /**
     * Add a hyperEdge to all related data structures
     * @param {string} id - The hyperEdge ID
     * @param {Object} hyperEdge - The hyperEdge object
     * @private
     */
    _addHyperEdgeToAllStructures(id: any, hyperEdge: any): void;
    /**
     * Remove a node from all related data structures
     * @param {string} id - The node ID to remove
     * @private
     */
    _removeNodeFromAllStructures(id: any): void;
    /**
     * Remove an edge from all related data structures
     * @param {string} id - The edge ID to remove
     * @private
     */
    _removeEdgeFromAllStructures(id: any): void;
    /**
     * Remove a container from all related data structures
     * @param {string} id - The container ID to remove
     * @private
     */
    _removeContainerFromAllStructures(id: any): void;
    /**
     * Remove a hyperEdge from all related data structures
     * @param {string} id - The hyperEdge ID to remove
     * @private
     */
    _removeHyperEdgeFromAllStructures(id: any): void;
    /**
     * Clear all data structures in the correct order
     * @private
     */
    _clearAllDataStructures(): void;
    /**
     * Add a child to container hierarchy and maintain all indexes
     * @param {string} containerId - The container ID
     * @param {string} childId - The child node/container ID to add
     * @private
     */
    _addChildToContainerHierarchy(containerId: any, childId: any): void;
    /**
     * Remove a child from container hierarchy and maintain all indexes
     * @param {string} containerId - The container ID
     * @param {string} childId - The child node/container ID to remove
     * @private
     */
    _removeChildFromContainerHierarchy(containerId: any, childId: any): void;
    /**
     * Initialize container hierarchy for a new container with children
     * @param {string} containerId - The container ID
     * @param {Set<string>} children - The Set of child IDs
     * @private
     */
    _initializeContainerHierarchy(containerId: any, children: any): void;
    /**
     * Clean up container hierarchy when removing a container
     * @param {string} containerId - The container ID being removed
     * @private
     */
    _cleanupContainerHierarchy(containerId: any): void;
    /**
     * Check if an endpoint (node or container) is visible and should be connected to
     */
    _isEndpointConnectable(endpointId: any): boolean;
    /**
     * Categorize children into nodes and containers
     */
    _categorizeChildren(children: any): {
        containerNodes: Set<unknown>;
        childContainers: Set<unknown>;
    };
    /**
     * Apply visibility changes to a set of nodes
     */
    _setNodesVisibility(nodeIds: any, hidden: any): void;
    /**
     * Process hyperEdges by predicate and apply update function
     */
    _processHyperEdges(predicate: any, updateFn: any): void;
    /**
     * Generic visibility update method - consolidates _updateVisibleNodes, _updateVisibleEdges, _updateVisibleContainers
     */
    _updateVisibilityMap(visibilityMap: any, id: any, entity: any): void;
    _updateVisibleNodes(id: any, node: any): void;
    _updateVisibleEdges(id: any, edge: any): void;
    _updateVisibleContainers(id: any, container: any): void;
    _updateExpandedContainers(id: any, container: any): void;
    /**
     * Add edge to node mapping for efficient edge lookup
     */
    _addEdgeToNodeMapping(edgeId: any, sourceId: any, targetId: any): void;
    /**
     * Remove edge from node mapping
     */
    _removeEdgeFromNodeMapping(edgeId: any, sourceId: any, targetId: any): void;
    /**
     * Aggregate multiple edge styles into a single hyperEdge style
     */
    _aggregateEdgeStyles(edges: any): "default";
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
 * console.log(state.getGraphNode('node1')); // { id: 'node1', label: 'My First Node', ... }
 * ```
 */
export declare function createVisualizationState(): VisualizationState;
declare module './VisState.js' {
    interface VisualizationState {
        setNodeHidden(id: string, hidden: boolean): void;
        getNodeHidden(id: string): boolean | undefined;
        setEdgeHidden(id: string, hidden: boolean): void;
        getEdgeHidden(id: string): boolean | undefined;
        setContainerCollapsed(id: string, collapsed: boolean): void;
        getContainerCollapsed(id: string): boolean | undefined;
        setContainerHidden(id: string, hidden: boolean): void;
        getContainerHidden(id: string): boolean | undefined;
    }
}
//# sourceMappingURL=VisState.d.ts.map