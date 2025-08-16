/**
 * @fileoverview Bridge-Based Render Module Exports
 * 
 * Complete replacement for alpha render module using our bridge architecture.
 * Maintains identical API for seamless migration.
 */

// Main components
export { FlowGraph } from './FlowGraph';

// Node and edge components for compatibility
export { StandardNode as GraphStandardNode, ContainerNode as GraphContainerNode } from './nodes';
export { StandardEdge as GraphStandardEdge, HyperEdge as GraphHyperEdge } from './edges';

// Event handlers (deprecated but for compatibility)
export { 
  createNodeEventHandlers, 
  createEdgeEventHandlers, 
  createContainerEventHandlers 
} from './eventHandlers';

// Configuration
export { DEFAULT_RENDER_CONFIG } from './config';

// Re-export our own types
export type {
  RenderConfig,
  FlowGraphEventHandlers
} from '../core/types';
