/**
 * Visualization Design Constants
 * 
 * @deprecated This file is deprecated. Use ../shared/config.ts instead.
 * Re-exports for backward compatibility.
 */

// Re-export the new comprehensive configuration
export * from './config.js';

// Legacy constants for backward compatibility
// @deprecated Use NODE_STYLES from config.ts instead
export const NODE_STYLES = {
  DEFAULT: 'default',
  HIGHLIGHTED: 'highlighted',
  SELECTED: 'selected',
  WARNING: 'warning',
  ERROR: 'error'
} as const;

// @deprecated Use EDGE_STYLES from config.ts instead
export const EDGE_STYLES = {
  DEFAULT: 'default',
  HIGHLIGHTED: 'highlighted',
  DASHED: 'dashed',
  THICK: 'thick',
  WARNING: 'warning'
} as const;

// @deprecated Use CONTAINER_STYLES from config.ts instead
export const CONTAINER_STYLES = {
  DEFAULT: 'default',
  HIGHLIGHTED: 'highlighted',
  SELECTED: 'selected',
  MINIMIZED: 'minimized'
} as const;

// @deprecated Use LAYOUT_CONSTANTS from config.ts instead
export const LAYOUT_CONSTANTS = {
  DEFAULT_NODE_WIDTH: 100,
  DEFAULT_NODE_HEIGHT: 40,
  DEFAULT_CONTAINER_PADDING: 20,
  MIN_CONTAINER_WIDTH: 150,
  MIN_CONTAINER_HEIGHT: 100
} as const;

// Legacy type exports for backward compatibility
// @deprecated Use types from types.ts instead
export type NodeStyle = typeof NODE_STYLES[keyof typeof NODE_STYLES];
export type EdgeStyle = typeof EDGE_STYLES[keyof typeof EDGE_STYLES];
export type ContainerStyle = typeof CONTAINER_STYLES[keyof typeof CONTAINER_STYLES];
