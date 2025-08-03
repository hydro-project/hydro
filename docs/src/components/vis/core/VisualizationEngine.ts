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
import { ELKBridge } from '../bridges/ELKBridge';
import { ReactFlowBridge } from '../bridges/ReactFlowBridge';
import type { ReactFlowData } from '../bridges/ReactFlowBridge';

// Visualization states
export type VisualizationPhase = 
  | 'initial'       // Fresh data loaded
  | 'laying_out'    // ELK layout in progress
  | 'ready'         // Layout complete, ready to render
  | 'rendering'     // ReactFlow conversion in progress
  | 'displayed'     // ReactFlow data ready for display
  | 'error';        // Error occurred

export interface VisualizationEngineState {
  phase: VisualizationPhase;
  lastUpdate: number;
  layoutCount: number;
  error?: string;
}

export interface VisualizationEngineConfig {
  autoLayout: boolean;          // Automatically run layout on data changes
  layoutDebounceMs: number;     // Debounce layout calls
  enableLogging: boolean;       // Enable detailed logging
}

const DEFAULT_CONFIG: VisualizationEngineConfig = {
  autoLayout: true,
  layoutDebounceMs: 300,
  enableLogging: true
};

export class VisualizationEngine {
  private visState: VisualizationState;
  private elkBridge: ELKBridge;
  private reactFlowBridge: ReactFlowBridge;
  private config: VisualizationEngineConfig;
  private state: VisualizationEngineState;
  private layoutTimeout?: NodeJS.Timeout;
  private listeners: Map<string, (state: VisualizationEngineState) => void> = new Map();

  constructor(
    visState: VisualizationState, 
    config: Partial<VisualizationEngineConfig> = {}
  ) {
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
  getState(): VisualizationEngineState {
    return { ...this.state };
  }

  /**
   * Get the underlying VisState
   */
  getVisState(): VisualizationState {
    return this.visState;
  }

  /**
   * Run layout on current VisState data
   */
  async runLayout(): Promise<void> {
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
      
    } catch (error) {
      this.handleError('Layout failed', error);
    }
  }

  /**
   * Get ReactFlow data for rendering
   */
  getReactFlowData(): ReactFlowData {
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
      
    } catch (error) {
      this.handleError('ReactFlow conversion failed', error);
      throw error;
    }
  }

  /**
   * Complete visualization pipeline: layout + render
   */
  async visualize(): Promise<ReactFlowData> {
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
  scheduleLayout(): void {
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
  onDataChanged(): void {
    this.log('📝 VisState data changed');
    this.updateState('initial');
    this.scheduleLayout();
  }

  /**
   * Add state change listener
   */
  onStateChange(id: string, listener: (state: VisualizationEngineState) => void): void {
    this.listeners.set(id, listener);
  }

  /**
   * Remove state change listener
   */
  removeStateListener(id: string): void {
    this.listeners.delete(id);
  }

  /**
   * Clean up resources
   */
  dispose(): void {
    if (this.layoutTimeout) {
      clearTimeout(this.layoutTimeout);
    }
    this.listeners.clear();
    this.log('🧹 VisualizationEngine disposed');
  }

  // ============ Internal Methods ============

  private updateState(phase: VisualizationPhase): void {
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
      } catch (error) {
        console.error('[VisualizationEngine] Listener error:', error);
      }
    });
  }

  private handleError(message: string, error: any): void {
    const errorMessage = `${message}: ${error instanceof Error ? error.message : String(error)}`;
    this.state.error = errorMessage;
    this.updateState('error');
    
    console.error(`[VisualizationEngine] ❌ ${errorMessage}`, error);
  }

  private log(message: string): void {
    if (this.config.enableLogging) {
      console.log(`[VisualizationEngine] ${message}`);
    }
  }
}

/**
 * Factory function to create a visualization engine
 */
export function createVisualizationEngine(
  visState: VisualizationState,
  config?: Partial<VisualizationEngineConfig>
): VisualizationEngine {
  return new VisualizationEngine(visState, config);
}
