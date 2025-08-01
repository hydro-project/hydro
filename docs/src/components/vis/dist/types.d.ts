/**
 * @fileoverview Type definitions for the Vis component
 *
 * Core TypeScript interfaces and types for the Hydro graph visualization system.
 * These types provide compile-time safety and better developer experience.
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
export type NodeStyle = typeof NODE_STYLES[keyof typeof NODE_STYLES];
export type EdgeStyle = typeof EDGE_STYLES[keyof typeof EDGE_STYLES];
export type ContainerStyle = typeof CONTAINER_STYLES[keyof typeof CONTAINER_STYLES];
export interface Dimensions {
    width: number;
    height: number;
}
export interface GraphNode {
    id: string;
    label: string;
    style: NodeStyle;
    hidden: boolean;
    [key: string]: any;
}
export interface GraphEdge {
    id: string;
    source: string;
    target: string;
    style: EdgeStyle;
    hidden: boolean;
    [key: string]: any;
}
export interface Container {
    id: string;
    expandedDimensions: Dimensions;
    collapsed: boolean;
    hidden: boolean;
    children: Set<string>;
    [key: string]: any;
}
export interface HyperEdge {
    id: string;
    source: string;
    target: string;
    style: EdgeStyle;
    aggregatedEdges: GraphEdge[];
    [key: string]: any;
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
    [key: string]: any;
}
export interface ParseResult {
    state: VisualizationState;
    metadata: {
        selectedGrouping: string | null;
        nodeCount: number;
        edgeCount: number;
        containerCount: number;
    };
}
export interface ValidationResult {
    isValid: boolean;
    errors: string[];
    warnings: string[];
    nodeCount: number;
    edgeCount: number;
    hierarchyCount: number;
}
export interface GroupingOption {
    id: string;
    name: string;
}
export interface VisualizationState {
    setGraphNode(id: string, props: CreateNodeProps): GraphNode;
    getGraphNode(id: string): GraphNode | undefined;
    setNodeHidden(id: string, hidden: boolean): void;
    getNodeHidden(id: string): boolean | undefined;
    removeGraphNode(id: string): void;
    setGraphEdge(id: string, props: CreateEdgeProps): GraphEdge;
    getGraphEdge(id: string): GraphEdge | undefined;
    setEdgeHidden(id: string, hidden: boolean): void;
    getEdgeHidden(id: string): boolean | undefined;
    removeGraphEdge(id: string): void;
    setContainer(id: string, props: CreateContainerProps): Container;
    getContainer(id: string): Container | undefined;
    setContainerCollapsed(id: string, collapsed: boolean): void;
    getContainerCollapsed(id: string): boolean | undefined;
    setContainerHidden(id: string, hidden: boolean): void;
    getContainerHidden(id: string): boolean | undefined;
    getVisibleNodes(): GraphNode[];
    getVisibleEdges(): GraphEdge[];
    getVisibleContainers(): Container[];
    getHyperEdges(): HyperEdge[];
    addContainerChild(containerId: string, childId: string): void;
    removeContainerChild(containerId: string, childId: string): void;
    getContainerChildren(containerId: string): Set<string> | undefined;
    getNodeContainer(nodeId: string): string | undefined;
    collapseContainer(containerId: string): void;
    expandContainer(containerId: string): void;
}
//# sourceMappingURL=types.d.ts.map