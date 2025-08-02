/**
 * @fileoverview Layout defaults
 * @deprecated This file is deprecated. Use ../shared/config.ts instead.
 * Re-exports for backward compatibility.
 */

import type { LayoutConfig } from './types';
import { ELK_LAYOUT_CONFIG } from '../shared/config';

// @deprecated Use ELK_LAYOUT_CONFIG.DEFAULT from ../shared/config.ts instead
export const DEFAULT_LAYOUT_CONFIG: Required<LayoutConfig> = ELK_LAYOUT_CONFIG.DEFAULT;
