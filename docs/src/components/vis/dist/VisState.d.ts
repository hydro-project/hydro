/**
 * Visualization State - Core Data Structure
 *
 * Maintains the mutable state of the visualization including nodes, edges, containers, and hyperEdges.
 * Provides efficient access to visible/non-hidden elements through Maps and collections.
 */
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
export declare class VisualizationState {
    private readonly graphNodes;
    private readonly graphEdges;
    private readonly containers;
    private readonly hyperEdges;
    private readonly visibleNodes;
    private readonly visibleEdges;
    private readonly visibleContainers;
    private readonly expandedContainers;
    private readonly collapsedContainers;
    private readonly containerChildren;
    private readonly nodeContainers;
    private readonly nodeToEdges;
    /**
     * Create a new VisualizationState instance
     * @constructor
     */
    constructor();
    /**
     * Validate that an entity exists and optionally check a condition
     * @param {string} entityType - The type of entity being validated
     * @param {string} id - The ID of the entity
     * @param {Object|null} entity - The entity object to validate
     * @param {string} operation - The operation being attempted
     * @param {Function} [conditionFn] - Optional condition function to check
     * @throws {Error} When entity doesn't exist or condition fails
     */
    _validateEntity(entityType: any, id: any, entity: any, operation: any, conditionFn?: any): boolean;
    /**
     * Validate that an entity exists and optionally check a condition (non-throwing version)
     * @param {Object|null} entity - The entity object to validate
     * @param {Function} [conditionFn] - Optional condition function to check
     * @returns {boolean} True if entity exists and passes condition
     */
    _validateEntitySafe(entity: any, conditionFn?: any): any;
    /**
     * Generic method to get an entity from any collection
     */
    _getEntity(entityType: any, id: any): any;
    /**
     * Generic method to set hidden flag for any entity type that supports it
     * @param {string} entityType - The type of entity
     * @param {string} id - The entity ID
     * @param {boolean} hidden - Whether the entity should be hidden
     * @throws {Error} When entity doesn't exist or doesn't support hiding
     */
    _setEntityHidden(entityType: any, id: any, hidden: any): void;
    /**
     * Generic method to get hidden flag for any entity type that supports it
     * @param {string} entityType - The type of entity
     * @param {string} id - The entity ID
     * @returns {boolean|undefined} The hidden flag or undefined if entity doesn't exist
     */
    _getEntityHidden(entityType: any, id: any): any;
    /**
     * Get the main collection for an entity type
     */
    _getEntityCollection(entityType: any): Map<string, any>;
    /**
     * Update visibility collections based on entity type and hidden state
     */
    _updateVisibilityCollection(entityType: any, id: any, entity: any): void;
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
    setGraphNode(id: string, { label, style, hidden, ...otherProps }: any): any;
    /**
     * Get a graph node by id
     * @param {string} id - The node ID to retrieve
     * @returns {Object|undefined} The node object or undefined if not found
     */
    getGraphNode(id: any): any;
    /**
     * Set hidden flag for a graph node
     * @param {string} id - The node ID
     * @param {boolean} hidden - Whether the node should be hidden
     * @throws {Error} When node doesn't exist
     */
    setNodeHidden(id: any, hidden: any): void;
    /**
     * Get hidden flag for a graph node
     * @param {string} id - The node ID
     * @returns {boolean|undefined} The hidden flag or undefined if node doesn't exist
     */
    getNodeHidden(id: any): any;
    /**
     * Remove a graph node
     * @param {string} id - The node ID to remove
     * @throws {Error} When node doesn't exist
     */
    removeGraphNode(id: any): void;
    /**
     * Add or update a graph edge
     */
    setGraphEdge(id: string, { source, target, style, hidden, ...otherProps }: any): any;
    /**
     * Get a graph edge by id
     */
    getGraphEdge(id: any): any;
    /**
     * Set hidden flag for a graph edge
     */
    setEdgeHidden(id: any, hidden: any): void;
    /**
     * Get hidden flag for a graph edge
     */
    getEdgeHidden(id: any): any;
    /**
     * Remove a graph edge
     */
    removeGraphEdge(id: any): void;
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
    setContainer(id: any, { expandedDimensions, collapsed, hidden, children, ...otherProps }: {
        [x: string]: any;
        expandedDimensions?: {
            width: number;
            height: number;
        };
        collapsed?: boolean;
        hidden?: boolean;
        children?: any[];
    }): {
        id: string;
        expandedDimensions: {
            width: number;
            height: number;
        };
        collapsed: boolean;
        hidden: boolean;
        children: Set<any>;
    };
    /**
     * Get a container by id
     * @param {string} id - The container ID to retrieve
     * @returns {Object|undefined} The container object or undefined if not found
     */
    getContainer(id: any): any;
    /**
     * Set collapsed flag for a container
     * @param {string} id - The container ID
     * @param {boolean} collapsed - Whether the container should be collapsed
     * @throws {Error} When container doesn't exist
     */
    setContainerCollapsed(id: any, collapsed: any): void;
    /**
     * Get collapsed flag for a container
     * @param {string} id - The container ID
     * @returns {boolean|undefined} The collapsed flag or undefined if container doesn't exist
     */
    getContainerCollapsed(id: any): any;
    /**
     * Set hidden flag for a container
     * @param {string} id - The container ID
     * @param {boolean} hidden - Whether the container should be hidden
     * @throws {Error} When container doesn't exist
     */
    setContainerHidden(id: any, hidden: any): void;
    /**
     * Get hidden flag for a container
     * @param {string} id - The container ID
     * @returns {boolean|undefined} The hidden flag or undefined if container doesn't exist
     */
    getContainerHidden(id: any): any;
    /**
     * Add a child to a container
     * @param {string} containerId - The container ID
     * @param {string} childId - The child node/container ID to add
     * @throws {Error} When container doesn't exist
     */
    addContainerChild(containerId: any, childId: any): void;
    /**
     * Remove a child from a container
     * @param {string} containerId - The container ID
     * @param {string} childId - The child node/container ID to remove
     * @throws {Error} When container doesn't exist
     */
    removeContainerChild(containerId: any, childId: any): void;
    /**
     * Remove a container
     */
    removeContainer(id: any): void;
    /**
     * Add or update a hyper edge
     */
    setHyperEdge(id: any, { source, target, style, ...otherProps }: {
        [x: string]: any;
        source: any;
        target: any;
        style?: "default";
    }): {
        id: any;
        source: any;
        target: any;
        style: "default";
    };
    /**
     * Get a hyper edge by id
     */
    getHyperEdge(id: any): any;
    /**
     * Remove a hyper edge
     */
    removeHyperEdge(id: any): void;
    /**
     * Get all visible (non-hidden) nodes
     */
    getVisibleNodes(): any[];
    /**
     * Get all visible (non-hidden) edges
     */
    getVisibleEdges(): any[];
    /**
     * Get all visible (non-hidden) containers
     */
    getVisibleContainers(): any[];
    /**
     * Get all expanded (non-collapsed) containers
     */
    getExpandedContainers(): any[];
    /**
     * Get all hyper edges
     */
    getHyperEdges(): any[];
    /**
     * Get container children for a container id
     */
    getContainerChildren(containerId: any): Set<string>;
    /**
     * Get the container that contains a given node
     */
    getNodeContainer(nodeId: any): string;
    /**
     * Clear all data
     */
    clear(): void;
    /**
     * Collapse a container (depth-first, bottom-up with edge lifting)
     */
    collapseContainer(containerId: any): void;
    /**
     * Expand a container (depth-first, top-down with edge grounding)
     * SYMMETRIC INVERSE of collapseContainer()
     */
    expandContainer(containerId: any): void;
    /**
     * Perform the actual collapse operation for a single container
     * This includes lifting edges and hyperEdges from child containers
     */
    _performCollapseWithLift(containerId: any): void;
    /**
     * Create collapsed container representation
     */
    _createCollapsedContainerRepresentation(containerId: any, container: any): void;
    /**
     * Mark container as collapsed and update tracking
     */
    _markContainerAsCollapsed(containerId: any, container: any): void;
    /**
     * Hide child nodes and reroute existing hyperEdges
     */
    _hideChildNodesAndRerouteEdges(containerId: any, containerNodes: any): void;
    /**
     * Perform the actual expansion operation for a single container
     * This includes grounding edges and hyperEdges to child containers
     * SYMMETRIC INVERSE of _performCollapseWithLift()
     */
    _performExpandWithGround(containerId: any): void;
    /**
     * Mark container as expanded and remove collapsed representation
     */
    _markContainerAsExpandedAndCleanup(containerId: any, container: any): void;
    /**
     * Show all direct child nodes
     */
    _showChildNodes(containerId: any): void;
    /**
     * Reroute existing hyperEdges that point to nodes we're about to hide
     * when collapsing a container
     */
    _rerouteHyperEdgesToCollapsedContainer(containerId: any, containerNodes: any): void;
    /**
     * Calculate if a hyperEdge needs rerouting and return the new endpoints
     */
    _calculateHyperEdgeReroute(hyperEdge: any, containerNodes: any, containerId: any): {
        newSource: any;
        newTarget: any;
    };
    /**
     * Lift edges and hyperEdges from nodes and child containers to the parent container
     */
    _liftEdgesToContainer(containerId: any, containerNodes: any, childContainers: any): void;
    /**
     * Ground hyperEdges and edges connected to the expanding container
     * This is the inverse of lifting: restore connections to the correct child endpoints
     * SYMMETRIC INVERSE of _liftEdgesToContainer()
     */
    _groundEdgesFromContainer(containerId: any): void;
    /**
     * Lift edges from direct child nodes
     */
    _liftNodeEdges(containerId: any, containerNodes: any, liftedConnections: any): void;
    /**
     * Process a single node edge during lifting
     */
    _processNodeEdge(edge: any, containerNodes: any, liftedConnections: any): void;
    /**
     * Ground edges from direct child nodes
     * SYMMETRIC INVERSE of _liftNodeEdges()
     */
    _groundNodeEdges(containerId: any, children: any): void;
    /**
     * Lift hyperEdges from child containers to this container level
     */
    _liftChildContainerHyperEdges(containerId: any, childContainers: any, liftedConnections: any): void;
    /**
     * Lift a single child container hyperEdge
     */
    _liftChildContainerHyperEdge(hyperEdge: any, childContainers: any, liftedConnections: any): void;
    /**
     * Ground hyperEdges connected to the expanding container
     * SYMMETRIC INVERSE of _liftChildContainerHyperEdges()
     */
    _groundContainerHyperEdges(containerId: any): void;
    /**
     * Ground a single container hyperEdge
     */
    _groundSingleContainerHyperEdge(hyperEdge: any, containerId: any): void;
    /**
     * Helper to add an edge to lifted connections with proper direction
     */
    _addToLiftedConnections(liftedConnections: any, externalId: any, edge: any, isOutgoing: any, internalEndpoint: any): void;
    /**
     * Ground a single connection during container expansion
     * SYMMETRIC INVERSE of _addToLiftedConnections()
     */
    _groundConnection(externalId: any, internalEndpoint: any, hyperEdge: any, isSourceContainer: any): void;
    /**
     * Create hyperEdges from lifted connections
     */
    _createHyperEdgesFromLiftedConnections(containerId: any, liftedConnections: any): void;
    /**
     * Create hyperEdges for both directions (incoming and outgoing)
     */
    _createDirectionalHyperEdges(containerId: any, externalId: any, connections: any): void;
    /**
     * Create a single hyperEdge with proper metadata
     */
    _createHyperEdge(sourceId: any, targetId: any, edgesArray: any): void;
    /**
     * Find the original internal endpoint that should receive grounded connections
     * For multiple edges, prefer containers over nodes, then use the first one
     */
    _findOriginalInternalEndpoint(edges: any, containerId: any): unknown;
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
export declare function createVisualizationState(): VisualizationState;
//# sourceMappingURL=VisState.d.ts.map