/**
 * Container Collapse/Expand Engine
 *
 * Clean implementation using hierarchical lineage tracking approach.
 * Each hyperedge stores the original leaf endpoints and we use dynamic
 * ancestor lookup to find the current visible state of any node.
 */
export declare class ContainerCollapseExpandEngine {
    private readonly state;
    constructor(visualizationState: any);
    /**
     * Find the current visible ancestor of a node by walking up the hierarchy
     */
    private _findCurrentVisibleAncestor;
    /**
     * Collapse a container: hide children and lift their edges to container level
     */
    collapseContainer(containerId: string): void;
    /**
     * Expand a container: show children and ground hyperedges back to edges
     */
    expandContainer(containerId: string): void;
    private _collapseChildContainers;
    private _expandChildContainers;
    private _hideContainerChildren;
    private _showContainerChildren;
    private _hideCrossingEdges;
    /**
     * Find all edges that cross the container boundary
     */
    private _findCrossingEdges;
    /**
     * Group crossing edges by their current external endpoint (using leaf mapping)
     */
    private _groupEdgesByCurrentExternalEndpoint;
    private _createHyperedgesFromGroups;
    private _createHyperedge;
    private _isContainerEndpoint;
    private _findContainerHyperedges;
    /**
     * Ground a hyperedge: convert it back to edges using current visible ancestors
     */
    private _groundHyperedgeWithLeafMapping;
    private _processOriginalEndpoint;
    private _areBothVisibleLeafNodes;
    private _shouldCreateIntermediateHyperedge;
    private _createIntermediateHyperedge;
    private _aggregateStyles;
    validateTreeHierarchy(parentId: string, childId: string): void;
    private _wouldCreateCycle;
    rebuildEdgeIndex(): void;
}
//# sourceMappingURL=ContainerCollapseExpand.d.ts.map