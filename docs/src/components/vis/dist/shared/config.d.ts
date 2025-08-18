/**
 * @fileoverview Centralized configuration for the graph visualization system
 *
 * Professional color system based on ColorBrewer and WCAG accessibility guidelines.
 * Ported from the existing visualizer app with enhancements for the new vis system.
 */
export declare const COLORS: {
    readonly WHITE: "#ffffff";
    readonly GRAY_50: "#f9fafb";
    readonly GRAY_100: "#f3f4f6";
    readonly GRAY_200: "#e5e7eb";
    readonly GRAY_300: "#d1d5db";
    readonly GRAY_400: "#9ca3af";
    readonly GRAY_500: "#6b7280";
    readonly GRAY_600: "#4b5563";
    readonly GRAY_700: "#374151";
    readonly GRAY_800: "#1f2937";
    readonly GRAY_900: "#111827";
    readonly BLACK: "#000000";
    readonly SUCCESS_50: "#f0f9f0";
    readonly SUCCESS_100: "#dcf2dc";
    readonly SUCCESS_500: "#16a34a";
    readonly SUCCESS_600: "#15803d";
    readonly SUCCESS_700: "#166534";
    readonly WARNING_50: "#fffbeb";
    readonly WARNING_100: "#fef3c7";
    readonly WARNING_500: "#f59e0b";
    readonly WARNING_600: "#d97706";
    readonly WARNING_700: "#b45309";
    readonly ERROR_50: "#fef2f2";
    readonly ERROR_100: "#fee2e2";
    readonly ERROR_500: "#ef4444";
    readonly ERROR_600: "#dc2626";
    readonly ERROR_700: "#b91c1c";
    readonly INFO_50: "#eff6ff";
    readonly INFO_100: "#dbeafe";
    readonly INFO_500: "#3b82f6";
    readonly INFO_600: "#2563eb";
    readonly INFO_700: "#1d4ed8";
    readonly VIZ_TEAL: "#1b9e77";
    readonly VIZ_ORANGE: "#d95f02";
    readonly VIZ_PURPLE: "#7570b3";
    readonly VIZ_PINK: "#e7298a";
    readonly VIZ_GREEN: "#66a61e";
    readonly VIZ_YELLOW: "#e6ab02";
    readonly VIZ_BROWN: "#a6761d";
    readonly VIZ_GRAY: "#666666";
    readonly CONTAINER_L0: "rgba(59, 130, 246, 0.08)";
    readonly CONTAINER_L1: "rgba(16, 185, 129, 0.08)";
    readonly CONTAINER_L2: "rgba(245, 158, 11, 0.08)";
    readonly CONTAINER_L3: "rgba(139, 92, 246, 0.08)";
    readonly CONTAINER_L4: "rgba(239, 68, 68, 0.08)";
    readonly CONTAINER_BORDER_L0: "#3b82f6";
    readonly CONTAINER_BORDER_L1: "#10b981";
    readonly CONTAINER_BORDER_L2: "#f59e0b";
    readonly CONTAINER_BORDER_L3: "#8b5cf6";
    readonly CONTAINER_BORDER_L4: "#ef4444";
    readonly PRIMARY: "#3b82f6";
    readonly PRIMARY_HOVER: "#2563eb";
    readonly PRIMARY_LIGHT: "rgba(59, 130, 246, 0.1)";
};
export declare const COLOR_PALETTES: {
    readonly Set2: readonly [{
        readonly primary: "#1b9e77";
        readonly secondary: "#a6cee3";
        readonly name: "Teal";
    }, {
        readonly primary: "#d95f02";
        readonly secondary: "#1f78b4";
        readonly name: "Orange";
    }, {
        readonly primary: "#7570b3";
        readonly secondary: "#b2df8a";
        readonly name: "Purple";
    }, {
        readonly primary: "#e7298a";
        readonly secondary: "#33a02c";
        readonly name: "Pink";
    }, {
        readonly primary: "#66a61e";
        readonly secondary: "#fb9a99";
        readonly name: "Green";
    }, {
        readonly primary: "#e6ab02";
        readonly secondary: "#e31a1c";
        readonly name: "Yellow";
    }, {
        readonly primary: "#a6761d";
        readonly secondary: "#fdbf6f";
        readonly name: "Brown";
    }, {
        readonly primary: "#666666";
        readonly secondary: "#ff7f00";
        readonly name: "Gray";
    }];
    readonly Set3: readonly [{
        readonly primary: "#8dd3c7";
        readonly secondary: "#ffffb3";
        readonly name: "Light Teal";
    }, {
        readonly primary: "#bebada";
        readonly secondary: "#fb8072";
        readonly name: "Light Purple";
    }, {
        readonly primary: "#80b1d3";
        readonly secondary: "#fdb462";
        readonly name: "Light Blue";
    }, {
        readonly primary: "#fccde5";
        readonly secondary: "#b3de69";
        readonly name: "Light Pink";
    }, {
        readonly primary: "#d9d9d9";
        readonly secondary: "#fccde5";
        readonly name: "Light Gray";
    }, {
        readonly primary: "#bc80bd";
        readonly secondary: "#ccebc5";
        readonly name: "Medium Purple";
    }, {
        readonly primary: "#ccebc5";
        readonly secondary: "#ffed6f";
        readonly name: "Light Green";
    }, {
        readonly primary: "#ffed6f";
        readonly secondary: "#8dd3c7";
        readonly name: "Light Yellow";
    }];
    readonly Professional: readonly [{
        readonly primary: "#1e40af";
        readonly secondary: "#93c5fd";
        readonly name: "Corporate Blue";
    }, {
        readonly primary: "#059669";
        readonly secondary: "#86efac";
        readonly name: "Success Green";
    }, {
        readonly primary: "#dc2626";
        readonly secondary: "#fca5a5";
        readonly name: "Alert Red";
    }, {
        readonly primary: "#7c2d12";
        readonly secondary: "#fdba74";
        readonly name: "Warm Brown";
    }, {
        readonly primary: "#4338ca";
        readonly secondary: "#c4b5fd";
        readonly name: "Deep Purple";
    }, {
        readonly primary: "#0891b2";
        readonly secondary: "#67e8f9";
        readonly name: "Ocean Blue";
    }];
};
export declare const COMPONENT_COLORS: {
    readonly EDGE_DEFAULT: "#9ca3af";
    readonly EDGE_HOVER: "#4b5563";
    readonly EDGE_SELECTED: "#3b82f6";
    readonly EDGE_NETWORK: "#7570b3";
    readonly HANDLE_DEFAULT: "#6b7280";
    readonly HANDLE_HOVER: "#374151";
    readonly HANDLE_ACTIVE: "#3b82f6";
    readonly BACKGROUND_PRIMARY: "#ffffff";
    readonly BACKGROUND_SECONDARY: "#f9fafb";
    readonly BACKGROUND_TERTIARY: "#f3f4f6";
    readonly BORDER_LIGHT: "#e5e7eb";
    readonly BORDER_MEDIUM: "#d1d5db";
    readonly BORDER_STRONG: "#9ca3af";
    readonly TEXT_PRIMARY: "#111827";
    readonly TEXT_SECONDARY: "#4b5563";
    readonly TEXT_TERTIARY: "#6b7280";
    readonly TEXT_DISABLED: "#9ca3af";
    readonly TEXT_INVERSE: "#ffffff";
    readonly INTERACTIVE_DEFAULT: "#3b82f6";
    readonly INTERACTIVE_HOVER: "#2563eb";
    readonly INTERACTIVE_ACTIVE: "#1d4ed8";
    readonly INTERACTIVE_DISABLED: "#d1d5db";
    readonly STATUS_SUCCESS: "#16a34a";
    readonly STATUS_WARNING: "#f59e0b";
    readonly STATUS_ERROR: "#ef4444";
    readonly STATUS_INFO: "#3b82f6";
    readonly PANEL_BACKGROUND: "#ffffff";
    readonly PANEL_HEADER_BACKGROUND: "#f9fafb";
    readonly BUTTON_HOVER_BACKGROUND: "#f3f4f6";
};
export declare const NODE_COLORS: {
    readonly BACKGROUND: {
        readonly DEFAULT: "#ffffff";
        readonly HIGHLIGHTED: "#fef3c7";
        readonly SELECTED: "#dbeafe";
        readonly WARNING: "#fef3c7";
        readonly ERROR: "#fee2e2";
    };
    readonly BORDER: {
        readonly DEFAULT: "#6b7280";
        readonly HIGHLIGHTED: "#f59e0b";
        readonly SELECTED: "#3b82f6";
        readonly WARNING: "#d97706";
        readonly ERROR: "#dc2626";
    };
    readonly TEXT: {
        readonly DEFAULT: "#111827";
        readonly HIGHLIGHTED: "#b45309";
        readonly SELECTED: "#1d4ed8";
        readonly WARNING: "#b45309";
        readonly ERROR: "#b91c1c";
    };
    readonly HANDLE: "#6b7280";
};
export declare const EDGE_COLORS: {
    readonly DEFAULT: "#9ca3af";
    readonly DATA: "#16a34a";
    readonly CONTROL: "#f59e0b";
    readonly ERROR: "#ef4444";
    readonly THICK: "#374151";
    readonly DASHED: "#6b7280";
    readonly SELECTED: "#3b82f6";
    readonly HIGHLIGHTED: "#ff6b6b";
    readonly NETWORK: "#7570b3";
};
export declare const CONTAINER_COLORS: {
    readonly BACKGROUND: "rgba(59, 130, 246, 0.08)";
    readonly BORDER: "#e5e7eb";
    readonly BORDER_SELECTED: "#3b82f6";
    readonly HEADER_BACKGROUND: "rgba(100, 116, 139, 0.1)";
    readonly HEADER_TEXT: "#374151";
};
export declare const PANEL_COLORS: {
    readonly BACKGROUND: "rgba(59, 130, 246, 0.1)";
    readonly BORDER: "#3b82f6";
    readonly TEXT: "#3b82f6";
};
export declare const SIZES: {
    readonly NODE_MIN_WIDTH: 120;
    readonly NODE_MIN_HEIGHT: 40;
    readonly NODE_PADDING: 12;
    readonly NODE_BORDER_RADIUS: 6;
    readonly EDGE_WIDTH_DEFAULT: 1;
    readonly EDGE_WIDTH_THICK: 3;
    readonly BORDER_WIDTH_DEFAULT: 2;
    readonly BORDER_WIDTH_SELECTED: 2;
    readonly BORDER_RADIUS_DEFAULT: 6;
    readonly CONTAINER_MIN_WIDTH: 200;
    readonly CONTAINER_MIN_HEIGHT: 100;
    readonly CONTAINER_PADDING: 16;
    readonly CONTAINER_BORDER_RADIUS: 8;
    readonly CONTAINER_HEADER_HEIGHT: 30;
    readonly MINIMAP_NODE_BORDER_RADIUS: 4;
    readonly GRID_SIZE: 15;
    readonly SPACING_XS: 4;
    readonly SPACING_SM: 8;
    readonly SPACING_MD: 12;
    readonly SPACING_LG: 16;
    readonly SPACING_XL: 20;
    readonly SPACING_XXL: 24;
};
export declare const SHADOWS: {
    readonly NODE_DEFAULT: "0 2px 4px rgba(0, 0, 0, 0.1)";
    readonly NODE_SELECTED: "0 0 10px rgba(59, 130, 246, 0.5)";
    readonly NODE_HOVER: "0 4px 8px rgba(0, 0, 0, 0.25)";
    readonly CONTAINER_DEFAULT: "0 1px 3px rgba(0, 0, 0, 0.1)";
    readonly CONTAINER_SELECTED: "0 0 0 2px #3b82f6";
    readonly PANEL: "0 2px 8px rgba(0, 0, 0, 0.1)";
    readonly PANEL_DEFAULT: "0 2px 8px rgba(0, 0, 0, 0.1)";
    readonly PANEL_DRAGGING: "0 8px 25px rgba(0, 0, 0, 0.25)";
};
export declare const DEFAULT_STYLES: {
    readonly BORDER_WIDTH: "1px";
    readonly BORDER_WIDTH_THICK: "2px";
    readonly BORDER_RADIUS_SM: "4px";
    readonly BORDER_RADIUS: "6px";
    readonly BORDER_RADIUS_LG: "8px";
    readonly BOX_SHADOW_SM: "0 1px 2px 0 rgba(0, 0, 0, 0.05)";
    readonly BOX_SHADOW: "0 1px 3px 0 rgba(0, 0, 0, 0.1), 0 1px 2px 0 rgba(0, 0, 0, 0.06)";
    readonly BOX_SHADOW_LG: "0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)";
};
export declare const TYPOGRAPHY: {
    readonly FONT_FAMILY: "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, sans-serif";
    readonly FONT_SIZES: {
        readonly XS: "10px";
        readonly SM: "12px";
        readonly MD: "14px";
        readonly LG: "16px";
        readonly XL: "18px";
        readonly XXL: "20px";
    };
    readonly FONT_WEIGHTS: {
        readonly NORMAL: 400;
        readonly MEDIUM: 500;
        readonly SEMIBOLD: 600;
        readonly BOLD: 700;
    };
    readonly LINE_HEIGHTS: {
        readonly TIGHT: 1.2;
        readonly NORMAL: 1.4;
        readonly RELAXED: 1.6;
    };
};
export declare const ANIMATIONS: {
    readonly DURATION_FAST: "150ms";
    readonly DURATION_NORMAL: "200ms";
    readonly DURATION_SLOW: "300ms";
    readonly EASING_DEFAULT: "ease";
    readonly EASING_IN: "ease-in";
    readonly EASING_OUT: "ease-out";
    readonly EASING_IN_OUT: "ease-in-out";
    readonly TRANSITION_DEFAULT: "all 200ms ease";
    readonly TRANSITION_FAST: "all 150ms ease";
    readonly FIT_VIEW_DURATION: 300;
    readonly FIT_VIEW_DEBOUNCE: 100;
    readonly LAYOUT_DEBOUNCE: 200;
    readonly RESIZE_DEBOUNCE: 500;
};
export declare const LAYOUT_SPACING: {
    readonly NODE_TO_NODE_COMPACT: 15;
    readonly NODE_TO_NODE_NORMAL: 75;
    readonly NODE_TO_NODE_LOOSE: 125;
    readonly EDGE_TO_NODE: 0;
    readonly EDGE_TO_EDGE: 10;
    readonly EDGE_TO_EDGE_ALTERNATE: 15;
    readonly LAYER_SEPARATION: 25;
    readonly COMPONENT_TO_COMPONENT: 60;
    readonly CONTAINER_PADDING: 60;
    readonly ROOT_PADDING: 20;
    readonly BORDER_TO_NODE: 20;
};
export declare const ZOOM_LEVELS: {
    readonly MIN_INTERACTIVE: 0.2;
    readonly MAX_INTERACTIVE: 2;
    readonly MIN_FIT_VIEW: 0.1;
    readonly MAX_FIT_VIEW: 1.5;
    readonly DEFAULT: 0.5;
};
export declare const MINIMAP_CONFIG: {
    readonly NODE_STROKE_COLOR: "#374151";
    readonly NODE_COLOR: "#e5e7eb";
    readonly NODE_BORDER_RADIUS: 4;
};
export declare const DASH_PATTERNS: {
    readonly SOLID: any;
    readonly DASHED: "5,5";
    readonly DOTTED: "2,2";
    readonly DASH_DOT: "8,4,2,4";
};
export declare const Z_INDEX: {
    readonly BACKGROUND: 0;
    readonly EDGES: 1;
    readonly NODES: 2;
    readonly CONTAINERS: 3;
    readonly HANDLES: 4;
    readonly CONTROLS: 5;
    readonly PANELS: 6;
    readonly MODALS: 7;
    readonly TOOLTIPS: 8;
};
export declare const BREAKPOINTS: {
    readonly SM: 640;
    readonly MD: 768;
    readonly LG: 1024;
    readonly XL: 1280;
};
/**
 * ELK.js layout algorithms and their configurations
 * Consolidated from layout/config.ts for centralized configuration
 */
