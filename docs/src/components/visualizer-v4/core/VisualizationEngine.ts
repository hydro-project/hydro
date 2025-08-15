/**
 * @fileoverview Visualization Engine - Orchestrates visualization with business logic centralized
 * 
 * This engine now manages all business logic that was previously in bridges:
 * 1. Data Input → VisState
 * 2. Layout (VisState → ELK → VisState with centralized decisions) 
 * 3. Render (VisState → ReactFlow with centralized mapping)
 * 
 * Clean separation: Engine orchestrates + decides, Bridges translate, VisState stores
 */

import type { VisualizationState } from './VisState';
import { ELKBridge } from '../bridges/ELKBridge';
import { ReactFlowBridge } from '../bridges/ReactFlowBridge';
import type { ReactFlowData } from '../bridges/ReactFlowBridge';
import type { LayoutConfig } from './types';
import { LAYOUT_CONSTANTS } from '../shared/config';
import { getHandleConfig } from '../render/handleConfig';

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
  colorPalette?: string;        // Default color palette for rendering
}

const DEFAULT_CONFIG: VisualizationEngineConfig = {
  autoLayout: true,
  layoutDebounceMs: 300,
  enableLogging: true,
  colorPalette: 'Set3',
  layoutConfig: {
    enableSmartCollapse: true,
    algorithm: 'layered',
    direction: 'DOWN'
  }
};

export class VisualizationEngine {
  private visState: VisualizationState;
  private config: VisualizationEngineConfig;
  private state: VisualizationEngineState;
  private layoutTimeout?: NodeJS.Timeout;
  private listeners: Map<string, (state: VisualizationEngineState) => void> = new Map();

  constructor(
    visState: VisualizationState, 
    config: Partial<VisualizationEngineConfig> = {}
  ) {
    this.visState = visState;
    this.config = { ...DEFAULT_CONFIG, ...config };
    
    this.state = {
      phase: 'initial',
      lastUpdate: Date.now(),
      layoutCount: 0
    };

    this.log('🚀 VisualizationEngine initialized');
    this.log(`🔧 Config: ${JSON.stringify(this.config)}`);
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
    
    this.log(`🔧 Layout config updated: ${JSON.stringify(layoutConfig)}`);
    
    if (autoReLayout) {
      // Reset layout count to trigger smart collapse on algorithm change
      this.state.layoutCount = 0;
      this.runLayout();
    }
  }

  /**
   * Set color palette for rendering
   */
  setColorPalette(palette: string): void {
    this.config.colorPalette = palette;
    this.log(`🎨 Color palette updated: ${palette}`);
  }

  /**
   * Run layout on current VisState data - now with centralized business logic
   */
  async runLayout(): Promise<void> {
    this.log('📊 Layout requested');
    
    if (this.state.phase === 'laying_out') {
      this.log('⚠️ Layout already in progress, skipping');
      return;
    }

    try {
      this.updateState('laying_out');
      
      // Use refactored ELK bridge as pure transformation
      await ELKBridge.layoutVisState(this.visState, this.config.layoutConfig!);
      
      // Run smart collapse if enabled (business logic centralized here)
      if (this.config.layoutConfig?.enableSmartCollapse && (this.state.layoutCount === 0)) {
        this.log('🧠 Running smart collapse after initial layout');
        await this.runSmartCollapse();
        // Re-layout after smart collapse
        await ELKBridge.layoutVisState(this.visState, this.config.layoutConfig!);
      }

      this.state.layoutCount++;
      this.updateState('ready');
      
      this.log(`✅ Layout complete (${this.state.layoutCount} total layouts)`);
      
    } catch (error) {
      this.handleError('Layout failed', error);
    }
  }

