/**
 * @fileoverview Bridge Architecture Constants
 *
 * Clean constants for our bridge-based implementation.
 * No dependencies on alpha.
 */
export declare const NODE_STYLES: {
    readonly DEFAULT: "default";
    readonly HIGHLIGHTED: "highlighted";
    readonly SELECTED: "selected";
    readonly WARNING: "warning";
    readonly ERROR: "error";
};
export declare const EDGE_STYLES: {
    readonly DEFAULT: "default";
    readonly HIGHLIGHTED: "highlighted";
    readonly DASHED: "dashed";
    readonly THICK: "thick";
    readonly WARNING: "warning";
};
export declare const CONTAINER_STYLES: {
    readonly DEFAULT: "default";
    readonly HIGHLIGHTED: "highlighted";
    readonly SELECTED: "selected";
    readonly MINIMIZED: "minimized";
};
export declare const LAYOUT_CONSTANTS: {
    readonly DEFAULT_NODE_WIDTH: 180;
    readonly DEFAULT_NODE_HEIGHT: 60;
    readonly DEFAULT_CONTAINER_PADDING: 20;
    readonly MIN_CONTAINER_WIDTH: 200;
    readonly MIN_CONTAINER_HEIGHT: 150;
};
export type NodeStyle = keyof typeof NODE_STYLES;
export type EdgeStyle = keyof typeof EDGE_STYLES;
export type ContainerStyle = keyof typeof CONTAINER_STYLES;
//# sourceMappingURL=constants.d.ts.map