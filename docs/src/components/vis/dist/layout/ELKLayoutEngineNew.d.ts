/**
 * @fileoverview ELK Layout Engine (Enhanced with working patterns)
 *
 * ELK-based automatic layout engine using proven patterns from the working visualizer.
 * Handles hierarchical layouts with proper container dimension management.
 */
import { LayoutEngine, LayoutResult, LayoutConfig } from './types';
import { GraphNode, GraphEdge, Container, HyperEdge } from '../shared/types';
export declare class ELKLayoutEngine implements LayoutEngine {
    private elkStateManager;
    private dimensionsCache;
    constructor();
    layout(nodes: GraphNode[], edges: GraphEdge[], containers: Container[], hyperEdges: HyperEdge[], config?: LayoutConfig): Promise<LayoutResult>;
    /**
     * Get cached container dimensions
     */
    getCachedDimensions(containerId: string): {
        width: number;
        height: number;
    } | undefined;
    /**
     * Clear the dimensions cache
     */
    clearCache(): void;
}
//# sourceMappingURL=ELKLayoutEngineNew.d.ts.map