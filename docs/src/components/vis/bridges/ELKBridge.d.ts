/**
 * @fileoverview ELK Bridge - Clean interface between VisState and ELK
 *
 * This bridge implements the core architectural principle:
 * - VisState contains ALL data (nodes, edges, containers)
 * - ELK gets visible elements only through visibleEdges (hyperedges included transparently)
 * - ELK returns layout positions that get applied back to VisState
 */
import { VisualizationState } from '../core/VisState';
import type { LayoutConfig } from '../core/types';
export declare class ELKBridge {
    private elk;
    private layoutConfig;
    constructor(layoutConfig?: LayoutConfig);
    /**
     * Update layout configuration
     */
    updateLayoutConfig(config: LayoutConfig): void;
    /**
     * Convert VisState to ELK format and run layout
     * Key insight: Include ALL visible edges (regular + hyper) with no distinction
     */
    layoutVisState(visState: VisualizationState): Promise<void>;
    /**
     * Validate ELK input data to prevent null reference errors
     */
    private validateELKInput;
    /**
     * Convert VisState to ELK format
     */
    private visStateToELK;
    /**
     * Extract visible nodes (both GraphNodes and collapsed containers as nodes)
     */
    private extractVisibleNodes;
    /**
     * Extract visible containers (only expanded ones that need hierarchical layout)
     */
    private extractVisibleContainers;
    /**
     * Build ELK graph from extracted data
     */
    private buildELKGraph;
    /**
     * Apply ELK results back to VisState
     */
    private elkToVisState;
    /**
     * Update edge routing information from ELK result
     */
    private updateEdgeFromELK;
    /**
     * Update container dimensions and child positions from ELK result
     */
    private updateContainerFromELK;
    /**
     * Update node position from ELK result
     */
    private updateNodeFromELK;
    private isNodeInContainer;
    private isNodeInAnyContainer;
}
//# sourceMappingURL=ELKBridge.d.ts.map