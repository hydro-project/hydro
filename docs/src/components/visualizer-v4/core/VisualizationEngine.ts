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
import type { LayoutConfig } from './types';

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
  layoutConfig?: LayoutConfig;  // Layout configuration
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
    this.elkBridge = new ELKBridge(config.layoutConfig);
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
   * Update layout configuration and optionally re-run layout
   */
  updateLayoutConfig(layoutConfig: LayoutConfig, autoReLayout: boolean = true): void {
    this.config.layoutConfig = { ...this.config.layoutConfig, ...layoutConfig };
    this.elkBridge.updateLayoutConfig(layoutConfig);
    
    this.log(`🔧 Layout config updated: ${JSON.stringify(layoutConfig)}`);
    
    if (autoReLayout) {
      this.runLayout();
    }
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
    
    // If smart collapse is enabled, apply it before scheduling layout
    if (this.config.layoutConfig?.enableSmartCollapse) {
      this.log('🧠 Smart collapse enabled, applying algorithm');
      // Use setTimeout to allow state update to propagate
      setTimeout(() => {
        this.applySmartContainerCollapse().catch(error => {
          this.handleError('Smart collapse failed', error);
        });
      }, 10);
    } else {
      this.scheduleLayout();
    }
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
   * Smart container collapse algorithm - optimizes screen real estate usage
   * by starting with all containers collapsed and iteratively expanding
   * based on area and zoom impact
   */
  async applySmartContainerCollapse(): Promise<void> {
    this.log('🧠 Applying smart container collapse algorithm');
    
    try {
      // Step 1: Get all containers and sort by area (smallest first)
      const containers = this.visState.visibleContainers;
      
      if (containers.length === 0) {
        this.log('ℹ️ No containers found, skipping smart collapse');
        return;
      }

      // Step 2: Start with all containers collapsed
      this.log(`📦 Collapsing ${containers.length} containers initially`);
      containers.forEach(container => {
        this.visState.collapseContainer(container.id);
      });

      // Step 3: Calculate areas and sort (smallest first)
      const containerAreas = containers.map(container => {
        const area = this.calculateContainerArea(container);
        return { container, area };
      }).sort((a, b) => a.area - b.area);

      this.log(`📊 Container areas calculated: ${containerAreas.map(ca => `${ca.container.id}=${ca.area}`).join(', ')}`);

      // Step 4: Get initial zoom level with all collapsed
      await this.runLayout();
      const initialZoomLevel = await this.calculateZoomLevel();
      this.log(`🔍 Initial zoom level (all collapsed): ${initialZoomLevel.toFixed(3)}`);

      // Step 5: Iteratively expand containers starting with smallest
      let expanded = 0;
      const zoomThreshold = 0.7; // Don't let zoom go below 70% of initial
      const minAcceptableZoom = initialZoomLevel * zoomThreshold;

      for (const { container } of containerAreas) {
        // Try expanding this container
        this.visState.expandContainer(container.id);
        
        // Calculate new layout and zoom
        await this.runLayout();
        const newZoomLevel = await this.calculateZoomLevel();
        
        this.log(`🔍 After expanding ${container.id}: zoom=${newZoomLevel.toFixed(3)}, threshold=${minAcceptableZoom.toFixed(3)}`);
        
        // If zoom level drops too much, revert the expansion
        if (newZoomLevel < minAcceptableZoom) {
          this.log(`⚠️ Zoom level too low, reverting expansion of ${container.id}`);
          this.visState.collapseContainer(container.id);
          await this.runLayout(); // Restore previous layout
        } else {
          expanded++;
          this.log(`✅ Kept expansion of ${container.id} (${expanded}/${containers.length})`);
        }
      }

      this.log(`🎯 Smart collapse complete: ${expanded}/${containers.length} containers expanded`);
      
    } catch (error) {
      this.handleError('Smart container collapse failed', error);
    }
  }

  /**
   * Calculate the area of a container based on its children
   */
  private calculateContainerArea(container: any): number {
    // Use number of children as a proxy for area (smaller containers = fewer children)
    const childCount = container.children ? container.children.size || container.children.length || 0 : 0;
    
    // Add a small base area to ensure containers with no children have non-zero area
    return childCount + 1;
  }

  /**
   * Calculate zoom level needed to fit all visible elements
   */
  private async calculateZoomLevel(): Promise<number> {
    try {
      // Get current bounds of all visible elements
      const bounds = this.calculateVisibleElementsBounds();
      
      if (!bounds) {
        return 1.0; // Default zoom if no bounds
      }

      // Assume a viewport size (this could be made configurable)
      const viewportWidth = 1200;
      const viewportHeight = 800;
      const padding = 50;

      // Calculate zoom to fit with padding
      const zoomX = (viewportWidth - 2 * padding) / bounds.width;
      const zoomY = (viewportHeight - 2 * padding) / bounds.height;
      
      // Use the smaller zoom to ensure everything fits
      const zoom = Math.min(zoomX, zoomY, 1.0); // Cap at 1.0 for no zoom-in
      
      this.log(`📐 Calculated zoom: bounds=${bounds.width}x${bounds.height}, zoom=${zoom.toFixed(3)}`);
      return zoom;
      
    } catch (error) {
      this.log(`⚠️ Error calculating zoom level: ${error}`);
      return 1.0; // Default zoom on error
    }
  }

  /**
   * Calculate bounding box of all visible elements
   */
  private calculateVisibleElementsBounds(): { x: number, y: number, width: number, height: number } | null {
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    let hasElements = false;

    // Include visible nodes
    const visibleNodes = this.visState.visibleNodes;
    visibleNodes.forEach(node => {
      if (node.x !== undefined && node.y !== undefined) {
        const width = node.width || 180;
        const height = node.height || 60;
        
        minX = Math.min(minX, node.x);
        minY = Math.min(minY, node.y);
        maxX = Math.max(maxX, node.x + width);
        maxY = Math.max(maxY, node.y + height);
        hasElements = true;
      }
    });

    // Include visible containers
    const visibleContainers = this.visState.visibleContainers;
    visibleContainers.forEach(container => {
      if (container.x !== undefined && container.y !== undefined) {
        const width = container.width || 200;
        const height = container.height || 100;
        
        minX = Math.min(minX, container.x);
        minY = Math.min(minY, container.y);
        maxX = Math.max(maxX, container.x + width);
        maxY = Math.max(maxY, container.y + height);
        hasElements = true;
      }
    });

    if (!hasElements) {
      return null;
    }

    return {
      x: minX,
      y: minY,
      width: maxX - minX,
      height: maxY - minY
    };
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
