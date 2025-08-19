/**
 * @fileoverview Visualizer v4 - Integration of v3 Core/Bridges with v2 Frontend
 * 
 * This combines the improved front-end logic from visualizer (v2) with the 
 * modern, clean architecture of vis (v3). The result maintains:
 * - VisState.ts as the single source of truth for app state
 * - All bridges remain stateless  
 * - Non-transient state flows through core/VisState.ts
 * 
 * @version 4.0.0
 * @author Hydro Project
 */

/**
 * The current version of the visualizer-v4 package.
 */
export const VERSION = '4.0.0' as const;

// ============ Core State Management (from vis v3) ============

/**
 * Core visualization state class - single source of truth
 */
export { VisualizationState } from './core/VisualizationState';

/**
 * Factory function to create a new VisualizationState instance.
 */
export { createVisualizationState } from './core/VisualizationState';

// ============ Bridge Architecture (from vis v3) ============

/**
 * Stateless bridge components for layout and rendering
 */
export { ELKBridge } from './bridges/ELKBridge';
export { ReactFlowBridge } from './bridges/ReactFlowBridge';
export { CoordinateTranslator } from './bridges/CoordinateTranslator';

// ============ Visualization Engine (from vis v3) ============

export { VisualizationEngine, createVisualizationEngine } from './core/VisualizationEngine';
export type { VisualizationEngineConfig } from './core/VisualizationEngine';

// ============ JSON Parsing ============

export { parseGraphJSON, createGraphParser, getAvailableGroupings, validateGraphJSON, createRenderConfig } from './core/JSONParser';
export type { ParseResult, ValidationResult, GroupingOption, ParserOptions } from './core/JSONParser';

// ============ Rendering Components ============

export { FlowGraph } from './render/FlowGraph';
export { DEFAULT_RENDER_CONFIG } from './render/config';
export type { RenderConfig, FlowGraphEventHandlers } from './core/types';

// ============ Types and Constants ============

export { NODE_STYLES, EDGE_STYLES, CONTAINER_STYLES, LAYOUT_CONSTANTS } from './shared/config';

export type {
  NodeStyle,
  EdgeStyle,
  ContainerStyle,
  Dimensions,
  GraphNode,
  GraphEdge,
  Container,
  HyperEdge,
  CreateNodeProps,
  CreateEdgeProps,
  CreateContainerProps
} from './shared/types';

export type { ReactFlowData } from './bridges/ReactFlowBridge';

// ============ Frontend Components ============

/**
 * File upload and drop zone component
 */
export { default as FileDropZone } from './components/FileDropZone';

/**
 * UI Control components
 */
export { LayoutControls } from './components/LayoutControls';
export { StyleTunerPanel } from './components/StyleTunerPanel';
export { InfoPanel } from './components/InfoPanel';
export { GroupingControls } from './components/GroupingControls';
export { Legend } from './components/Legend';

/**
 * Additional UI components
 */
export { HierarchyTree } from './components/HierarchyTree';
export { CollapsibleSection } from './components/CollapsibleSection';
export { AntDockablePanel, DockablePanel } from './components/AntDockablePanel';
export { useDockablePanels } from './hooks/useDockablePanels';

// ============ Architecture Status ============

/**
 * Integration status of v4 visualizer components
 */
export const INTEGRATION_STATUS = {
  core_architecture: 'vis-v3',
  components: 'v4-unified',
  state_management: 'VisState.ts (single source of truth)',
  bridges: 'stateless (ELK, ReactFlow, CoordinateTranslator)',
  integration_version: '4.0.0'
} as const;
