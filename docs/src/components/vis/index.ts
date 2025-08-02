/**
 * @fileoverview Vis - Next Generation Hydro Graph Visualizer
 * 
 * A modern, efficient visualization system for Hydro graphs with support for hierarchical 
 * containers, edge routing, dynamic collapse/expand operations, and ReactFlow rendering
 * with ELK automatic layout.
 * 
 * @version 1.0.0
 * @author Hydro Project
 * @since 2025-08-01
 * 
 * @example
 * ```typescript
 * import { 
 *   createVisualizationState, 
 *   NODE_STYLES, 
 *   parseHydroGraphJSON,
 *   GraphFlow,
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
 * <GraphFlow visualizationState={state} />
 * 
 * // Parse existing Hydro graph data
 * const { state: parsedState } = parseHydroGraphJSON(hydroGraphData);
 * ```
 */

/**
 * The current version of the vis components package.
 */
export const VERSION = '1.0.0' as const;

// ============ State Management ============

/**
 * Core visualization state class that manages all graph elements including nodes, edges, 
 * containers, and hyperEdges with efficient visibility tracking.
 * 
 * @see {@link ./core/VisState.ts} for full implementation details
 */
export { VisualizationState } from './core/VisState.js';

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
export { createVisualizationState } from './core/VisState.js';

// ============ Types and Constants ============

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
export { NODE_STYLES } from './shared/constants.js';

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
export { EDGE_STYLES } from './shared/constants.js';

/**
 * Pre-defined container styling constants for hierarchical groupings.
 */
export { CONTAINER_STYLES } from './shared/constants.js';

/**
 * Layout dimension constants for consistent spacing and sizing.
 */
export { LAYOUT_CONSTANTS } from './shared/constants.js';

/**
 * TypeScript type definitions for better development experience.
 */
export type {
  NodeStyle,
  EdgeStyle,
  ContainerStyle,
  Dimensions,
  GraphNode,
  GraphEdge,
  Container,
  HyperEdge,
  CollapsedContainer,
  CreateNodeProps,
  CreateEdgeProps,
  CreateContainerProps
} from './shared/constants.js';

// ============ JSON Data Processing ============

/**
 * Parse Hydro graph JSON data and create a populated VisualizationState.
 * Converts legacy visualization format into the new state management system.
 * 
 * @param jsonData - The JSON data (object or JSON string)
 * @param selectedGrouping - Which hierarchy grouping to use (defaults to first available)
 * @returns Object containing the populated state and metadata
 * @throws {Error} When JSON data is invalid or malformed
 * @example
 * ```typescript
 * const { state, metadata } = parseHydroGraphJSON(hydroData, 'myGrouping');
 * console.log(`Parsed ${state.getVisibleNodes().length} nodes`);
 * console.log(`Used grouping: ${metadata.selectedGrouping}`);
 * ```
 */
export { parseHydroGraphJSON } from './core/JSONParser.js';

/**
 * Create a reusable parser instance for processing multiple Hydro graph datasets.
 * Useful when parsing multiple graphs with similar structure/settings.
 * 
 * @param options - Parser configuration options
 * @returns Parser function that accepts JSON data
 */
export { createHydroGraphParser } from './core/JSONParser.js';

/**
 * Extract available hierarchical groupings from Hydro graph JSON data.
 * Useful for presenting grouping options to users before parsing.
 * 
 * @param jsonData - The JSON data (object or JSON string)
 * @returns Array of available grouping objects
 * @example
 * ```typescript
 * const groupings = getAvailableGroupings(hydroData);
 * groupings.forEach(g => console.log(`${g.name} (${g.id})`));
 * ```
 */
export { getAvailableGroupings } from './core/JSONParser.js';

/**
 * Validate Hydro graph JSON data structure and content.
 * Provides detailed validation results including errors and warnings.
 * 
 * @param jsonData - The JSON data (object or JSON string)
 * @returns Validation result object
 * @example
 * ```typescript
 * const validation = validateHydroGraphJSON(suspiciousData);
 * if (!validation.isValid) {
 *   console.error('Validation failed:', validation.errors);
 *   return;
 * }
 * if (validation.warnings.length > 0) {
 *   console.warn('Warnings found:', validation.warnings);
 * }
 * ```
 */
export { validateHydroGraphJSON } from './core/JSONParser.js';

// ============ Layout Engine ============

/**
 * ELK-based automatic layout engine for positioning graph elements.
 * Supports hierarchical layouts, multiple algorithms, and custom spacing.
 */
export { 
  ELKLayoutEngine,
  DEFAULT_LAYOUT_CONFIG,
  LAYOUT_ALGORITHMS,
  LAYOUT_DIRECTIONS
} from './layout/index.js';

/**
 * Layout configuration and result types for the ELK layout engine.
 */
export type {
  LayoutConfig,
  LayoutPosition,
  LayoutDimensions,
  PositionedNode,
  PositionedEdge,
  PositionedContainer,
  PositionedHyperEdge,
  LayoutResult,
  LayoutEngine
} from './layout/index.js';

// ============ ReactFlow Renderer ============

/**
 * ReactFlow-based graph visualization component with custom nodes and edges.
 * Integrates with ELK layout engine for automatic positioning.
 */
export { 
  GraphFlow,
  ReactFlowConverter,
  GraphStandardNode,
  GraphContainerNode,
  GraphStandardEdge,
  GraphHyperEdge,
  DEFAULT_RENDER_CONFIG,
  NODE_STYLE_CLASSES,
  EDGE_STYLE_CLASSES,
  CONTAINER_STYLE_CLASSES
} from './render/index.js';

/**
 * ReactFlow rendering configuration and event handler types.
 */
export type {
  RenderConfig,
  GraphFlowEventHandlers
} from './render/index.js';

/**
 * Parser and validation result types for better TypeScript integration.
 */
export type {
  ParseResult,
  ValidationResult,
  GroupingOption,
  ParserOptions
} from './core/JSONParser.js';
