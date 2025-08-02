/**
 * @fileoverview Layout module exports
 * 
 * Central export point for layout functionality.
 */

export { ELKLayoutEngine } from './ELKLayoutEngine';
export { DEFAULT_LAYOUT_CONFIG } from './config';
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
} from './types.js';