export declare const ELK_ALGORITHMS: {
    readonly LAYERED: "layered";
    readonly STRESS: "stress";
    readonly MRTREE: "mrtree";
    readonly RADIAL: "radial";
    readonly FORCE: "force";
};
export declare const ELK_DIRECTIONS: {
    readonly DOWN: "DOWN";
    readonly UP: "UP";
    readonly LEFT: "LEFT";
    readonly RIGHT: "RIGHT";
};
/**
 * Default ELK layout configuration
 * Ported from layout/config.ts with enhanced spacing based on LAYOUT_SPACING
 */
export declare const ELK_LAYOUT_CONFIG: {
    readonly DEFAULT: {
        readonly algorithm: "layered";
        readonly direction: "DOWN";
        readonly spacing: 75;
        readonly nodeSize: {
            readonly width: 120;
            readonly height: 40;
        };
    };
    readonly COMPACT: {
        readonly algorithm: "layered";
        readonly direction: "DOWN";
        readonly spacing: 15;
        readonly nodeSize: {
            readonly width: 120;
            readonly height: 40;
        };
    };
    readonly LOOSE: {
        readonly algorithm: "layered";
        readonly direction: "DOWN";
        readonly spacing: 125;
        readonly nodeSize: {
            readonly width: 120;
            readonly height: 40;
        };
    };
    readonly FORCE_DIRECTED: {
        readonly algorithm: "force";
        readonly direction: "DOWN";
        readonly spacing: 75;
        readonly nodeSize: {
            readonly width: 120;
            readonly height: 40;
        };
    };
    readonly HORIZONTAL: {
        readonly algorithm: "layered";
        readonly direction: "RIGHT";
        readonly spacing: 75;
        readonly nodeSize: {
            readonly width: 120;
            readonly height: 40;
        };
    };
};
/**
 * ELK-specific layout options for fine-tuning
 * Maps to elkjs layout options for advanced configuration
 */
