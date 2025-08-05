/**
 * @fileoverview New Bridge-Based Layout Engine
 *
 * Complete replacement for alpha ELKLayoutEngine using our bridge architecture.
 * Maintains identical API while using the new VisualizationEngine internally.
 */
import type { GraphNode, GraphEdge, ExternalContainer } from '../shared/types';
import type { LayoutResult, LayoutEngine, LayoutConfig, LayoutEventCallback, LayoutStatistics } from '../core/types';
export declare class ELKLayoutEngine implements LayoutEngine {
    private config;
    private callbacks;
    private lastStatistics;
    constructor(config?: LayoutConfig);
    /**
     * Run layout - SAME API as alpha
     */
    layout(nodes: GraphNode[], edges: GraphEdge[], containers: ExternalContainer[], config?: LayoutConfig): Promise<LayoutResult>;
    /**
     * Layout with changed container - compatibility method
     */
    layoutWithChangedContainer(nodes: GraphNode[], edges: GraphEdge[], containers: ExternalContainer[], config?: LayoutConfig, changedContainerId?: string | null, visualizationState?: any): Promise<LayoutResult>;
    /**
     * Convert nodes to positioned format
     */
    private convertNodes;
    /**
     * Convert edges to positioned format
     */
    private convertEdges;
    /**
     * Convert containers to positioned format
     */
    private convertContainers;
    /**
     * Emit event to listeners
     */
    private emit;
    /**
     * Get last layout statistics
     */
    getLastLayoutStatistics(): LayoutStatistics | null;
    /**
     * Add event listener
     */
    on(event: string, callback: LayoutEventCallback): void;
    /**
     * Remove event listener
     */
    off(event: string, callback: LayoutEventCallback): void;
}
/**
 * Default layout configuration - MRTree as default for better hierarchical display
 */
export declare const DEFAULT_LAYOUT_CONFIG: LayoutConfig;
/**
 * Create ELK state manager - compatibility wrapper
 */
export declare function createELKStateManager(): {
    updatePositions: () => void;
    dispose: () => void;
};
//# sourceMappingURL=ELKLayoutEngine.d.ts.map