/**
 * Get appropriate text color for given background
 * Ensures WCAG AA compliance (4.5:1 contrast ratio)
 */
export function getAccessibleTextColor(backgroundColor: any): string;
/**
 * Get semantic color for status/feedback
 */
export function getStatusColor(status: any, variant?: string): any;
/**
 * Get node border color based on style and state
 */
export function getNodeBorderColor(style: any, selected: any, highlighted: any): string;
/**
 * Get node text color based on style
 */
export function getNodeTextColor(style: any): string;
/**
 * Get edge color based on style and state
 */
export function getEdgeColor(style: any, selected: any, highlighted: any): string;
/**
 * Get edge stroke width based on style
 */
export function getEdgeStrokeWidth(style: any): number;
/**
 * Get edge dash pattern based on style
 */
export function getEdgeDashPattern(style: any): any;
/**
 * Get ELK layout configuration by name or return custom config
 */
export function getELKLayoutConfig(configKey: any): any;
/**
 * Get ELK layout options for specific algorithm with enhanced spacing
 */
export function getELKLayoutOptions(algorithm: any): {
    'elk.padding.left': number;
    'elk.padding.right': number;
    'elk.padding.top': number;
    'elk.padding.bottom': number;
    'elk.spacing.nodeNode': number;
    'elk.spacing.edgeNode': number;
    'elk.spacing.edgeEdge': number;
    'elk.spacing.componentComponent': number;
};
/**
 * Create ELK layout options for fixed positioning (unchanged containers)
 * @param x - X position to fix
 * @param y - Y position to fix
 * @returns ELK layout options for fixed positioning
 */
export function createFixedPositionOptions(x: any, y: any): {
    'elk.position.x': any;
    'elk.position.y': any;
    'elk.nodeSize.constraints': string;
    'elk.nodeSize.options': string;
};
/**
 * Create ELK layout options for free positioning (containers that can move)
 * @returns ELK layout options for free positioning
 */
