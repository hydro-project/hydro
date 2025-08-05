/**
 * @fileoverview Visualization Engine - Orchestrates the entire visualization pipeline
 *
 * This engine manages the state machine for visualization:
 * 1. Data Input → VisState
 * 2. Layout (VisState → ELK → VisState)
 * 3. Render (VisState → ReactFlow)
 *
 * Clean separation: Engine orchestrates, Bridges translate, VisState stores
 */
import type { VisualizationState } from './VisState';
import type { ReactFlowData } from '../bridges/ReactFlowBridge';
import type { LayoutConfig } from './types';
export type VisualizationPhase = 'initial' | 'laying_out' | 'ready' | 'rendering' | 'displayed' | 'error';
export interface VisualizationEngineState {
    phase: VisualizationPhase;
    lastUpdate: number;
    layoutCount: number;
    error?: string;
}
export interface VisualizationEngineConfig {
    autoLayout: boolean;
    layoutDebounceMs: number;
    enableLogging: boolean;
    layoutConfig?: LayoutConfig;
}
export declare class VisualizationEngine {
    private visState;
    private elkBridge;
    private reactFlowBridge;
    private config;
    private state;
    private layoutTimeout?;
    private listeners;
    constructor(visState: VisualizationState, config?: Partial<VisualizationEngineConfig>);
    /**
     * Get current engine state
     */
    getState(): VisualizationEngineState;
    /**
     * Get the underlying VisState
     */
    getVisState(): VisualizationState;
    /**
     * Update layout configuration and optionally re-run layout
     */
    updateLayoutConfig(layoutConfig: LayoutConfig, autoReLayout?: boolean): void;
    /**
     * Run layout on current VisState data
     */
    runLayout(): Promise<void>;
    /**
     * Get ReactFlow data for rendering
     */
    getReactFlowData(): ReactFlowData;
    /**
     * Complete visualization pipeline: layout + render
     */
    visualize(): Promise<ReactFlowData>;
    /**
     * Trigger layout with debouncing (for auto-layout)
     */
    scheduleLayout(): void;
    /**
     * Notify that VisState data has changed
     */
    onDataChanged(): void;
    /**
     * Add state change listener
     */
    onStateChange(id: string, listener: (state: VisualizationEngineState) => void): void;
    /**
     * Remove state change listener
     */
    removeStateListener(id: string): void;
    /**
     * Clean up resources
     */
    dispose(): void;
    private updateState;
    private handleError;
    private log;
}
/**
 * Factory function to create a visualization engine
 */
export declare function createVisualizationEngine(visState: VisualizationState, config?: Partial<VisualizationEngineConfig>): VisualizationEngine;
//# sourceMappingURL=VisualizationEngine.d.ts.map