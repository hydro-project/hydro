/**
 * @fileoverview Bridge-Based Render Module Exports
 *
 * Complete replacement for alpha render module using our bridge architecture.
 * Maintains identical API for seamless migration.
 */
export { FlowGraph } from './FlowGraph';
export { ReactFlowConverter } from './ReactFlowConverter';
export { StandardNode as GraphStandardNode, ContainerNode as GraphContainerNode } from './nodes';
export { StandardEdge as GraphStandardEdge, HyperEdge as GraphHyperEdge } from './edges';
export { createNodeEventHandlers, createEdgeEventHandlers, createContainerEventHandlers } from './eventHandlers';
export { DEFAULT_RENDER_CONFIG } from './config';
export type { RenderConfig, FlowGraphEventHandlers } from '../core/types';
//# sourceMappingURL=index.d.ts.map