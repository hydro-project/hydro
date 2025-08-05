/**
 * @fileoverview New Bridge Architecture Exports
 *
 * Clean exports for the new visualization architecture with bridges
 */
export { VisualizationState, createVisualizationState } from './core/VisState';
export { ELKBridge } from './bridges/ELKBridge';
export { ReactFlowBridge } from './bridges/ReactFlowBridge';
export { CoordinateTranslator } from './bridges/CoordinateTranslator';
export type { ReactFlowData, ReactFlowNode, ReactFlowEdge } from './bridges/ReactFlowBridge';
export type { ElkGraph, ElkNode, ElkEdge } from './bridges/elk-types';
export { VisualizationEngine, createVisualizationEngine } from './core/VisualizationEngine';
export type { VisualizationPhase, VisualizationEngineState, VisualizationEngineConfig } from './core/VisualizationEngine';
export { useVisualization, VisualizationProvider } from './hooks/useVisualization';
export type { UseVisualizationResult, UseVisualizationConfig } from './hooks/useVisualization';
export { VisualizationComponent, ExampleVisualization } from './components/VisualizationComponent';
export * from './shared/types';
export * from './shared/constants';
export type { GraphNode, GraphEdge, Container, HyperEdge, NodeStyle, EdgeStyle, ContainerStyle } from './shared/types';
//# sourceMappingURL=new-index.d.ts.map