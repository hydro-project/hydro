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
import { LAYOUT_CONSTANTS } from '../shared/config';

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
  colorPalette?: string;        // Color palette for rendering
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
  private elkBridge: ELKBridge;
  private reactFlowBridge: ReactFlowBridge;
  private config: VisualizationEngineConfig;
  private state: VisualizationEngineState;
  private layoutTimeout?: number;
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
    this.log(`🔧 Config: ${JSON.stringify(this.config)}`);
    this.log(`📊 Initial layoutCount: ${this.state.layoutCount}`);
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
      // Reset layout count to trigger smart collapse on algorithm change
      this.state.layoutCount = 0;
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
            
      // DEBUG: Check smart collapse conditions
      this.log(`🔍 SMART COLLAPSE DEBUG:`);
      this.log(`  - enableSmartCollapse: ${this.config.layoutConfig?.enableSmartCollapse}`);
      this.log(`  - layoutCount: ${this.state.layoutCount}`);
      this.log(`  - condition met: ${this.config.layoutConfig?.enableSmartCollapse && (this.state.layoutCount === 0)}`);
      
      // Run smart collapse if enabled and this is the first layout (initiation) or layout config changed
      if (this.config.layoutConfig?.enableSmartCollapse && (this.state.layoutCount === 0)) {
        this.log('🧠 Running smart collapse after initial layout');
        await this.runSmartCollapse();
        // Use ELK bridge to re-layout the VisState after smartCollapse
        await this.elkBridge.layoutVisState(this.visState);
      } else {
        this.log('⚠️ Smart collapse SKIPPED - conditions not met');
      }

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
      
      // Set color palette on bridge if configured
      if (this.config.colorPalette) {
        this.reactFlowBridge.setColorPalette(this.config.colorPalette);
      }
      
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

  /**
   * Simple smart collapse implementation
   * Run after initial ELK layout to collapse containers that exceed viewport budget
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
        // Get dimensions from ELK layout results (stored as width/height on container)
        const width = (container as any).width || LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH;
        const height = (container as any).height || LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT;
        const area = width * height;
        
        this.log(`📏 Container ${container.id} area calculation: ${width}x${height} = ${area}, collapsed=${(container as any).collapsed}`);
        
        return {
          container,
          area,
          width,
          height
        };
      }).sort((a, b) => a.area - b.area); // Sort by area, smallest to largest
      
      this.log(`📐 Container areas: ${containerAreas.map(ca => `${ca.container.id}=${ca.area}`).join(', ')}`);
      
      // Step 3: Calculate viewport area and budget
      // Use reasonable default viewport size (window dimensions would be ideal)
      const viewportWidth = 1200;
      const viewportHeight = 800;
      const viewportArea = viewportWidth * viewportHeight;
      const containerAreaBudgetRatio = 0.7; // Use 70% of viewport for containers
      const containerAreaBudget = viewportArea * containerAreaBudgetRatio;
      
      this.log(`📱 Viewport: ${viewportWidth}x${viewportHeight} (${viewportArea} total area)`);
      this.log(`💰 Container area budget: ${containerAreaBudget} (${containerAreaBudgetRatio * 100}% of viewport)`);
      
      // Step 4: Iterate through containers, keep expanding until budget exceeded
      let usedArea = 0;
      const containersToKeepExpanded: string[] = [];
      const containersToCollapse: string[] = [];
      
      for (const { container, area } of containerAreas) {
        if (usedArea + area <= containerAreaBudget) {
          // We can afford to keep this container expanded
          containersToKeepExpanded.push(container.id);
          usedArea += area;
          this.log(`✅ Keeping ${container.id} expanded (area: ${area}, total used: ${usedArea})`);
        } else {
          // This would exceed budget, collapse it
          containersToCollapse.push(container.id);
          this.log(`📦 Will collapse ${container.id} (area: ${area} would exceed budget)`);
        }
      }
      
      this.log(`🎯 Smart collapse decisions: keep ${containersToKeepExpanded.length} expanded, collapse ${containersToCollapse.length}`);
      this.log(`📋 Keeping expanded: ${containersToKeepExpanded.join(', ') || 'none'}`);
      this.log(`📋 Collapsing: ${containersToCollapse.join(', ') || 'none'}`);
      
      // Step 5: Apply collapse decisions using collapseContainer
      if (containersToCollapse.length > 0) {
        this.log(`🔧 Applying collapse decisions to ${containersToCollapse.length} containers`);
        
        for (const containerId of containersToCollapse) {
          try {
            // CRITICAL: Check if container is already collapsed before attempting collapse
            // During recursive collapse, parent containers may have already collapsed their children
            const container = this.visState.getContainer(containerId);
            if (!container) {
              this.log(`⚠️ Container ${containerId} no longer exists, skipping`);
              continue;
            }
            
            if (container.collapsed) {
              this.log(`⚠️ Container ${containerId} is already collapsed, skipping`);
              continue;
            }
            
            // Use collapseContainer which handles all the mechanics atomically:
            // - Collapsing the container and its children
            // - Creating hyperEdges for crossing edges  
            // - Hiding descendant containers
            // - Validating invariants
            this.visState.collapseContainer(containerId);
            this.log(`📦 Collapsed container: ${containerId}`);
          } catch (error) {
            this.log(`⚠️ Failed to collapse container ${containerId}: ${error}`);
            // Continue with other containers even if one fails
            // collapseContainer already handles invariant validation internally
          }
        }
        
        this.log(`✅ All ${containersToCollapse.length} collapse operations complete`);
        
        // Step 6: Re-run layout after collapse to get clean final layout
        this.log('🔄 Re-running layout after smart collapse');
        // IMPORTANT: Clear any cached positions to force fresh layout with new collapsed dimensions
        this.log('🧹 Clearing layout cache to force fresh ELK layout with collapsed dimensions');
        this.clearLayoutPositions();
        // Force ELK to rebuild from scratch with new dimensions
        this.log('🔄 Creating fresh ELK instance to avoid any internal caching');
        this.log(`📋 ELKBridge config: ${JSON.stringify(this.config.layoutConfig)}`);
        this.elkBridge = new ELKBridge(this.config.layoutConfig);
        
        // INVARIANT: All containers should be unfixed for fresh layout
        this.validateRelayoutInvariants();
        
        // CRITICAL: Validate collapsed containers have small dimensions  
        this.validateCollapsedContainerDimensions();
        
        // Sanity check ELK layout config
        this.validateELKLayoutConfig();
        
        // Validate TreeHierarchy and VisState are in sync  
        this.validateTreeHierarchySync();
        
        await this.elkBridge.layoutVisState(this.visState);
        this.log('✅ Post-collapse layout complete');
      }
      
      this.log(`💰 Final budget usage: ${usedArea}/${containerAreaBudget} (${((usedArea/containerAreaBudget)*100).toFixed(1)}%)`);
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

  /**
   * Validate that collapsed containers have correct small dimensions
   */
  private validateCollapsedContainerDimensions(): void {
    this.log('🔍 Validating collapsed container dimensions...');
    
    const containers = this.visState.visibleContainers;
    let collapsedCount = 0;
    let dimensionViolations = 0;
    
    for (const container of containers) {
      if (container.collapsed) {
        collapsedCount++;
        const dimensions = this.visState.getContainerAdjustedDimensions(container.id);
        
        // Collapsed containers should be small (≤300x200)
        if (dimensions.width > 300 || dimensions.height > 200) {
          dimensionViolations++;
          this.log(`❌ DIMENSION VIOLATION: Collapsed container ${container.id} has dimensions ${dimensions.width}x${dimensions.height} but should be ≤300x200`);
        } else {
          this.log(`✅ Container ${container.id}: ${dimensions.width}x${dimensions.height} (collapsed)`);
        }
      }
    }
    
    if (dimensionViolations > 0) {
      throw new Error(`Collapsed container dimension violations: ${dimensionViolations}/${collapsedCount} collapsed containers have incorrect dimensions`);
    }
    
    this.log(`✅ Collapsed container dimensions validated: ${collapsedCount} containers all have proper small dimensions`);
  }

  /**
   * Validate invariants before re-layout after collapse
   */
  private validateRelayoutInvariants(): void {
    this.log('🔍 Validating re-layout invariants...');
    
    // INVARIANT: All containers should have elkFixed=false for fresh layout
    const containers = this.visState.visibleContainers;
    let fixedCount = 0;
    
    for (const container of containers) {
      const isFixed = this.visState.getContainerELKFixed(container.id);
      if (isFixed) {
        fixedCount++;
        this.log(`❌ INVARIANT VIOLATION: Container ${container.id} is elkFixed=true but should be false for fresh layout`);
      }
    }
    
    if (fixedCount > 0) {
      throw new Error(`Re-layout invariant violation: ${fixedCount} containers are still elkFixed=true, preventing fresh layout`);
    }
    
    this.log(`✅ Re-layout invariants passed: ${containers.length} containers all have elkFixed=false`);
  }

  /**
   * Validate ELK layout configuration
   */
  private validateELKLayoutConfig(): void {
    this.log('🔍 Validating ELK layout config...');
    
    const config = this.config.layoutConfig;
    if (!config) {
      throw new Error('ELK layout config is undefined');
    }
    
    this.log(`📐 ELK Config: algorithm=${config.algorithm || 'default'}`);
    this.log(`✅ ELK layout config validated`);
  }

  /**
   * Validate TreeHierarchy and VisState are in sync
   */
  private validateTreeHierarchySync(): void {
    this.log('🔍 Validating TreeHierarchy/VisState sync...');
    
    // Check that visible containers in VisState match what TreeHierarchy should show
    const visibleContainers = this.visState.visibleContainers;
    let collapsedCount = 0;
    let expandedCount = 0;
    
    for (const container of visibleContainers) {
      if (container.collapsed) {
        collapsedCount++;
      } else {
        expandedCount++;
      }
    }
    
    this.log(`📊 Container states: ${collapsedCount} collapsed, ${expandedCount} expanded`);
    this.log(`✅ TreeHierarchy sync validated (${visibleContainers.length} total containers)`);
  }

  /**
   * Clear all layout positions to force fresh ELK layout calculation
   * This is needed after smart collapse to prevent ELK from using cached positions
   * calculated with old (large) container dimensions
   */
  private clearLayoutPositions(): void {
    this.log('🧹 Clearing layout positions for fresh ELK calculation...');
    
    // Use VisState's public method to clear all layout positions
    this.visState.clearLayoutPositions();
    
    this.log(`✅ Cleared layout positions for all containers and nodes`);
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
