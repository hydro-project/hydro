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
import { ELKBridge } from '../bridges/ELKBridge';
import { ReactFlowBridge } from '../bridges/ReactFlowBridge';
const DEFAULT_CONFIG = {
    autoLayout: true,
    layoutDebounceMs: 300,
    enableLogging: true
};
export class VisualizationEngine {
    constructor(visState, config = {}) {
        this.listeners = new Map();
        this.visState = visState;
        this.elkBridge = new ELKBridge();
        this.reactFlowBridge = new ReactFlowBridge();
        this.config = { ...DEFAULT_CONFIG, ...config };
        this.state = {
            phase: 'initial',
            lastUpdate: Date.now(),
            layoutCount: 0
        };
        this.log('🚀 VisualizationEngine initialized');
    }
    // ============ Public API ============
    /**
     * Get current engine state
     */
    getState() {
        return { ...this.state };
    }
    /**
     * Get the underlying VisState
     */
    getVisState() {
        return this.visState;
    }
    /**
     * Run layout on current VisState data
     */
    async runLayout() {
        this.log('📊 Layout requested');
        if (this.state.phase === 'laying_out') {
            this.log('⚠️ Layout already in progress, skipping');
            return;
        }
        try {
            this.updateState('laying_out');
            // Use ELK bridge to layout the VisState
            await this.elkBridge.layoutVisState(this.visState);
            this.state.layoutCount++;
            this.updateState('ready');
            this.log(`✅ Layout complete (${this.state.layoutCount} total layouts)`);
        }
        catch (error) {
            this.handleError('Layout failed', error);
        }
    }
    /**
     * Get ReactFlow data for rendering
     */
    getReactFlowData() {
        this.log('🔄 ReactFlow data requested');
        if (this.state.phase === 'error') {
            throw new Error(`Cannot render in error state: ${this.state.error}`);
        }
        try {
            this.updateState('rendering');
            // Use ReactFlow bridge to convert VisState
            const reactFlowData = this.reactFlowBridge.visStateToReactFlow(this.visState);
            this.updateState('displayed');
            this.log(`✅ ReactFlow data generated: ${reactFlowData.nodes.length} nodes, ${reactFlowData.edges.length} edges`);
            return reactFlowData;
        }
        catch (error) {
            this.handleError('ReactFlow conversion failed', error);
            throw error;
        }
    }
    /**
     * Complete visualization pipeline: layout + render
     */
    async visualize() {
        this.log('🎨 Full visualization pipeline requested');
        // Step 1: Run layout if needed
        if (this.state.phase !== 'ready' && this.state.phase !== 'displayed') {
            await this.runLayout();
        }
        // Step 2: Generate ReactFlow data
        return this.getReactFlowData();
    }
    /**
     * Trigger layout with debouncing (for auto-layout)
     */
    scheduleLayout() {
        if (!this.config.autoLayout) {
            return;
        }
        this.log('⏱️ Layout scheduled with debouncing');
        // Clear existing timeout
        if (this.layoutTimeout) {
            clearTimeout(this.layoutTimeout);
        }
        // Schedule new layout
        this.layoutTimeout = setTimeout(() => {
            this.runLayout().catch(error => {
                this.handleError('Scheduled layout failed', error);
            });
        }, this.config.layoutDebounceMs);
    }
    /**
     * Notify that VisState data has changed
     */
    onDataChanged() {
        this.log('📝 VisState data changed');
        this.updateState('initial');
        this.scheduleLayout();
    }
    /**
     * Add state change listener
     */
    onStateChange(id, listener) {
        this.listeners.set(id, listener);
    }
    /**
     * Remove state change listener
     */
    removeStateListener(id) {
        this.listeners.delete(id);
    }
    /**
     * Clean up resources
     */
    dispose() {
        if (this.layoutTimeout) {
            clearTimeout(this.layoutTimeout);
        }
        this.listeners.clear();
        this.log('🧹 VisualizationEngine disposed');
    }
    // ============ Internal Methods ============
    updateState(phase) {
        const previousPhase = this.state.phase;
        this.state.phase = phase;
        this.state.lastUpdate = Date.now();
        if (phase !== 'error') {
            delete this.state.error;
        }
        this.log(`🔄 State: ${previousPhase} → ${phase}`);
        // Notify listeners
        this.listeners.forEach(listener => {
            try {
                listener({ ...this.state });
            }
            catch (error) {
                console.error('[VisualizationEngine] Listener error:', error);
            }
        });
    }
    handleError(message, error) {
        const errorMessage = `${message}: ${error instanceof Error ? error.message : String(error)}`;
        this.state.error = errorMessage;
        this.updateState('error');
        console.error(`[VisualizationEngine] ❌ ${errorMessage}`, error);
    }
    log(message) {
        if (this.config.enableLogging) {
            console.log(`[VisualizationEngine] ${message}`);
        }
    }
}
/**
 * Factory function to create a visualization engine
 */
export function createVisualizationEngine(visState, config) {
    return new VisualizationEngine(visState, config);
}
//# sourceMappingURL=VisualizationEngine.js.map