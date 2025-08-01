/**
 * @fileoverview Vis - Next Generation Hydro Graph Visualizer
 *
 * A modern, efficient visualization system for Hydro graphs with support for hierarchical
 * containers, edge routing, and dynamic collapse/expand operations.
 *
 * @version 1.0.0
 * @author Hydro Project
 * @since 2025-08-01
 *
 * @example
 * ```typescript
 * import { createVisualizationState, NODE_STYLES, parseHydroGraphJSON } from './vis';
 *
 * // Create a new visualization state
 * const state = createVisualizationState();
 *
 * // Add nodes and edges
 * state.setGraphNode('node1', { label: 'My Node', style: NODE_STYLES.DEFAULT });
 * state.setGraphEdge('edge1', { source: 'node1', target: 'node2' });
 *
 * // Parse existing Hydro graph data
 * const { state: parsedState } = parseHydroGraphJSON(hydroGraphData);
 * ```
 */
/**
 * The current version of the vis components package.
 */
export const VERSION = '1.0.0';
// ============ State Management ============
/**
 * Core visualization state class that manages all graph elements including nodes, edges,
 * containers, and hyperEdges with efficient visibility tracking.
 *
 * @see {@link ./VisState.ts} for full implementation details
 */
export { VisualizationState } from './VisState.js';
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
export { createVisualizationState } from './VisState.js';
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
export { NODE_STYLES } from './constants.js';
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
export { EDGE_STYLES } from './constants.js';
/**
 * Pre-defined container styling constants for hierarchical groupings.
 */
export { CONTAINER_STYLES } from './constants.js';
/**
 * Layout dimension constants for consistent spacing and sizing.
 */
export { LAYOUT_CONSTANTS } from './constants.js';
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
export { parseHydroGraphJSON } from './JSONParser.js';
/**
 * Create a reusable parser instance for processing multiple Hydro graph datasets.
 * Useful when parsing multiple graphs with similar structure/settings.
 *
 * @param options - Parser configuration options
 * @returns Parser function that accepts JSON data
 */
export { createHydroGraphParser } from './JSONParser.js';
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
export { getAvailableGroupings } from './JSONParser.js';
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
export { validateHydroGraphJSON } from './JSONParser.js';
//# sourceMappingURL=index.js.map