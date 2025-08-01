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
 * ```javascript
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
 * @constant {string}
 */
export const VERSION = '1.0.0';

// ============ State Management ============

/**
 * Core visualization state class that manages all graph elements including nodes, edges, 
 * containers, and hyperEdges with efficient visibility tracking.
 * 
 * @class VisualizationState
 * @see {@link ./VisState.js} for full implementation details
 */
export { VisualizationState } from './VisState.js';

/**
 * Factory function to create a new VisualizationState instance.
 * Preferred over direct constructor usage for consistency.
 * 
 * @function createVisualizationState
 * @returns {VisualizationState} A new visualization state instance
 * @example
 * ```javascript
 * const state = createVisualizationState();
 * state.setGraphNode('myNode', { label: 'Hello World' });
 * ```
 */
export { createVisualizationState } from './VisState.js';

// ============ Styling and Layout Constants ============

/**
 * Pre-defined node styling constants for consistent visual representation.
 * 
 * @namespace NODE_STYLES
 * @property {string} DEFAULT - Standard node appearance
 * @property {string} HIGHLIGHTED - Emphasized node for user attention
 * @property {string} SELECTED - Currently selected node
 * @property {string} WARNING - Node indicating warning state
 * @property {string} ERROR - Node indicating error state
 * @example
 * ```javascript
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
 * @namespace EDGE_STYLES
 * @property {string} DEFAULT - Standard edge appearance
 * @property {string} HIGHLIGHTED - Emphasized edge for user attention
 * @property {string} DASHED - Dashed line style for conditional connections
 * @property {string} THICK - Thick line for important connections
 * @property {string} WARNING - Edge indicating warning state
 * @example
 * ```javascript
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
 * 
 * @namespace CONTAINER_STYLES
 * @property {string} DEFAULT - Standard container appearance
 * @property {string} HIGHLIGHTED - Emphasized container for user attention
 * @property {string} SELECTED - Currently selected container
 * @property {string} MINIMIZED - Collapsed/minimized container state
 */
export { CONTAINER_STYLES } from './constants.js';

/**
 * Layout dimension constants for consistent spacing and sizing.
 * 
 * @namespace LAYOUT_CONSTANTS
 * @property {number} DEFAULT_NODE_WIDTH - Standard node width in pixels
 * @property {number} DEFAULT_NODE_HEIGHT - Standard node height in pixels
 * @property {number} DEFAULT_CONTAINER_PADDING - Container padding in pixels
 * @property {number} MIN_CONTAINER_WIDTH - Minimum container width in pixels
 * @property {number} MIN_CONTAINER_HEIGHT - Minimum container height in pixels
 */
export { LAYOUT_CONSTANTS } from './constants.js';

// ============ JSON Data Processing ============

/**
 * Parse Hydro graph JSON data and create a populated VisualizationState.
 * Converts legacy visualization format into the new state management system.
 * 
 * @function parseHydroGraphJSON
 * @param {Object|string} jsonData - The JSON data (object or JSON string)
 * @param {string} [selectedGrouping] - Which hierarchy grouping to use (defaults to first available)
 * @returns {Object} Object containing the populated state and metadata
 * @returns {VisualizationState} returns.state - The populated visualization state
 * @returns {Object} returns.metadata - Parsing metadata including selected grouping
 * @throws {Error} When JSON data is invalid or malformed
 * @example
 * ```javascript
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
 * @function createHydroGraphParser
 * @param {Object} [options] - Parser configuration options
 * @returns {Function} Parser function that accepts JSON data
 */
export { createHydroGraphParser } from './JSONParser.js';

/**
 * Extract available hierarchical groupings from Hydro graph JSON data.
 * Useful for presenting grouping options to users before parsing.
 * 
 * @function getAvailableGroupings
 * @param {Object|string} jsonData - The JSON data (object or JSON string)
 * @returns {Array<Object>} Array of available grouping objects
 * @returns {string} returns[].id - Unique identifier for the grouping
 * @returns {string} returns[].name - Human-readable name for the grouping
 * @example
 * ```javascript
 * const groupings = getAvailableGroupings(hydroData);
 * groupings.forEach(g => console.log(`${g.name} (${g.id})`));
 * ```
 */
export { getAvailableGroupings } from './JSONParser.js';

/**
 * Validate Hydro graph JSON data structure and content.
 * Provides detailed validation results including errors and warnings.
 * 
 * @function validateHydroGraphJSON
 * @param {Object|string} jsonData - The JSON data (object or JSON string)
 * @returns {Object} Validation result object
 * @returns {boolean} returns.isValid - Whether the data is valid
 * @returns {Array<string>} returns.errors - Critical validation errors
 * @returns {Array<string>} returns.warnings - Non-critical validation warnings
 * @returns {number} returns.nodeCount - Number of nodes found
 * @returns {number} returns.edgeCount - Number of edges found
 * @returns {number} returns.hierarchyCount - Number of hierarchies found
 * @example
 * ```javascript
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
