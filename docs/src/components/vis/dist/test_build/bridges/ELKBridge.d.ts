export class ELKBridge {
    elk: import("elkjs").ELK;
    /**
     * Convert VisState to ELK format and run layout
     * Key insight: Include ALL visible edges (regular + hyper) with no distinction
     */
    layoutVisState(visState: any): Promise<void>;
    /**
     * Validate ELK input data to prevent null reference errors
     */
    validateELKInput(elkGraph: any): void;
    /**
     * Convert VisState to ELK format
     */
    visStateToELK(visState: any): {
        id: string;
        children: any[];
        edges: any;
        layoutOptions: {
            'elk.algorithm': string;
            'elk.direction': string;
            'elk.spacing.nodeNode': string;
            'elk.spacing.edgeNode': string;
        };
    };
    /**
     * Extract visible nodes (both GraphNodes and collapsed containers as nodes)
     */
    extractVisibleNodes(visState: any): any[];
    /**
     * Extract visible containers (only expanded ones that need hierarchical layout)
     */
    extractVisibleContainers(visState: any): any[];
    /**
     * Extract ALL edges - both regular edges and hyperedges with no distinction
     * This is the critical fix: hyperedges were getting lost in the old implementation
     */
    extractAllEdges(visState: any): any[];
    /**
     * Build ELK graph from extracted data
     */
    buildELKGraph(nodes: any, containers: any, edges: any): {
        id: string;
        children: any[];
        edges: any;
        layoutOptions: {
            'elk.algorithm': string;
            'elk.direction': string;
            'elk.spacing.nodeNode': string;
            'elk.spacing.edgeNode': string;
        };
    };
    /**
     * Apply ELK results back to VisState
     */
    elkToVisState(elkResult: any, visState: any): void;
    /**
     * Update container dimensions and child positions from ELK result
     */
    updateContainerFromELK(elkNode: any, visState: any): void;
    /**
     * Update node position from ELK result
     */
    updateNodeFromELK(elkNode: any, visState: any): void;
    isNodeInContainer(nodeId: any, containerId: any, container: any): any;
    isNodeInAnyContainer(nodeId: any, containers: any): any;
}
//# sourceMappingURL=ELKBridge.d.ts.map