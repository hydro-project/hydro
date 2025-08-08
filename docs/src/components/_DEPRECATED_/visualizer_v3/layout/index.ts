/**
 * @fileoverview Layout module exports - Bridge Architecture Implementation
 * 
 * Clean layout module using our bridge architecture.
 * No dependencies on alpha.
 */

// Core layout engine - bridge-based!
export { ELKLayoutEngine, DEFAULT_LAYOUT_CONFIG, createELKStateManager } from './ELKLayoutEngine';

// Our own types
export type {
  LayoutConfig,
  LayoutResult,
  LayoutEngine,
  LayoutStatistics,
  LayoutEventData,
  LayoutEventCallback,
  PositionedNode,
  PositionedEdge,
  PositionedContainer,
  PositionedHyperEdge,
  Position
} from '../core/types';

// Import types for local use
import type { LayoutConfig } from '../core/types';
import { DEFAULT_LAYOUT_CONFIG } from './ELKLayoutEngine';

// Configuration helpers
export function getLayoutConfig(name: string): LayoutConfig {
  console.log(`[Layout] Getting config: ${name}`);
  return DEFAULT_LAYOUT_CONFIG;
}

export function createLayoutConfig(overrides: Partial<LayoutConfig>): LayoutConfig {
  return { ...DEFAULT_LAYOUT_CONFIG, ...overrides };
}

// Pre-defined configurations for compatibility
export const LAYOUT_CONFIGS = {
  default: DEFAULT_LAYOUT_CONFIG,
  compact: { ...DEFAULT_LAYOUT_CONFIG, spacing: 50 },
  spacious: { ...DEFAULT_LAYOUT_CONFIG, spacing: 150 }
};

// State manager type for compatibility
export interface ELKStateManager {
  updatePositions(): void;
  dispose(): void;
}
