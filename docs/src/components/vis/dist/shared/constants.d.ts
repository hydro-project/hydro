/**
 * Visualization Design Constants
 *
 * @deprecated This file is deprecated. Use ../shared/config.ts instead.
 * Re-exports for backward compatibility.
 */
export * from './config';
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
    readonly DEFAULT_NODE_WIDTH: 100;
    readonly DEFAULT_NODE_HEIGHT: 40;
    readonly DEFAULT_CONTAINER_PADDING: 20;
    readonly MIN_CONTAINER_WIDTH: 150;
    readonly MIN_CONTAINER_HEIGHT: 100;
};
export type NodeStyle = typeof NODE_STYLES[keyof typeof NODE_STYLES];
export type EdgeStyle = typeof EDGE_STYLES[keyof typeof EDGE_STYLES];
export type ContainerStyle = typeof CONTAINER_STYLES[keyof typeof CONTAINER_STYLES];
//# sourceMappingURL=constants.d.ts.map