  /**
   * Get ReactFlow data for rendering - now with centralized business logic
   */
  getReactFlowData(): ReactFlowData {
    this.log('🔄 ReactFlow data requested');
    
    if (this.state.phase === 'error') {
      throw new Error(`Cannot render in error state: ${this.state.error}`);
    }

    try {
      this.updateState('rendering');
      
      // Generate business logic data for bridges
      const parentChildMap = this.buildParentChildMap();
      const edgeHandles = this.buildEdgeHandles();
      
      // Use refactored ReactFlow bridge as pure transformation
      const reactFlowData = ReactFlowBridge.visStateToReactFlow(
        this.visState, 
        parentChildMap, 
        edgeHandles,
        this.config.colorPalette!
      );
      
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

  // ============ Centralized Business Logic (moved from bridges) ============

  /**
   * Build parent-child relationship map (business logic moved from ReactFlowBridge)
   */
  private buildParentChildMap(): Map<string, string> {
    const parentMap = new Map<string, string>();
    
    // Map nodes to their parent containers (only if parent is expanded)
    for (const node of this.visState.visibleNodes) {
      const parentContainer = this.visState.getNodeContainer(node.id);
      if (parentContainer) {
        const container = this.visState.getContainer(parentContainer);
        // Only include if parent container is expanded
        if (container && !container.collapsed && !container.hidden) {
          parentMap.set(node.id, parentContainer);
        }
      }
    }
    
    // Map containers to their parent containers
    for (const container of this.visState.visibleContainers) {
      const parentContainer = this.findContainerParent(container.id);
      if (parentContainer) {
        const parentObj = this.visState.getContainer(parentContainer);
        // Only include if parent container is expanded
        if (parentObj && !parentObj.collapsed && !parentObj.hidden) {
          parentMap.set(container.id, parentContainer);
        }
      }
    }
    
    return parentMap;
  }

  /**
   * Build edge handles map (business logic moved from ReactFlowBridge)
   */
  private buildEdgeHandles(): Map<string, { sourceHandle?: string; targetHandle?: string }> {
    const handleMap = new Map<string, { sourceHandle?: string; targetHandle?: string }>();
    const handleConfig = getHandleConfig();
    
    if (!handleConfig.enableContinuousHandles) {
      // Assign handles to edges
      for (const edge of this.visState.visibleEdges) {
        handleMap.set(edge.id, {
          sourceHandle: (edge as any).sourceHandle || 'default-out',
          targetHandle: (edge as any).targetHandle || 'default-in'
        });
      }
    }
    // For continuous handles, return empty map (ReactFlow auto-connects)
    
    return handleMap;
  }

  /**
   * Find the parent container for a given container (business logic)
   */
  private findContainerParent(containerId: string): string | undefined {
    for (const container of this.visState.visibleContainers) {
      const children = this.visState.getContainerChildren(container.id);
      if (children && children.has(containerId)) {
        return container.id;
      }
    }
    return undefined;
  }

  /**
   * Smart collapse implementation (business logic centralized here)
   */
  private async runSmartCollapse(): Promise<void> {
    try {
      this.log('🧠 Starting smart collapse algorithm');
      
      // Step 1: Get all visible containers from VisState
      const containers = this.visState.visibleContainers;
      
      if (containers.length === 0) {
        this.log('ℹ️ No containers found, skipping smart collapse');
        return;
      }
      
      this.log(`📊 Found ${containers.length} containers for smart collapse analysis`);
      
      // Step 2: Calculate container areas using layout dimensions
      const containerAreas = containers.map(container => {
        const width = (container as any).width || LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH;
        const height = (container as any).height || LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT;
        const area = width * height;
        
        return {
          container,
          area,
          width,
          height
        };
      }).sort((a, b) => a.area - b.area);
      
      // Step 3: Calculate viewport area and budget
      const viewportWidth = 1200;
      const viewportHeight = 800;
      const viewportArea = viewportWidth * viewportHeight;
      const containerAreaBudget = viewportArea * 0.7; // 70% of viewport
      
      // Step 4: Determine which containers to collapse
      let usedArea = 0;
      const containersToCollapse: string[] = [];
      
      for (const { container, area } of containerAreas) {
        if (usedArea + area <= containerAreaBudget) {
          usedArea += area;
          this.log(`✅ Keeping ${container.id} expanded`);
        } else {
          containersToCollapse.push(container.id);
          this.log(`📦 Will collapse ${container.id}`);
        }
      }
      
      // Step 5: Apply collapse decisions
      for (const containerId of containersToCollapse) {
        try {
          const container = this.visState.getContainer(containerId);
          if (container && !container.collapsed) {
            this.visState.collapseContainer(containerId);
            this.log(`📦 Collapsed container: ${containerId}`);
          }
        } catch (error) {
          this.log(`⚠️ Failed to collapse container ${containerId}: ${error}`);
        }
      }
      
      this.log('🎉 Smart collapse algorithm complete');
      
    } catch (error) {
      this.handleError('Smart collapse failed', error);
    }
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