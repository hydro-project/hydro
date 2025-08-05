/**
 * Create a new visualization state instance
 */
export function createVisualizationState(): VisualizationState;
/**
 * Core visualization state class that manages all graph elements
 */
export class VisualizationState {
    graphNodes: Map<any, any>;
    graphEdges: Map<any, any>;
    containers: Map<any, any>;
    hyperEdges: Map<any, any>;
    visibleNodes: Map<any, any>;
    visibleEdges: Map<any, any>;
    visibleContainers: Map<any, any>;
    expandedContainers: Map<any, any>;
    collapsedContainers: Map<any, any>;
    containerChildren: Map<any, any>;
    nodeContainers: Map<any, any>;
    nodeToEdges: Map<any, any>;
    /**
     * Validate that an entity exists and optionally check a condition
     */
    _validateEntity(entity: any, conditionFn?: any): any;
    /**
     * Generic method to get an entity from any collection
     */
    _getEntity(entityType: any, id: any): any;
    /**
     * Generic method to set hidden flag for any entity type that supports it
     */
    _setEntityHidden(entityType: any, id: any, hidden: any): void;
    /**
     * Generic method to get hidden flag for any entity type that supports it
     */
    _getEntityHidden(entityType: any, id: any): any;
    /**
     * Get the main collection for an entity type
     */
    _getEntityCollection(entityType: any): Map<any, any>;
    /**
     * Update visibility collections based on entity type and hidden state
     */
    _updateVisibilityCollection(entityType: any, id: any, entity: any): void;
    /**
     * Add or update a graph node
     */
    setGraphNode(id: any, { label, style, hidden, ...otherProps }: {
        [x: string]: any;
        label: any;
        style?: any;
        hidden?: boolean;
    }): {
        id: any;
        label: any;
        style: any;
        hidden: boolean;
    };
    /**
     * Get a graph node by id
     */
    getGraphNode(id: any): any;
    /**
     * Set hidden flag for a graph node
     */
    setNodeHidden(id: any, hidden: any): void;
    /**
     * Get hidden flag for a graph node
     */
    getNodeHidden(id: any): any;
    /**
     * Remove a graph node
     */
    removeGraphNode(id: any): void;
    /**
     * Add or update a graph edge
     */
    setGraphEdge(id: any, { source, target, style, hidden, ...otherProps }: {
        [x: string]: any;
        source: any;
        target: any;
        style?: any;
        hidden?: boolean;
    }): {
        id: any;
        source: any;
        target: any;
        style: any;
        hidden: boolean;
    };
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
        id: any;
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
     */
    getContainer(id: any): any;
    /**
     * Set collapsed flag for a container
     */
    setContainerCollapsed(id: any, collapsed: any): void;
    /**
     * Get collapsed flag for a container
     */
    getContainerCollapsed(id: any): any;
    /**
     * Set hidden flag for a container
     */
    setContainerHidden(id: any, hidden: any): void;
    /**
     * Get hidden flag for a container
     */
    getContainerHidden(id: any): any;
    /**
     * Add a child to a container
     */
    addContainerChild(containerId: any, childId: any): void;
    /**
     * Remove a child from a container
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
        style?: any;
    }): {
        id: any;
        source: any;
        target: any;
        style: any;
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
    getContainerChildren(containerId: any): any;
    /**
     * Get the container that contains a given node
     */
    getNodeContainer(nodeId: any): any;
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
    _findOriginalInternalEndpoint(edges: any, containerId: any): any;
    /**
     * Check if an endpoint (node or container) is visible and should be connected to
     */
    _isEndpointConnectable(endpointId: any): boolean;
    /**
     * Categorize children into nodes and containers
     */
    _categorizeChildren(children: any): {
        containerNodes: Set<any>;
        childContainers: Set<any>;
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
    _aggregateEdgeStyles(edges: any): any;
    /**
     * Check all internal hyperedge invariants
     * This is called after operations that affect containers or edges
     * @private
     */
    private _checkHyperEdgeInvariants;
    /**
     * Invariant: HyperEdges exist only for visible, collapsed containers
     * @private
     */
    private _checkHyperEdgeExistence;
    /**
     * Invariant: HyperEdges connect to valid visible endpoints (collapsed containers or visible nodes)
     * @private
     */
    private _checkHyperEdgeEndpoints;
    /**
     * Invariant: HyperEdges are completely encapsulated and never appear in visibleEdges
     * @private
     */
    private _checkHyperEdgeVisibilityEncapsulation;
}
//# sourceMappingURL=VisState.d.ts.map