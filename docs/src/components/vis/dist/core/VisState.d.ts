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
    getParentContainer(nodeId: string): string | undefined;
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
    private readonly manualPositions;
    private readonly collapseExpandEngine;
    /**
     * Create a new VisualizationState instance
     * @constructor
     */
    constructor();
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
    setHyperEdge(id: string, { source, target, style, hidden, ...otherProps }: {
        source: string;
        target: string;
        style?: string;
        hidden?: boolean;
        [key: string]: any;
    }): this;
    /**
     * Get all containers
     */
    get allContainers(): any[];
    /**
     * Get all visible (non-hidden) nodes with computed position/dimension properties
     */
    get visibleNodes(): any[];
    /**
     * Get all visible (non-hidden) edges, including hyperedges when appropriate
     * This provides a unified view of edges for external systems (ELK, ReactFlow)
     * Hyperedges are included when their corresponding containers are collapsed
     */
    get visibleEdges(): any[];
    /**
     * Get all visible (non-hidden) containers with computed position/dimension properties
     */
    get visibleContainers(): {
        id: any;
        collapsed: any;
        hidden: any;
        children: any;
        x: any;
        y: any;
        width: any;
        height: any;
    }[];
    /**
     * Get all expanded (non-collapsed) containers
     */
    get expandedContainers(): any[];
    /**
     * Get all collapsed containers
     */
    get collapsedContainerNodes(): any[];
    /**
     * Get all hyper-edges that are currently visible
     */
    get visibleHyperEdges(): any[];
    /**
     * Get the children of a container
     */
    getContainerChildren(containerId: string): Set<string>;
    /**
     * Get the parent container of a node or container
     */
    getParentContainer(childId: string): string | undefined;
    /**
     * Get a specific node, edge, or container by ID
     */
    getEntity(id: string): any;
    /**
     * Add a node to the graph
     */
    addNode(node: any): void;
    /**
     * Add an edge to the graph
     */
    addEdge(edge: any): void;
    /**
     * Add a container to the graph
     */
    addContainer(container: any): void;
    /**
     * Add a hyper-edge to the graph
     */
    addHyperEdge(hyperEdge: any): void;
    /**
     * Update a hyper-edge's properties
     */
    updateHyperEdge(id: string, updates: Partial<any>): void;
    /**
     * Remove a node from the graph
     */
    removeNode(id: string): void;
    /**
     * Remove an edge from the graph
     */
    removeEdge(id: string): void;
    /**
     * Clear the entire graph
     */
    clearAll(): void;
    /**
     * Get all manual positions
     * @returns {Map<string, {x: number, y: number}>} Copy of manual positions
     */
    get allManualPositions(): Map<string, {
        x: number;
        y: number;
    }>;
    /**
     * Get all visible (non-hidden) nodes with computed position/dimension properties
     */
    get visibleNodes(): any[];
    /**
     * Get all visible (non-hidden) edges, including hyperedges when appropriate
     * This provides a unified view of edges for external systems (ELK, ReactFlow)
     * Hyperedges are included when their corresponding containers are collapsed
     */
    get visibleEdges(): any[];
    /**
     * Get all visible (non-hidden) containers with computed position/dimension properties
     */
    get visibleContainers(): {
        id: any;
        collapsed: any;
        hidden: any;
        children: any;
        x: any;
        y: any;
        width: any;
        height: any;
    }[];
    /**
     * Get all expanded (non-collapsed) containers
     */
    get expandedContainers(): any[];
    /**
     * Get all collapsed containers
     */
    get collapsedContainerNodes(): any[];
    /**
     * Get all hyper-edges that are currently visible
     */
    get visibleHyperEdges(): any[];
    /**
     * Add a node to all related data structures
     * @param {string} id - The node ID
     * @param {Object} node - The node object
     * @private
     */
    _addNodeToAllStructures(id: string, node: any): void;
    /**
     * Add edge to node mapping for tracking
     * @private
     */
    _addEdgeToNodeMapping(edgeId: string, source: string, target: string): void;
    /**
     * Remove edge from node mapping
     * @private
     */
    _removeEdgeFromNodeMapping(edgeId: string, source: string, target: string): void;
    /**
     * Update expanded containers visibility
     * @private
     */
    _updateExpandedContainers(id: string, container: any): void;
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
    _updateVisibility(id: any, entity: any): void;
    _updateVisibleNodes(id: any, node: any): void;
    _updateVisibleEdges(id: any, edge: any): void;
    _updateVisibleContainers(id: any, container: any): void;
    _updateVisibilityMap(map: any, id: any, entity: any): void;
    /**
     * Get all nodes that are currently visible and require layouting
     */
    getNodesRequiringLayout(changedNodeId?: string): any[];
    /**
     * Get all edges that are currently visible and require layouting
     */
    getEdgesRequiringLayout(changedEdgeId?: string): any[];
    /**
     * Get all hyper-edges that are currently visible and require layouting
     */
    getHyperEdgesRequiringLayout(changedHyperEdgeId?: string): any[];
    /**
     * Get all entities that are currently visible and require layouting
     */
    getEntitiesRequiringLayout(changedEntityId?: string): {
        nodes: any[];
        edges: any[];
        containers: any[];
        hyperEdges: any[];
    };
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
        setNodeLayout(id: string, layout: Partial<import('../shared/types').LayoutState>): void;
        getNodeLayout(id: string): import('../shared/types').LayoutState | undefined;
        setEdgeLayout(id: string, layout: Partial<import('../shared/types').LayoutState>): void;
        getEdgeLayout(id: string): import('../shared/types').LayoutState | undefined;
        setContainerLayout(id: string, layout: Partial<import('../shared/types').LayoutState>): void;
        getContainerLayout(id: string): import('../shared/types').LayoutState | undefined;
        setContainerELKFixed(id: string, fixed: boolean): void;
        getContainerELKFixed(id: string): boolean | undefined;
        getContainersRequiringLayout(changedContainerId?: string): import('../shared/types').Container[];
        setManualPosition(elementId: string, x: number, y: number): void;
        getManualPosition(elementId: string): {
            x: number;
            y: number;
        } | null;
        hasManualPosition(elementId: string): boolean;
        clearManualPosition(elementId: string): void;
        clearAllManualPositions(): void;
        getAllManualPositions(): Map<string, {
            x: number;
            y: number;
        }>;
        hasAnyManualPositions(): boolean;
        collapseContainer(containerId: string): void;
        expandContainer(containerId: string): void;
    }
}
//# sourceMappingURL=VisState.d.ts.map