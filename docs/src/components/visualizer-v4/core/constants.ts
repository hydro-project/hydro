/**
 * @fileoverview Bridge Architecture Constants
 * 
 * Clean constants for our bridge-based implementation.
 * No dependencies on alpha.
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

// Layout dimension constants
export const LAYOUT_CONSTANTS = {
  DEFAULT_NODE_WIDTH: 180,
  DEFAULT_NODE_HEIGHT: 60,
  DEFAULT_CONTAINER_PADDING: 20,
  MIN_CONTAINER_WIDTH: 200,
  MIN_CONTAINER_HEIGHT: 150
} as const;

// Export types
export type NodeStyle = keyof typeof NODE_STYLES;
export type EdgeStyle = keyof typeof EDGE_STYLES;
export type ContainerStyle = keyof typeof CONTAINER_STYLES;