export declare const ELK_LAYOUT_OPTIONS: {
    readonly LAYERED: {
        readonly 'elk.layered.spacing.nodeNodeBetweenLayers': 25;
        readonly 'elk.layered.nodePlacement.strategy': "SIMPLE";
        readonly 'elk.layered.crossingMinimization.strategy': "LAYER_SWEEP";
        readonly 'elk.layered.layering.strategy': "LONGEST_PATH";
    };
    readonly FORCE: {
        readonly 'elk.force.repulsivePower': 200;
        readonly 'elk.force.iterations': 300;
        readonly 'elk.force.temperature': 0.001;
    };
    readonly STRESS: {
        readonly 'elk.stress.iterations': 300;
        readonly 'elk.stress.epsilon': 0.0001;
    };
    readonly SPACING: {
        readonly 'elk.spacing.nodeNode': 75;
        readonly 'elk.spacing.edgeNode': 0;
        readonly 'elk.spacing.edgeEdge': 10;
        readonly 'elk.spacing.componentComponent': 60;
    };
    readonly PADDING: {
        readonly 'elk.padding.left': 20;
        readonly 'elk.padding.right': 20;
        readonly 'elk.padding.top': 20;
        readonly 'elk.padding.bottom': 20;
    };
};
export type ELKAlgorithm = typeof ELK_ALGORITHMS[keyof typeof ELK_ALGORITHMS];
export type ELKDirection = typeof ELK_DIRECTIONS[keyof typeof ELK_DIRECTIONS];
export type ELKLayoutConfigKey = keyof typeof ELK_LAYOUT_CONFIG;
/**
 * ELK Layout Configuration Interface
 * Matches the LayoutConfig from layout/types.ts for consistency
 */
