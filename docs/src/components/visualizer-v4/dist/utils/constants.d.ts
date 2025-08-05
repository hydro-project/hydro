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
    export let EDGE_NETWORK: any;
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
export function getAccessibleTextColor(backgroundColor: any): string;
export function getStatusColor(status: any, variant?: string): any;
export function isValidGraphData(graphData: any): boolean;
export function isValidNodesArray(nodes: any): boolean;
export function filterNodesByType(nodes: any, type: any): any;
export function filterNodesByParent(nodes: any, parentId: any): any;
export function filterNodesExcludingType(nodes: any, type: any): any;
export function getUniqueNodesById(nodes: any): any;
export namespace ANIMATION_TIMINGS {
    let FIT_VIEW_DURATION: number;
    let FIT_VIEW_DEBOUNCE: number;
    let LAYOUT_DEBOUNCE: number;
    let RESIZE_DEBOUNCE: number;
}
export namespace ZOOM_LEVELS {
    let MIN_INTERACTIVE: number;
    let MAX_INTERACTIVE: number;
    let MIN_FIT_VIEW: number;
    let MAX_FIT_VIEW: number;
    let DEFAULT: number;
}
export namespace LAYOUT_SPACING {
    let NODE_TO_NODE_COMPACT: number;
    let NODE_TO_NODE_NORMAL: number;
    let NODE_TO_NODE_LOOSE: number;
    let EDGE_TO_NODE: number;
    let EDGE_TO_EDGE: number;
    let EDGE_TO_EDGE_ALTERNATE: number;
    let LAYER_SEPARATION: number;
    let COMPONENT_TO_COMPONENT: number;
    let CONTAINER_PADDING: number;
    let ROOT_PADDING: number;
    let BORDER_TO_NODE: number;
}
//# sourceMappingURL=constants.d.ts.map