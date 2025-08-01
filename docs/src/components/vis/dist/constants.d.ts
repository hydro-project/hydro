/**
 * Visualization Design Constants
 *
 * Centralized constants for styling and configuration of the visualization system
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
    readonly DEFAULT_NODE_WIDTH: 100;
    readonly DEFAULT_NODE_HEIGHT: 40;
    readonly DEFAULT_CONTAINER_PADDING: 20;
    readonly MIN_CONTAINER_WIDTH: 150;
    readonly MIN_CONTAINER_HEIGHT: 100;
};
export type NodeStyle = typeof NODE_STYLES[keyof typeof NODE_STYLES];
export type EdgeStyle = typeof EDGE_STYLES[keyof typeof EDGE_STYLES];
export type ContainerStyle = typeof CONTAINER_STYLES[keyof typeof CONTAINER_STYLES];
export interface Dimensions {
    width: number;
    height: number;
}
export interface BaseEntity {
    id: string;
    hidden: boolean;
    [key: string]: any;
}
export interface GraphNode extends BaseEntity {
    label: string;
    style: NodeStyle;
}
export interface GraphEdge extends BaseEntity {
    source: string;
    target: string;
    style: EdgeStyle;
}
export interface Container extends BaseEntity {
    expandedDimensions: Dimensions;
    collapsed: boolean;
    children: Set<string>;
    label?: string;
}
export interface HyperEdge {
    id: string;
    source: string;
    target: string;
    style: EdgeStyle;
    aggregatedEdges: GraphEdge[];
    [key: string]: any;
}
export interface CollapsedContainer {
    id: string;
    originalContainer: Container;
    position: {
        x: number;
        y: number;
    };
    dimensions: Dimensions;
}
export interface CreateNodeProps {
    label: string;
    style?: NodeStyle;
    hidden?: boolean;
    [key: string]: any;
}
export interface CreateEdgeProps {
    source: string;
    target: string;
    style?: EdgeStyle;
    hidden?: boolean;
    [key: string]: any;
}
export interface CreateContainerProps {
    expandedDimensions?: Dimensions;
    collapsed?: boolean;
    hidden?: boolean;
    children?: string[];
    label?: string;
    [key: string]: any;
}
//# sourceMappingURL=constants.d.ts.map