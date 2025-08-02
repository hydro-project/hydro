/**
 * @fileoverview Layout module exports
 * 
 * Central export point for layout functionality with proper TypeScript support.
 */

// Core layout engine
export { ELKLayoutEngine } from './ELKLayoutEngine.js';

// State management
export { createELKStateManager } from './ELKStateManager.js';
export type { ELKStateManager, LayoutDimensions, LayoutPosition } from './ELKStateManager.js';

// Configuration
export { DEFAULT_LAYOUT_CONFIG, LAYOUT_CONFIGS, getLayoutConfig, createLayoutConfig } from './config.js';

// Types
export type {
  LayoutConfig,
  LayoutResult,
  LayoutEngine,
  AdvancedLayoutEngine,
  LayoutEngineOptions,
  LayoutValidationResult,
  LayoutValidationError,
  LayoutValidationWarning,
  LayoutStatistics,
  LayoutEventData,
  LayoutEventCallback,
  PositionedNode,
  PositionedEdge,
  PositionedContainer,
  PositionedHyperEdge,
} from './types.js';

// Re-export shared config types for convenience
export type { ELKAlgorithm, ELKDirection } from '../shared/config.js';