export function createFreePositionOptions(): {
    'elk.nodeSize.constraints': string;
    'elk.nodeSize.options': string;
};
export namespace COLORS {
    let WHITE: string;
    let GRAY_50: string;
    let GRAY_100: string;
    let GRAY_200: string;
    let GRAY_300: string;
    let GRAY_400: string;
    let GRAY_500: string;
    let GRAY_600: string;
    let GRAY_700: string;
    let GRAY_800: string;
    let GRAY_900: string;
    let BLACK: string;
    let SUCCESS_50: string;
    let SUCCESS_100: string;
    let SUCCESS_500: string;
    let SUCCESS_600: string;
    let SUCCESS_700: string;
    let WARNING_50: string;
    let WARNING_100: string;
    let WARNING_500: string;
    let WARNING_600: string;
    let WARNING_700: string;
    let ERROR_50: string;
    let ERROR_100: string;
    let ERROR_500: string;
    let ERROR_600: string;
    let ERROR_700: string;
    let INFO_50: string;
    let INFO_100: string;
    let INFO_500: string;
    let INFO_600: string;
    let INFO_700: string;
    let VIZ_TEAL: string;
    let VIZ_ORANGE: string;
    let VIZ_PURPLE: string;
    let VIZ_PINK: string;
    let VIZ_GREEN: string;
    let VIZ_YELLOW: string;
    let VIZ_BROWN: string;
    let VIZ_GRAY: string;
    let CONTAINER_L0: string;
    let CONTAINER_L1: string;
    let CONTAINER_L2: string;
    let CONTAINER_L3: string;
    let CONTAINER_L4: string;
    let CONTAINER_BORDER_L0: string;
    let CONTAINER_BORDER_L1: string;
    let CONTAINER_BORDER_L2: string;
    let CONTAINER_BORDER_L3: string;
    let CONTAINER_BORDER_L4: string;
    let PRIMARY: string;
    let PRIMARY_HOVER: string;
    let PRIMARY_LIGHT: string;
}
export namespace COLOR_PALETTES {
    let Set2: {
        primary: string;
        secondary: string;
        name: string;
    }[];
    let Set3: {
        primary: string;
        secondary: string;
        name: string;
    }[];
    let Professional: {
        primary: string;
        secondary: string;
        name: string;
    }[];
}
export namespace COMPONENT_COLORS {
    import EDGE_DEFAULT = COLORS.GRAY_400;
    export { EDGE_DEFAULT };
    import EDGE_HOVER = COLORS.GRAY_600;
    export { EDGE_HOVER };
    import EDGE_SELECTED = COLORS.INFO_500;
    export { EDGE_SELECTED };
    import EDGE_NETWORK = COLORS.VIZ_PURPLE;
    export { EDGE_NETWORK };
    import HANDLE_DEFAULT = COLORS.GRAY_500;
    export { HANDLE_DEFAULT };
    import HANDLE_HOVER = COLORS.GRAY_700;
    export { HANDLE_HOVER };
    import HANDLE_ACTIVE = COLORS.INFO_500;
    export { HANDLE_ACTIVE };
    import BACKGROUND_PRIMARY = COLORS.WHITE;
    export { BACKGROUND_PRIMARY };
    import BACKGROUND_SECONDARY = COLORS.GRAY_50;
    export { BACKGROUND_SECONDARY };
    import BACKGROUND_TERTIARY = COLORS.GRAY_100;
    export { BACKGROUND_TERTIARY };
    import BORDER_LIGHT = COLORS.GRAY_200;
    export { BORDER_LIGHT };
    import BORDER_MEDIUM = COLORS.GRAY_300;
    export { BORDER_MEDIUM };
    import BORDER_STRONG = COLORS.GRAY_400;
    export { BORDER_STRONG };
    import TEXT_PRIMARY = COLORS.GRAY_900;
    export { TEXT_PRIMARY };
    import TEXT_SECONDARY = COLORS.GRAY_600;
    export { TEXT_SECONDARY };
    import TEXT_TERTIARY = COLORS.GRAY_500;
    export { TEXT_TERTIARY };
    import TEXT_DISABLED = COLORS.GRAY_400;
    export { TEXT_DISABLED };
    import TEXT_INVERSE = COLORS.WHITE;
    export { TEXT_INVERSE };
    import INTERACTIVE_DEFAULT = COLORS.INFO_500;
    export { INTERACTIVE_DEFAULT };
    import INTERACTIVE_HOVER = COLORS.INFO_600;
    export { INTERACTIVE_HOVER };
    import INTERACTIVE_ACTIVE = COLORS.INFO_700;
    export { INTERACTIVE_ACTIVE };
    import INTERACTIVE_DISABLED = COLORS.GRAY_300;
    export { INTERACTIVE_DISABLED };
    import STATUS_SUCCESS = COLORS.SUCCESS_500;
    export { STATUS_SUCCESS };
    import STATUS_WARNING = COLORS.WARNING_500;
    export { STATUS_WARNING };
    import STATUS_ERROR = COLORS.ERROR_500;
    export { STATUS_ERROR };
    import STATUS_INFO = COLORS.INFO_500;
    export { STATUS_INFO };
    import PANEL_BACKGROUND = COLORS.WHITE;
    export { PANEL_BACKGROUND };
    import PANEL_HEADER_BACKGROUND = COLORS.GRAY_50;
    export { PANEL_HEADER_BACKGROUND };
    import BUTTON_HOVER_BACKGROUND = COLORS.GRAY_100;
    export { BUTTON_HOVER_BACKGROUND };
}
export namespace NODE_COLORS {
    export namespace BACKGROUND {
        import DEFAULT = COLORS.WHITE;
        export { DEFAULT };
        import HIGHLIGHTED = COLORS.WARNING_100;
        export { HIGHLIGHTED };
        import SELECTED = COLORS.INFO_100;
        export { SELECTED };
        import WARNING = COLORS.WARNING_100;
        export { WARNING };
        import ERROR = COLORS.ERROR_100;
        export { ERROR };
    }
    export namespace BORDER {
        import DEFAULT_1 = COLORS.GRAY_500;
        export { DEFAULT_1 as DEFAULT };
        import HIGHLIGHTED_1 = COLORS.WARNING_500;
        export { HIGHLIGHTED_1 as HIGHLIGHTED };
        import SELECTED_1 = COLORS.INFO_500;
        export { SELECTED_1 as SELECTED };
        import WARNING_1 = COLORS.WARNING_600;
        export { WARNING_1 as WARNING };
        import ERROR_1 = COLORS.ERROR_600;
        export { ERROR_1 as ERROR };
    }
    export namespace TEXT {
        import DEFAULT_2 = COMPONENT_COLORS.TEXT_PRIMARY;
        export { DEFAULT_2 as DEFAULT };
        import HIGHLIGHTED_2 = COLORS.WARNING_700;
        export { HIGHLIGHTED_2 as HIGHLIGHTED };
        import SELECTED_2 = COLORS.INFO_700;
        export { SELECTED_2 as SELECTED };
        import WARNING_2 = COLORS.WARNING_700;
        export { WARNING_2 as WARNING };
        import ERROR_2 = COLORS.ERROR_700;
        export { ERROR_2 as ERROR };
    }
    import HANDLE = COMPONENT_COLORS.HANDLE_DEFAULT;
    export { HANDLE };
}
export namespace EDGE_COLORS {
    import DEFAULT_3 = COMPONENT_COLORS.EDGE_DEFAULT;
    export { DEFAULT_3 as DEFAULT };
    import DATA = COLORS.SUCCESS_500;
    export { DATA };
    import CONTROL = COLORS.WARNING_500;
    export { CONTROL };
    import ERROR_3 = COLORS.ERROR_500;
    export { ERROR_3 as ERROR };
    import THICK = COLORS.GRAY_700;
    export { THICK };
    import DASHED = COLORS.GRAY_500;
    export { DASHED };
    import SELECTED_3 = COMPONENT_COLORS.EDGE_SELECTED;
    export { SELECTED_3 as SELECTED };
    let HIGHLIGHTED_3: string;
    export { HIGHLIGHTED_3 as HIGHLIGHTED };
    import NETWORK = COMPONENT_COLORS.EDGE_NETWORK;
    export { NETWORK };
}
export namespace CONTAINER_COLORS {
    import BACKGROUND_1 = COLORS.CONTAINER_L0;
    export { BACKGROUND_1 as BACKGROUND };
    import BORDER_1 = COMPONENT_COLORS.BORDER_LIGHT;
    export { BORDER_1 as BORDER };
    import BORDER_SELECTED = COLORS.INFO_500;
    export { BORDER_SELECTED };
    export let HEADER_BACKGROUND: string;
    import HEADER_TEXT = COLORS.GRAY_700;
    export { HEADER_TEXT };
}
export namespace PANEL_COLORS {
    import BACKGROUND_2 = COLORS.PRIMARY_LIGHT;
    export { BACKGROUND_2 as BACKGROUND };
    import BORDER_2 = COLORS.PRIMARY;
    export { BORDER_2 as BORDER };
    import TEXT_1 = COLORS.PRIMARY;
    export { TEXT_1 as TEXT };
}
export namespace SIZES {
    let NODE_MIN_WIDTH: number;
    let NODE_MIN_HEIGHT: number;
    let NODE_PADDING: number;
    let NODE_BORDER_RADIUS: number;
    let EDGE_WIDTH_DEFAULT: number;
    let EDGE_WIDTH_THICK: number;
    let BORDER_WIDTH_DEFAULT: number;
    let BORDER_WIDTH_SELECTED: number;
    let BORDER_RADIUS_DEFAULT: number;
    let CONTAINER_MIN_WIDTH: number;
    let CONTAINER_MIN_HEIGHT: number;
    let CONTAINER_PADDING: number;
    let CONTAINER_BORDER_RADIUS: number;
    let CONTAINER_HEADER_HEIGHT: number;
    let CONTAINER_TITLE_AREA_PADDING: number;
    let COLLAPSED_CONTAINER_WIDTH: number;
    let COLLAPSED_CONTAINER_HEIGHT: number;
    let MINIMAP_NODE_BORDER_RADIUS: number;
    let GRID_SIZE: number;
    let SPACING_XS: number;
    let SPACING_SM: number;
    let SPACING_MD: number;
    let SPACING_LG: number;
    let SPACING_XL: number;
    let SPACING_XXL: number;
}
export namespace SHADOWS {
    let NODE_DEFAULT: string;
    let NODE_SELECTED: string;
    let NODE_HOVER: string;
    let CONTAINER_DEFAULT: string;
    let CONTAINER_SELECTED: string;
    let PANEL: string;
    let PANEL_DEFAULT: string;
    let PANEL_DRAGGING: string;
}
export namespace DEFAULT_STYLES {
    let BORDER_WIDTH: string;
    let BORDER_WIDTH_THICK: string;
    let BORDER_RADIUS_SM: string;
    let BORDER_RADIUS: string;
    let BORDER_RADIUS_LG: string;
    let BOX_SHADOW_SM: string;
    let BOX_SHADOW: string;
    let BOX_SHADOW_LG: string;
}
export namespace TYPOGRAPHY {
    let FONT_FAMILY: string;
    namespace FONT_SIZES {
        let XS: string;
        let SM: string;
        let MD: string;
        let LG: string;
        let XL: string;
        let XXL: string;
    }
    namespace FONT_WEIGHTS {
        let NORMAL: number;
        let MEDIUM: number;
        let SEMIBOLD: number;
        let BOLD: number;
    }
    namespace LINE_HEIGHTS {
        export let TIGHT: number;
        let NORMAL_1: number;
        export { NORMAL_1 as NORMAL };
        export let RELAXED: number;
    }
}
export namespace ANIMATIONS {
    let DURATION_FAST: string;
    let DURATION_NORMAL: string;
    let DURATION_SLOW: string;
    let EASING_DEFAULT: string;
    let EASING_IN: string;
    let EASING_OUT: string;
    let EASING_IN_OUT: string;
    let TRANSITION_DEFAULT: string;
    let TRANSITION_FAST: string;
    let FIT_VIEW_DURATION: number;
    let FIT_VIEW_DEBOUNCE: number;
    let LAYOUT_DEBOUNCE: number;
    let RESIZE_DEBOUNCE: number;
}
export namespace LAYOUT_SPACING {
    export let NODE_TO_NODE_COMPACT: number;
    export let NODE_TO_NODE_NORMAL: number;
    export let NODE_TO_NODE_LOOSE: number;
    export let EDGE_TO_NODE: number;
    export let EDGE_TO_EDGE: number;
    export let EDGE_TO_EDGE_ALTERNATE: number;
    export let LAYER_SEPARATION: number;
    export let COMPONENT_TO_COMPONENT: number;
    let CONTAINER_PADDING_1: number;
    export { CONTAINER_PADDING_1 as CONTAINER_PADDING };
    export let ROOT_PADDING: number;
    export let BORDER_TO_NODE: number;
}
export namespace ZOOM_LEVELS {
    export let MIN_INTERACTIVE: number;
    export let MAX_INTERACTIVE: number;
    export let MIN_FIT_VIEW: number;
    export let MAX_FIT_VIEW: number;
    let DEFAULT_4: number;
    export { DEFAULT_4 as DEFAULT };
}
export namespace MINIMAP_CONFIG {
    import NODE_STROKE_COLOR = COLORS.GRAY_700;
    export { NODE_STROKE_COLOR };
    import NODE_COLOR = COLORS.GRAY_200;
    export { NODE_COLOR };
    import NODE_BORDER_RADIUS_1 = SIZES.MINIMAP_NODE_BORDER_RADIUS;
    export { NODE_BORDER_RADIUS_1 as NODE_BORDER_RADIUS };
}
export namespace DASH_PATTERNS {
    export let SOLID: any;
    let DASHED_1: string;
    export { DASHED_1 as DASHED };
    export let DOTTED: string;
    export let DASH_DOT: string;
}
export namespace Z_INDEX {
    let BACKGROUND_3: number;
    export { BACKGROUND_3 as BACKGROUND };
    export let EDGES: number;
    export let NODES: number;
    export let CONTAINERS: number;
    export let HANDLES: number;
    export let CONTROLS: number;
    export let PANELS: number;
    export let MODALS: number;
    export let TOOLTIPS: number;
}
export namespace BREAKPOINTS {
    let SM_1: number;
    export { SM_1 as SM };
    let MD_1: number;
    export { MD_1 as MD };
    let LG_1: number;
    export { LG_1 as LG };
    let XL_1: number;
    export { XL_1 as XL };
}
export namespace ELK_ALGORITHMS {
    let LAYERED: string;
    let STRESS: string;
    let MRTREE: string;
    let RADIAL: string;
    let FORCE: string;
}
export namespace ELK_DIRECTIONS {
    let DOWN: string;
    let UP: string;
    let LEFT: string;
    let RIGHT: string;
}
export namespace ELK_LAYOUT_CONFIG {
    export namespace DEFAULT_5 {
        import algorithm = ELK_ALGORITHMS.LAYERED;
        export { algorithm };
        import direction = ELK_DIRECTIONS.DOWN;
        export { direction };
        import spacing = LAYOUT_SPACING.NODE_TO_NODE_NORMAL;
        export { spacing };
        export namespace nodeSize {
            import width = SIZES.NODE_MIN_WIDTH;
            export { width };
            import height = SIZES.NODE_MIN_HEIGHT;
            export { height };
        }
    }
    export { DEFAULT_5 as DEFAULT };
    export namespace COMPACT {
        import algorithm_1 = ELK_ALGORITHMS.LAYERED;
        export { algorithm_1 as algorithm };
        import direction_1 = ELK_DIRECTIONS.DOWN;
        export { direction_1 as direction };
        import spacing_1 = LAYOUT_SPACING.NODE_TO_NODE_COMPACT;
        export { spacing_1 as spacing };
        export namespace nodeSize_1 {
            import width_1 = SIZES.NODE_MIN_WIDTH;
            export { width_1 as width };
            import height_1 = SIZES.NODE_MIN_HEIGHT;
            export { height_1 as height };
        }
        export { nodeSize_1 as nodeSize };
    }
    export namespace LOOSE {
        import algorithm_2 = ELK_ALGORITHMS.LAYERED;
        export { algorithm_2 as algorithm };
        import direction_2 = ELK_DIRECTIONS.DOWN;
        export { direction_2 as direction };
        import spacing_2 = LAYOUT_SPACING.NODE_TO_NODE_LOOSE;
        export { spacing_2 as spacing };
        export namespace nodeSize_2 {
            import width_2 = SIZES.NODE_MIN_WIDTH;
            export { width_2 as width };
            import height_2 = SIZES.NODE_MIN_HEIGHT;
            export { height_2 as height };
        }
        export { nodeSize_2 as nodeSize };
    }
    export namespace FORCE_DIRECTED {
        import algorithm_3 = ELK_ALGORITHMS.FORCE;
        export { algorithm_3 as algorithm };
        import direction_3 = ELK_DIRECTIONS.DOWN;
        export { direction_3 as direction };
        import spacing_3 = LAYOUT_SPACING.NODE_TO_NODE_NORMAL;
        export { spacing_3 as spacing };
        export namespace nodeSize_3 {
            import width_3 = SIZES.NODE_MIN_WIDTH;
            export { width_3 as width };
            import height_3 = SIZES.NODE_MIN_HEIGHT;
            export { height_3 as height };
        }
        export { nodeSize_3 as nodeSize };
    }
    export namespace HORIZONTAL {
        import algorithm_4 = ELK_ALGORITHMS.LAYERED;
        export { algorithm_4 as algorithm };
        import direction_4 = ELK_DIRECTIONS.RIGHT;
        export { direction_4 as direction };
        import spacing_4 = LAYOUT_SPACING.NODE_TO_NODE_NORMAL;
        export { spacing_4 as spacing };
        export namespace nodeSize_4 {
            import width_4 = SIZES.NODE_MIN_WIDTH;
            export { width_4 as width };
            import height_4 = SIZES.NODE_MIN_HEIGHT;
            export { height_4 as height };
        }
        export { nodeSize_4 as nodeSize };
    }
}
export namespace ELK_LAYOUT_OPTIONS {
    let LAYERED_1: {
        'elk.layered.spacing.nodeNodeBetweenLayers': number;
        'elk.layered.nodePlacement.strategy': string;
        'elk.layered.crossingMinimization.strategy': string;
        'elk.layered.layering.strategy': string;
    };
    export { LAYERED_1 as LAYERED };
    let FORCE_1: {
        'elk.force.repulsivePower': number;
        'elk.force.iterations': number;
        'elk.force.temperature': number;
    };
    export { FORCE_1 as FORCE };
    let STRESS_1: {
        'elk.stress.iterations': number;
        'elk.stress.epsilon': number;
    };
    export { STRESS_1 as STRESS };
    export let SPACING: {
        'elk.spacing.nodeNode': number;
        'elk.spacing.edgeNode': number;
        'elk.spacing.edgeEdge': number;
        'elk.spacing.componentComponent': number;
    };
    export let PADDING: {
        'elk.padding.left': number;
        'elk.padding.right': number;
        'elk.padding.top': number;
        'elk.padding.bottom': number;
    };
}
export namespace DEFAULT_NODE_STYLE {
    import borderRadius = DEFAULT_STYLES.BORDER_RADIUS;
    export { borderRadius };
    export let padding: string;
    export let color: string;
    import fontFamily = TYPOGRAPHY.FONT_FAMILY;
    export { fontFamily };
    export let fontSize: string;
    export let fontWeight: string;
    export let textShadow: string;
    export let border: string;
    import boxShadow = SHADOWS.NODE_DEFAULT;
    export { boxShadow };
    import transition = ANIMATIONS.TRANSITION_DEFAULT;
    export { transition };
    export let display: string;
    export let alignItems: string;
    export let justifyContent: string;
    export let textAlign: string;
    let width_5: number;
    export { width_5 as width };
    let height_5: number;
    export { height_5 as height };
    export let cursor: string;
}
export namespace DEFAULT_EDGE_STYLE {
    import strokeWidth = SIZES.EDGE_WIDTH_DEFAULT;
    export { strokeWidth };
    import stroke = EDGE_COLORS.DEFAULT;
    export { stroke };
    import strokeDasharray = DASH_PATTERNS.SOLID;
    export { strokeDasharray };
}
export namespace DEFAULT_CONTAINER_STYLE {
    import backgroundColor = CONTAINER_COLORS.BACKGROUND;
    export { backgroundColor };
    let border_1: string;
    export { border_1 as border };
    let borderRadius_1: string;
    export { borderRadius_1 as borderRadius };
    let padding_1: string;
    export { padding_1 as padding };
    import boxShadow_1 = SHADOWS.CONTAINER_DEFAULT;
    export { boxShadow_1 as boxShadow };
}
export namespace ELK_NODE_SIZE_CONSTRAINTS {
    let FREE: string;
    let FIXED_SIZE: string;
    let FIXED_POS: string;
    let MINIMUM_SIZE: string;
}
//# sourceMappingURL=config.d.ts.map