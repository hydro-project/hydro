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
// Orchestration Engine
export { VisualizationEngine, createVisualizationEngine } from './core/VisualizationEngine';
// React Integration
export { useVisualization, VisualizationProvider } from './hooks/useVisualization';
// React Components
export { VisualizationComponent, ExampleVisualization } from './components/VisualizationComponent';
// Legacy Alpha (for gradual migration)
// Export the new shared types and constants
export * from './shared/types';
export * from './shared/constants';
//# sourceMappingURL=new-index.js.map