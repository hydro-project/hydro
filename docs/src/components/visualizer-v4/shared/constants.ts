/**
 * Visualization Design Constants
 * 
 * Professional color system and styling constants for the visualization system.
 * Based on ColorBrewer and WCAG accessibility guidelines.
 */

// Node styling constants
export const NODE_STYLES = {
  DEFAULT: 'default',
  HIGHLIGHTED: 'highlighted',
  SELECTED: 'selected',
  WARNING: 'warning',
  ERROR: 'error'
} as const;

// Edge styling constants
export const EDGE_STYLES = {
  DEFAULT: 'default',
  HIGHLIGHTED: 'highlighted',
  DASHED: 'dashed',
  THICK: 'thick',
  WARNING: 'warning'
} as const;

// Container styling constants
export const CONTAINER_STYLES = {
  DEFAULT: 'default',
  HIGHLIGHTED: 'highlighted',
  SELECTED: 'selected',
  MINIMIZED: 'minimized'
} as const;

// Layout constants
export const LAYOUT_CONSTANTS = {
  DEFAULT_NODE_WIDTH: 100,
  DEFAULT_NODE_HEIGHT: 40,
  DEFAULT_CONTAINER_PADDING: 20,
  MIN_CONTAINER_WIDTH: 150,
  MIN_CONTAINER_HEIGHT: 100
} as const;

// Type exports
export type NodeStyle = typeof NODE_STYLES[keyof typeof NODE_STYLES];
export type EdgeStyle = typeof EDGE_STYLES[keyof typeof EDGE_STYLES];
export type ContainerStyle = typeof CONTAINER_STYLES[keyof typeof CONTAINER_STYLES];
