/**
 * @fileoverview Bridge-Based ReactFlow Converter
 *
 * Complete replacement for alpha ReactFlowConverter using our bridge architecture.
 * Maintains identical API while using the new ReactFlowBridge internally.
 */
import type { VisualizationState } from '../core/VisState';
import type { ReactFlowData } from '../bridges/ReactFlowBridge';
export declare class ReactFlowConverter {
    private bridge;
    constructor();
    /**
     * Convert VisualizationState to ReactFlow format - SAME API as alpha
     */
    convert(visState: VisualizationState): ReactFlowData;
    /**
     * Legacy method for compatibility
     */
    convertNodes(nodes: any[]): any[];
    /**
     * Legacy method for compatibility
     */
    convertEdges(edges: any[]): any[];
}
//# sourceMappingURL=ReactFlowConverter.d.ts.map