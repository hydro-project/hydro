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
export { VisualizationState } from './core/VisState';

/**
 * Factory function to create a new VisualizationState instance.
 */
export { createVisualizationState } from './core/VisState';

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

// ============ Types and Constants ============

export { NODE_STYLES, EDGE_STYLES, CONTAINER_STYLES } from './shared/constants';
export { LAYOUT_CONSTANTS } from './core/constants';

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

// ============ Frontend Components (adapted from visualizer v2) ============

/**
 * Main visualizer component that integrates v2 frontend with v3 architecture
 */
export { Visualizer } from './v2-components/Visualizer';

/**
 * React Flow integration components
 */
export { ReactFlowInner } from './v2-components/ReactFlowInner';
export { GraphCanvas } from './v2-components/GraphCanvas';

/**
 * UI Control components
 */
export { LayoutControls } from './v2-components/LayoutControls';
export { InfoPanel } from './v2-components/InfoPanel';
export { GroupingControls } from './v2-components/GroupingControls';
export { Legend } from './v2-components/Legend';

/**
 * Specialized node components
 */
export { GroupNode } from './v2-components/GroupNode';

// ============ Utilities (adapted from visualizer v2) ============

/**
 * Layout and state management utilities
 */
export * from './v2-utils/layout';
export * from './v2-utils/reactFlowConfig';
export * from './v2-utils/constants';

// ============ Architecture Status ============

/**
 * Integration status of v3 core/bridges with v2 frontend
 */
export const INTEGRATION_STATUS = {
  core_architecture: 'vis-v3',
  frontend_logic: 'visualizer-v2',
  state_management: 'VisState.ts (single source of truth)',
  bridges: 'stateless (ELK, ReactFlow, CoordinateTranslator)',
  integration_version: '4.0.0'
} as const;
