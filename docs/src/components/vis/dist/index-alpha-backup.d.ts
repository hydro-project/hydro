/**
 * @fileoverview Vis - Graph Visualization System
 *
 * A modern, efficient visualization system for graphs with support for hierarchical
 * containers, edge routing, dynamic collapse/expand operations, and ReactFlow rendering
 * with ELK automatic layout.
 *
 * @version 1.0.0
 * @author Graph Visualization Team
 * @since 2025-08-01
 *
 * @example
 * ```typescript
 * import {
 *   createVisualizationState,
 *   NODE_STYLES,
 *   parseGraphJSON,
 *   FlowGraph,
 *   ELKLayoutEngine
 * } from './vis';
 *
 * // Create a new visualization state
 * const state = createVisualizationState();
 *
 * // Add nodes and edges
 * state.setGraphNode('node1', { label: 'My Node', style: NODE_STYLES.DEFAULT });
 * state.setGraphEdge('edge1', { source: 'node1', target: 'node2' });
 *
 * // Render with ReactFlow
 * <FlowGraph visualizationState={state} />
 *
 * // Parse existing graph data
 * const { state: parsedState } = parseGraphJSON(graphData);
 * ```
 */
/**
 * The current version of the vis components package.
 */
export declare const VERSION: "1.0.0";
/**
 * Core visualization state class that manages all graph elements including nodes, edges,
 * containers, and hyperEdges with efficient visibility tracking.
 *
 * @see {@link ./core/VisState.ts} for full implementation details
 */
export { VisualizationState } from './core/VisState';
/**
 * Factory function to create a new VisualizationState instance.
 * Preferred over direct constructor usage for consistency.
 *
 * @returns A new visualization state instance
 * @example
 * ```typescript
 * const state = createVisualizationState();
 * state.setGraphNode('myNode', { label: 'Hello World' });
 * ```
 */
export { createVisualizationState } from './core/VisState';
/**
 * Pre-defined node styling constants for consistent visual representation.
 *
 * @example
 * ```typescript
 * state.setGraphNode('warningNode', {
 *   label: 'Check this!',
 *   style: NODE_STYLES.WARNING
 * });
 * ```
 */
export { NODE_STYLES } from './shared/constants';
/**
 * Pre-defined edge styling constants for consistent visual representation.
 *
 * @example
 * ```typescript
 * state.setGraphEdge('importantEdge', {
 *   source: 'node1',
 *   target: 'node2',
 *   style: EDGE_STYLES.THICK
 * });
 * ```
 */
export { EDGE_STYLES } from './shared/constants';
/**
 * Pre-defined container styling constants for hierarchical groupings.
 */
export { CONTAINER_STYLES } from './shared/constants';
/**
 * Layout dimension constants for consistent spacing and sizing.
 */
export { LAYOUT_CONSTANTS } from './shared/constants';
/**
 * TypeScript type definitions for better development experience.
 */
export type { NodeStyle, EdgeStyle, ContainerStyle, Dimensions, GraphNode, GraphEdge, Container, HyperEdge, CreateNodeProps, CreateEdgeProps, CreateContainerProps } from './shared/types';
/**
 * Parse graph JSON data and create a populated VisualizationState.
 * Converts JSON data into the state management system.
 *
 * @param jsonData - The JSON data (object or JSON string)
 * @param selectedGrouping - Which hierarchy grouping to use (defaults to first available)
 * @returns Object containing the populated state and metadata
 * @throws {Error} When JSON data is invalid or malformed
 * @example
 * ```typescript
 * const { state, metadata } = parseGraphJSON(graphData, 'myGrouping');
 * console.log(`Parsed ${state.visibleNodes.length} nodes`);
 * console.log(`Used grouping: ${metadata.selectedGrouping}`);
 * ```
 */
export { parseGraphJSON } from './core/JSONParser';
/**
 * Create a reusable parser instance for processing multiple graph datasets.
 * Useful when parsing multiple graphs with similar structure/settings.
 *
 * @param options - Parser configuration options
 * @returns Parser function that accepts JSON data
 */
export { createGraphParser } from './core/JSONParser';
/**
 * Extract available hierarchical groupings from Hydro graph JSON data.
 * Useful for presenting grouping options to users before parsing.
 *
 * @param jsonData - The JSON data (object or JSON string)
 * @returns Array of available grouping objects
 * @example
 * ```typescript
 * const groupings = getAvailableGroupings(graphData);
 * groupings.forEach(g => console.log(`${g.name} (${g.id})`));
 * ```
 */
export { getAvailableGroupings } from './core/JSONParser';
/**
 * Validate graph JSON data structure and content.
 * Provides detailed validation results including errors and warnings.
 *
 * @param jsonData - The JSON data (object or JSON string)
 * @returns Validation result object
 * @example
 * ```typescript
 * const validation = validateGraphJSON(suspiciousData);
 * if (!validation.isValid) {
 *   console.error('Validation failed:', validation.errors);
 *   return;
 * }
 * if (validation.warnings.length > 0) {
 *   console.warn('Warnings found:', validation.warnings);
 * }
 * ```
 */
export { validateGraphJSON } from './core/JSONParser';
/**
 * ELK-based automatic layout engine for positioning graph elements.
 * Supports hierarchical layouts, multiple algorithms, and custom spacing.
 */
export { ELKLayoutEngine, DEFAULT_LAYOUT_CONFIG } from './layout/index';
/**
 * Layout configuration and result types for the ELK layout engine.
 */
export type { LayoutConfig, LayoutResult, LayoutEngine } from './layout/index';
/**
 * ReactFlow-based graph visualization component with custom nodes and edges.
 * Integrates with ELK layout engine for automatic positioning.
 */
export { FlowGraph as FlowGraph, ReactFlowConverter, GraphStandardNode, GraphContainerNode, GraphStandardEdge, GraphHyperEdge, DEFAULT_RENDER_CONFIG } from './render/index';
/**
 * ReactFlow rendering configuration and event handler types.
 */
export type { RenderConfig, FlowGraphEventHandlers as FlowGraphEventHandlers } from './render/index';
/**
 * Parser and validation result types for better TypeScript integration.
 */
export type { ParseResult, ValidationResult, GroupingOption, ParserOptions } from './core/JSONParser';
//# sourceMappingURL=index-alpha-backup.d.ts.map