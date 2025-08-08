export interface ExternalContainer {
    id: string;
    collapsed: boolean;
    hidden: boolean;
    children: Set<string>;
    layout?: LayoutState;
    [key: string]: any;
}
/**
 * @fileoverview Type definitions for the Vis component
 *
 * Core TypeScript interfaces and types for the graph visualization system.
 * These types provide compile-time safety and better developer experience.
 */
import { NODE_STYLES, EDGE_STYLES, CONTAINER_STYLES } from './constants';
export { NODE_STYLES, EDGE_STYLES, CONTAINER_STYLES } from './constants';
export type NodeStyle = typeof NODE_STYLES[keyof typeof NODE_STYLES];
export type EdgeStyle = typeof EDGE_STYLES[keyof typeof EDGE_STYLES];
export type ContainerStyle = typeof CONTAINER_STYLES[keyof typeof CONTAINER_STYLES];
export interface Dimensions {
    width: number;
    height: number;
}
export interface Position {
    x: number;
    y: number;
}
export interface LayoutState {
    position?: Position;
    dimensions?: Dimensions;
    sections?: any[];
    elkFixed?: boolean;
    elkLayoutOptions?: Record<string, string>;
}
export interface GraphNode {
    id: string;
    label: string;
    style: NodeStyle;
    hidden: boolean;
    layout?: LayoutState;
    [key: string]: any;
}
export interface GraphEdge {
    id: string;
    source: string;
    target: string;
    style: EdgeStyle;
    hidden: boolean;
    layout?: LayoutState;
    [key: string]: any;
}
export interface Container {
    id: string;
    expandedDimensions: Dimensions;
    collapsed: boolean;
    hidden: boolean;
    children: Set<string>;
    layout?: LayoutState;
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
    layout?: LayoutState;
    [key: string]: any;
}
export interface CreateEdgeProps {
    source: string;
    target: string;
    style?: EdgeStyle;
    hidden?: boolean;
    layout?: LayoutState;
    [key: string]: any;
}
export interface CreateContainerProps {
    expandedDimensions?: Dimensions;
    collapsed?: boolean;
    hidden?: boolean;
    children?: string[];
    layout?: LayoutState;
    [key: string]: any;
}
export interface ParseResult {
    state: any;
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
    setContainer(id: string, props: CreateContainerProps): ExternalContainer;
    getContainer(id: string): ExternalContainer | undefined;
    setContainerCollapsed(id: string, collapsed: boolean): void;
    getContainerCollapsed(id: string): boolean | undefined;
    setContainerHidden(id: string, hidden: boolean): void;
    getContainerHidden(id: string): boolean | undefined;
    readonly visibleNodes: GraphNode[];
    readonly visibleEdges: GraphEdge[];
    readonly visibleContainers: ExternalContainer[];
    readonly allHyperEdges: HyperEdge[];
    addContainerChild(containerId: string, childId: string): void;
    removeContainerChild(containerId: string, childId: string): void;
    getContainerChildren(containerId: string): Set<string> | undefined;
    getNodeContainer(nodeId: string): string | undefined;
    collapseContainer(containerId: string): void;
    expandContainer(containerId: string): void;
}
//# sourceMappingURL=types.d.ts.map