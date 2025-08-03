/**
 * @fileoverview New Bridge Architecture Exports
 * 
 * Clean exports for the new visualization architecture with bridges
 */

// Core State Management
export { VisualizationState, createVisualizationState } from './core/VisState';

// Bridge Architecture  
export { ELKBridge } from './bridges/ELKBridge';
export { ReactFlowBridge } from './bridges/ReactFlowBridge';
export { CoordinateTranslator } from './bridges/CoordinateTranslator';
export type { ReactFlowData, ReactFlowNode, ReactFlowEdge } from './bridges/ReactFlowBridge';
export type { ElkGraph, ElkNode, ElkEdge } from './bridges/elk-types';

// Orchestration Engine
export { VisualizationEngine, createVisualizationEngine } from './core/VisualizationEngine';
export type { 
  VisualizationPhase, 
  VisualizationEngineState, 
  VisualizationEngineConfig 
} from './core/VisualizationEngine';

// React Integration
export { useVisualization, VisualizationProvider } from './hooks/useVisualization';
export type { UseVisualizationResult, UseVisualizationConfig } from './hooks/useVisualization';

// React Components
export { VisualizationComponent, ExampleVisualization } from './components/VisualizationComponent';

// Legacy Alpha (for gradual migration)
// Export the new shared types and constants
export * from './shared/types';
export * from './shared/constants';

// Types from current implementation
export type {
  GraphNode,
  GraphEdge,
  Container,
  HyperEdge,
  NodeStyle,
  EdgeStyle,
  ContainerStyle
} from './shared/types';