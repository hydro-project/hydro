/**
 * @fileoverview Layout configuration exports
 *
 * Re-exports layout configuration from centralized shared config.
 * This maintains backward compatibility while centralizing configuration.
 */
import { LayoutConfig } from './types';
export declare const DEFAULT_LAYOUT_CONFIG: Required<LayoutConfig>;
export declare const LAYOUT_CONFIGS: {
    readonly DEFAULT: Required<LayoutConfig>;
    readonly COMPACT: Required<LayoutConfig>;
    readonly LOOSE: Required<LayoutConfig>;
    readonly HORIZONTAL: Required<LayoutConfig>;
};
/**
 * Get layout configuration by name
 */
export declare function getLayoutConfig(name: keyof typeof LAYOUT_CONFIGS): Required<LayoutConfig>;
/**
 * Create custom layout configuration with defaults
 */
export declare function createLayoutConfig(overrides?: Partial<LayoutConfig>): Required<LayoutConfig>;
//# sourceMappingURL=config.d.ts.map