export class ReactFlowBridge {
    /**
     * Convert positioned VisState data to ReactFlow format
     * Pure data transformation - no layout logic
     */
    visStateToReactFlow(visState: any): {
        nodes: any[];
        edges: any[];
    };
    /**
     * Build parent-child relationship map
     */
    buildParentMap(visState: any): Map<any, any>;
    /**
     * Convert containers to ReactFlow container nodes
     */
    convertContainers(visState: any, nodes: any, parentMap: any): void;
    /**
     * Convert regular nodes to ReactFlow standard nodes
     */
    convertNodes(visState: any, nodes: any, parentMap: any): void;
    /**
     * Convert regular edges to ReactFlow edges
     */
    convertEdges(visState: any, edges: any): void;
    /**
     * Convert hyperedges to ReactFlow edges
     */
    convertHyperEdges(visState: any, edges: any): void;
    /**
     * Extract custom properties from graph elements
     */
    extractCustomProperties(element: any): {};
}
//# sourceMappingURL=ReactFlowBridge.d.ts.map