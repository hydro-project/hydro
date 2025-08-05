/**
 * Container collapse/expand operations with tree hierarchy validation and optimized edge processing.
 *
 * Features:
 * - Tree hierarchy enforcement (no cycles/DAGs)
 * - Symmetric collapse ↔️ expand operations
 * - Edge lifting/grounding with proper metadata preservation
 * - Optimized edge lookup with indexing
 * - Sequential operation guarantee (no concurrency)
 *
 * @class ContainerCollapseExpandEngine
 */
export class ContainerCollapseExpandEngine {
    constructor(visualizationState: any);
    state: any;
    containerToEdges: Map<any, any>;
    /**
     * Collapse a container (depth-first, bottom-up with edge lifting)
     * Validates tree hierarchy and processes edges efficiently
     */
    collapseContainer(containerId: any): void;
    /**
     * Expand a container (depth-first, top-down with edge grounding)
     * SYMMETRIC INVERSE of collapseContainer()
     */
    expandContainer(containerId: any): void;
    /**
     * Validate tree hierarchy when adding container child
     * Prevents cycles and enforces single-parent constraint
     */
    validateTreeHierarchy(parentId: any, childId: any): void;
    /**
     * Rebuild the container-to-edges index for optimized lookups
     */
    rebuildEdgeIndex(): void;
    /**
     * Perform the actual collapse operation for a single container
     * This includes lifting edges and hyperEdges from child containers
     */
    _performCollapseWithLift(containerId: any): void;
    /**
     * Perform the actual expansion operation for a single container
     * SYMMETRIC INVERSE of _performCollapseWithLift()
     */
    _performExpandWithGround(containerId: any): void;
    _validateContainerForCollapse(containerId: any, container: any): void;
    _validateContainerForExpansion(containerId: any, container: any): void;
    /**
     * Check if adding childId to parentId would create a cycle
     * Uses DFS to detect cycles in the container hierarchy
     */
    _wouldCreateCycle(parentId: any, childId: any): boolean;
    /**
     * Build optimized index of container -> edges for efficient lookups
     */
    _buildContainerEdgeIndex(): void;
    /**
     * Add an edge to the container index for all relevant containers
     */
    _indexEdgeForContainers(edgeId: any, edge: any): void;
    /**
     * Add edge to container's edge set in the index
     */
    _addEdgeToContainerIndex(containerId: any, edgeId: any): void;
    /**
     * Update edge index when container hierarchy changes
     */
    _updateContainerEdgeIndex(containerId: any): void;
    /**
     * Get edges efficiently using the optimized index
     */
    _getContainerEdges(containerId: any): any;
    _createCollapsedContainerRepresentation(containerId: any, container: any): void;
    _markContainerAsCollapsed(containerId: any, container: any): void;
    _markContainerAsExpandedAndCleanup(containerId: any, container: any): void;
    _showChildNodes(containerId: any): void;
    _hideChildNodesAndRerouteEdges(containerId: any, containerNodes: any): void;
    _liftEdgesToContainer(containerId: any, containerNodes: any, childContainers: any): void;
    /**
     * Optimized edge lifting using container edge index
     */
    _liftNodeEdgesOptimized(containerId: any, containerNodes: any, liftedConnections: any): void;
    _groundEdgesFromContainer(containerId: any): void;
    _processNodeEdge(edge: any, containerNodes: any, liftedConnections: any): void;
    _groundNodeEdges(containerId: any, children: any): void;
    _liftChildContainerHyperEdges(containerId: any, childContainers: any, liftedConnections: any): void;
    _liftChildContainerHyperEdge(hyperEdge: any, childContainers: any, liftedConnections: any): void;
    _groundContainerHyperEdges(containerId: any): void;
    _groundSingleContainerHyperEdge(hyperEdge: any, containerId: any): void;
    _rerouteHyperEdgesToCollapsedContainer(containerId: any, containerNodes: any): void;
    _calculateHyperEdgeReroute(hyperEdge: any, containerNodes: any, containerId: any): {
        newSource: any;
        newTarget: any;
    };
    _categorizeChildren(children: any): {
        containerNodes: Set<any>;
        childContainers: Set<any>;
    };
    _setNodesVisibility(nodeIds: any, hidden: any): void;
    _isEndpointConnectable(endpointId: any): boolean;
    _processHyperEdges(predicate: any, updateFn: any): void;
    _addToLiftedConnections(liftedConnections: any, externalId: any, edge: any, isOutgoing: any, internalEndpoint: any): void;
    _groundConnection(externalId: any, internalEndpoint: any, hyperEdge: any, isSourceContainer: any): void;
    _createHyperEdgesFromLiftedConnections(containerId: any, liftedConnections: any): void;
    _createDirectionalHyperEdges(containerId: any, externalId: any, connections: any): void;
    _createHyperEdge(sourceId: any, targetId: any, edgesArray: any): void;
    _findOriginalInternalEndpoint(edges: any, containerId: any): any;
    _aggregateEdgeStyles(edges: any): string;
}
//# sourceMappingURL=ContainerCollapseExpand.d.ts.map