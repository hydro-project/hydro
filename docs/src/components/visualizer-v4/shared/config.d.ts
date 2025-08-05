/**
 * @fileoverview Basic configuration exports
 *
 * This is a simplified config file that imports what's needed from the current implementation.
 */
export * from './constants';
export declare const COMPONENT_COLORS: {
    BACKGROUND_PRIMARY: string;
    BACKGROUND_SECONDARY: string;
    PANEL_BACKGROUND: string;
    PANEL_HEADER_BACKGROUND: string;
    BORDER_LIGHT: string;
    BORDER_MEDIUM: string;
    TEXT_PRIMARY: string;
    TEXT_SECONDARY: string;
    TEXT_TERTIARY: string;
    TEXT_DISABLED: string;
    BUTTON_HOVER_BACKGROUND: string;
};
export declare const COLOR_PALETTES: {
    Set3: {
        primary: string;
        secondary: string;
        name: string;
    }[];
};
export declare const SIZES: {
    SMALL: string;
    MEDIUM: string;
    LARGE: string;
    BORDER_RADIUS_DEFAULT: string;
    COLLAPSED_CONTAINER_WIDTH: number;
    COLLAPSED_CONTAINER_HEIGHT: number;
};
export declare const SHADOWS: {
    LIGHT: string;
    MEDIUM: string;
    LARGE: string;
    PANEL_DEFAULT: string;
    PANEL_DRAGGING: string;
};
export declare const ELK_ALGORITHMS: {
    MRTREE: string;
    LAYERED: string;
    FORCE: string;
    STRESS: string;
    RADIAL: string;
};
export declare const LAYOUT_SPACING: {
    NODE_NODE: number;
    NODE_EDGE: number;
    EDGE_EDGE: number;
    NODE_TO_NODE_NORMAL: number;
    EDGE_TO_EDGE: number;
    EDGE_TO_NODE: number;
    COMPONENT_TO_COMPONENT: number;
    ROOT_PADDING: number;
    CONTAINER_PADDING: number;
};
export declare const ELK_LAYOUT_OPTIONS: {
    'elk.algorithm': string;
    'elk.direction': string;
    'elk.hierarchyHandling': string;
    'elk.spacing.nodeNode': string;
    'elk.spacing.edgeNode': string;
    'elk.spacing.edgeEdge': string;
    'elk.spacing.componentComponent': string;
    'elk.layered.spacing.nodeNodeBetweenLayers': string;
};
export type ELKAlgorithm = typeof ELK_ALGORITHMS[keyof typeof ELK_ALGORITHMS];
export declare function getELKLayoutOptions(algorithm?: ELKAlgorithm): {
    'elk.algorithm': string;
    'elk.direction': string;
    'elk.hierarchyHandling': string;
    'elk.spacing.nodeNode': string;
    'elk.spacing.edgeNode': string;
    'elk.spacing.edgeEdge': string;
    'elk.spacing.componentComponent': string;
    'elk.layered.spacing.nodeNodeBetweenLayers': string;
};
export declare function createFixedPositionOptions(x?: number, y?: number): {
    'elk.position': string;
    'elk.algorithm': string;
    'elk.direction': string;
    'elk.hierarchyHandling': string;
    'elk.spacing.nodeNode': string;
    'elk.spacing.edgeNode': string;
    'elk.spacing.edgeEdge': string;
    'elk.spacing.componentComponent': string;
    'elk.layered.spacing.nodeNodeBetweenLayers': string;
} | {
    'elk.position.x': string;
    'elk.position.y': string;
    'elk.position': string;
    'elk.algorithm': string;
    'elk.direction': string;
    'elk.hierarchyHandling': string;
    'elk.spacing.nodeNode': string;
    'elk.spacing.edgeNode': string;
    'elk.spacing.edgeEdge': string;
    'elk.spacing.componentComponent': string;
    'elk.layered.spacing.nodeNodeBetweenLayers': string;
};
export declare function createFreePositionOptions(): {
    'elk.position': string;
    'elk.algorithm': string;
    'elk.direction': string;
    'elk.hierarchyHandling': string;
    'elk.spacing.nodeNode': string;
    'elk.spacing.edgeNode': string;
    'elk.spacing.edgeEdge': string;
    'elk.spacing.componentComponent': string;
    'elk.layered.spacing.nodeNodeBetweenLayers': string;
};
//# sourceMappingURL=config.d.ts.map