export interface ELKLayoutConfig {
    algorithm?: ELKAlgorithm;
    direction?: ELKDirection;
    spacing?: number;
    nodeSize?: {
        width: number;
        height: number;
    };
}
export declare const DEFAULT_NODE_STYLE: {
    readonly borderRadius: "6px";
    readonly padding: "12px";
    readonly color: "#ffffff";
    readonly fontFamily: "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, sans-serif";
    readonly fontSize: "14px";
    readonly fontWeight: 500;
    readonly border: "none";
    readonly boxShadow: "0 2px 4px rgba(0, 0, 0, 0.1)";
    readonly transition: "all 200ms ease";
    readonly display: "flex";
    readonly alignItems: "center";
    readonly justifyContent: "center";
    readonly textAlign: "center";
    readonly width: 200;
    readonly height: 60;
};
export declare const DEFAULT_EDGE_STYLE: {
    readonly strokeWidth: 1;
    readonly stroke: "#9ca3af";
    readonly strokeDasharray: any;
};
export declare const DEFAULT_CONTAINER_STYLE: {
    readonly backgroundColor: "rgba(59, 130, 246, 0.08)";
    readonly border: "2px solid #e5e7eb";
    readonly borderRadius: "8px";
    readonly padding: "16px";
    readonly boxShadow: "0 1px 3px rgba(0, 0, 0, 0.1)";
};
/**
 * Get appropriate text color for given background
 * Ensures WCAG AA compliance (4.5:1 contrast ratio)
 */
