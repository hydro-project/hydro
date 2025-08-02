/**
 * @fileoverview Layout configuration exports
 * 
 * Re-exports layout configuration from centralized shared config.
 * This maintains backward compatibility while centralizing configuration.
 */

import { LayoutConfig } from './types';
import { ELK_LAYOUT_CONFIG, type ELKLayoutConfig } from '../shared/config';

// Re-export for backward compatibility
export const DEFAULT_LAYOUT_CONFIG: Required<LayoutConfig> = {
  algorithm: ELK_LAYOUT_CONFIG.DEFAULT.algorithm,
  direction: ELK_LAYOUT_CONFIG.DEFAULT.direction,
  spacing: ELK_LAYOUT_CONFIG.DEFAULT.spacing,
  nodeSize: ELK_LAYOUT_CONFIG.DEFAULT.nodeSize,
};

// Export specific layout configurations
export const LAYOUT_CONFIGS = {
  DEFAULT: DEFAULT_LAYOUT_CONFIG,
  COMPACT: {
    algorithm: ELK_LAYOUT_CONFIG.COMPACT.algorithm,
    direction: ELK_LAYOUT_CONFIG.COMPACT.direction,
    spacing: ELK_LAYOUT_CONFIG.COMPACT.spacing,
    nodeSize: ELK_LAYOUT_CONFIG.COMPACT.nodeSize,
  } as Required<LayoutConfig>,
  LOOSE: {
    algorithm: ELK_LAYOUT_CONFIG.LOOSE.algorithm,
    direction: ELK_LAYOUT_CONFIG.LOOSE.direction,
    spacing: ELK_LAYOUT_CONFIG.LOOSE.spacing,
    nodeSize: ELK_LAYOUT_CONFIG.LOOSE.nodeSize,
  } as Required<LayoutConfig>,
  HORIZONTAL: {
    algorithm: ELK_LAYOUT_CONFIG.HORIZONTAL.algorithm,
    direction: ELK_LAYOUT_CONFIG.HORIZONTAL.direction,
    spacing: ELK_LAYOUT_CONFIG.HORIZONTAL.spacing,
    nodeSize: ELK_LAYOUT_CONFIG.HORIZONTAL.nodeSize,
  } as Required<LayoutConfig>,
} as const;

/**
 * Get layout configuration by name
 */
export function getLayoutConfig(name: keyof typeof LAYOUT_CONFIGS): Required<LayoutConfig> {
  return LAYOUT_CONFIGS[name];
}

/**
 * Create custom layout configuration with defaults
 */
export function createLayoutConfig(overrides: Partial<LayoutConfig> = {}): Required<LayoutConfig> {
  return {
    ...DEFAULT_LAYOUT_CONFIG,
    ...overrides,
  };
}
