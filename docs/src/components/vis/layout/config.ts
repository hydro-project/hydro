/**
 * @fileoverview Layout defaults
 */

import type { LayoutConfig } from './types';

export const DEFAULT_LAYOUT_CONFIG: Required<LayoutConfig> = {
  algorithm: 'layered',
  direction: 'DOWN',
  spacing: 50,
  nodeSize: { width: 120, height: 60 }
};