export declare function getAccessibleTextColor(backgroundColor: string): string;
/**
 * Get semantic color for status/feedback
 */
export declare function getStatusColor(status: 'success' | 'warning' | 'error' | 'info', variant?: '50' | '100' | '500' | '600' | '700'): string;
/**
 * Get node border color based on style and state
 */
export declare function getNodeBorderColor(style: string, selected?: boolean, highlighted?: boolean): string;
/**
 * Get node text color based on style
 */
export declare function getNodeTextColor(style: string): string;
/**
 * Get edge color based on style and state
 */
export declare function getEdgeColor(style?: string, selected?: boolean, highlighted?: boolean): string;
/**
 * Get edge stroke width based on style
 */
export declare function getEdgeStrokeWidth(style?: string): number;
/**
 * Get edge dash pattern based on style
 */
export declare function getEdgeDashPattern(style?: string): string | undefined;
/**
 * Get ELK layout configuration by name or return custom config
 */
export declare function getELKLayoutConfig(configKey?: ELKLayoutConfigKey | ELKLayoutConfig): ELKLayoutConfig;
/**
 * Get ELK layout options for specific algorithm with enhanced spacing
 */
export declare function getELKLayoutOptions(algorithm: ELKAlgorithm): Record<string, any>;
//# sourceMappingURL=config.d.ts.map