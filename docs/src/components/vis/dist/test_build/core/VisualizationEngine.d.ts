/**
 * Factory function to create a visualization engine
 */
export function createVisualizationEngine(visState: any, config: any): VisualizationEngine;
export class VisualizationEngine {
    constructor(visState: any, config?: {});
    listeners: Map<any, any>;
    visState: any;
    elkBridge: ELKBridge;
    reactFlowBridge: ReactFlowBridge;
    config: {
        autoLayout: boolean;
        layoutDebounceMs: number;
        enableLogging: boolean;
    };
    state: {
        phase: string;
        lastUpdate: number;
        layoutCount: number;
    };
    /**
     * Get current engine state
     */
    getState(): {
        phase: string;
        lastUpdate: number;
        layoutCount: number;
    };
    /**
     * Get the underlying VisState
     */
    getVisState(): any;
    /**
     * Run layout on current VisState data
     */
    runLayout(): Promise<void>;
    /**
     * Get ReactFlow data for rendering
     */
    getReactFlowData(): {
        nodes: any[];
        edges: any[];
    };
    /**
     * Complete visualization pipeline: layout + render
     */
    visualize(): Promise<{
        nodes: any[];
        edges: any[];
    }>;
    /**
     * Trigger layout with debouncing (for auto-layout)
     */
    scheduleLayout(): void;
    layoutTimeout: NodeJS.Timeout;
    /**
     * Notify that VisState data has changed
     */
    onDataChanged(): void;
    /**
     * Add state change listener
     */
    onStateChange(id: any, listener: any): void;
    /**
     * Remove state change listener
     */
    removeStateListener(id: any): void;
    /**
     * Clean up resources
     */
    dispose(): void;
    updateState(phase: any): void;
    handleError(message: any, error: any): void;
    log(message: any): void;
}
import { ELKBridge } from '../bridges/ELKBridge';
import { ReactFlowBridge } from '../bridges/ReactFlowBridge';
//# sourceMappingURL=VisualizationEngine.d.ts.map