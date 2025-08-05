/**
 * Visualization Design Constants
 *
 * @deprecated This file is deprecated. Use ../shared/config.ts instead.
 * Re-exports for backward compatibility.
 */
// Re-export the new comprehensive configuration
export * from './config';
// Legacy constants for backward compatibility
// @deprecated Use NODE_STYLES from config.ts instead
export const NODE_STYLES = {
    DEFAULT: 'default',
    HIGHLIGHTED: 'highlighted',
    SELECTED: 'selected',
    WARNING: 'warning',
    ERROR: 'error'
};
// @deprecated Use EDGE_STYLES from config.ts instead
export const EDGE_STYLES = {
    DEFAULT: 'default',
    HIGHLIGHTED: 'highlighted',
    DASHED: 'dashed',
    THICK: 'thick',
    WARNING: 'warning'
};
// @deprecated Use CONTAINER_STYLES from config.ts instead
export const CONTAINER_STYLES = {
    DEFAULT: 'default',
    HIGHLIGHTED: 'highlighted',
    SELECTED: 'selected',
    MINIMIZED: 'minimized'
};
// @deprecated Use LAYOUT_CONSTANTS from config.ts instead
export const LAYOUT_CONSTANTS = {
    DEFAULT_NODE_WIDTH: 100,
    DEFAULT_NODE_HEIGHT: 40,
    DEFAULT_CONTAINER_PADDING: 20,
    MIN_CONTAINER_WIDTH: 150,
    MIN_CONTAINER_HEIGHT: 100
};
// Log prefixes for consistent logging
export const LOG_PREFIXES = {
    STATE_MANAGER: '[ELKStateManager]',
    FULL_LAYOUT: '🏗️ FULL_LAYOUT:',
    VISUAL_LAYOUT: '🎯 VISUAL_LAYOUT:',
    VALIDATION: '🔍',
    CACHING: '💾 CACHING:',
    SUMMARY: '📊 SUMMARY:',
    CONTAINER: '📦',
    INPUT: 'INPUT',
    OUTPUT: 'OUTPUT',
    SUCCESS: '✅',
    WARNING: '⚠️',
    ERROR: '❌',
};
//# sourceMappingURL=constants.js.map