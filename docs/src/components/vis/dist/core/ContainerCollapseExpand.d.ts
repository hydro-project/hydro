/**
 * Container Collapse/Expand Engine
 *
 * Handles sophisticated container state transitions with symmetric edge lifting/grounding operations.
 * Ensures proper tree hierarchy validation and optimized edge processing.
 */
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
export declare class ContainerCollapseExpandEngine {
    private readonly state;
    private readonly containerToEdges;
    constructor(visualizationState: any);
    /**
     * Collapse a container (depth-first, bottom-up with edge lifting)
     * Validates tree hierarchy and processes edges efficiently
     */
    collapseContainer(containerId: string): void;
    /**
     * Expand a container (depth-first, top-down with edge grounding)
     * SYMMETRIC INVERSE of collapseContainer()
     */
    expandContainer(containerId: string): void;
    /**
     * Validate tree hierarchy when adding container child
     * Prevents cycles and enforces single-parent constraint
     */
    validateTreeHierarchy(parentId: string, childId: string): void;
    /**
     * Rebuild the container-to-edges index for optimized lookups
     */
    rebuildEdgeIndex(): void;
    /**
     * Perform the actual collapse operation for a single container
     * This includes lifting edges and hyperEdges from child containers
     */
    private _performCollapseWithLift;
    /**
     * Perform the actual expansion operation for a single container
     * SYMMETRIC INVERSE of _performCollapseWithLift()
     */
    private _performExpandWithGround;
    private _validateContainerForCollapse;
    private _validateContainerForExpansion;
    /**
     * Check if adding childId to parentId would create a cycle
     * Uses DFS to detect cycles in the container hierarchy
     */
    private _wouldCreateCycle;
    /**
     * Build optimized index of container -> edges for efficient lookups
     */
    private _buildContainerEdgeIndex;
    /**
     * Add an edge to the container index for all relevant containers
     */
    private _indexEdgeForContainers;
    /**
     * Add edge to container's edge set in the index
     */
    private _addEdgeToContainerIndex;
    /**
     * Update edge index when container hierarchy changes
     */
    private _updateContainerEdgeIndex;
    /**
     * Get edges efficiently using the optimized index
     */
    private _getContainerEdges;
    private _createCollapsedContainerRepresentation;
    private _markContainerAsCollapsed;
    private _markContainerAsExpandedAndCleanup;
    private _showChildNodes;
    private _hideChildNodesAndRerouteEdges;
    private _liftEdgesToContainer;
    /**
     * Optimized edge lifting using container edge index
     */
    private _liftNodeEdgesOptimized;
    private _groundEdgesFromContainer;
    private _processNodeEdge;
    private _groundNodeEdges;
    private _liftChildContainerHyperEdges;
    private _liftChildContainerHyperEdge;
    private _groundContainerHyperEdges;
    private _groundSingleContainerHyperEdge;
    private _rerouteHyperEdgesToCollapsedContainer;
    private _calculateHyperEdgeReroute;
    private _categorizeChildren;
    private _setNodesVisibility;
    private _isEndpointConnectable;
    private _processHyperEdges;
    private _addToLiftedConnections;
    private _groundConnection;
    private _createHyperEdgesFromLiftedConnections;
    private _createDirectionalHyperEdges;
    private _createHyperEdge;
    private _findOriginalInternalEndpoint;
    private _aggregateEdgeStyles;
}
//# sourceMappingURL=ContainerCollapseExpand.d.ts.map