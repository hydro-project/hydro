/**
 * Create a new visualization state instance
 */
export function createVisualizationState(): VisualizationState;
export namespace NODE_STYLES {
    let DEFAULT: string;
    let HIGHLIGHTED: string;
    let SELECTED: string;
    let WARNING: string;
    let ERROR: string;
}
export namespace EDGE_STYLES {
    let DEFAULT_1: string;
    export { DEFAULT_1 as DEFAULT };
    let HIGHLIGHTED_1: string;
    export { HIGHLIGHTED_1 as HIGHLIGHTED };
    export let DASHED: string;
    export let THICK: string;
    let WARNING_1: string;
    export { WARNING_1 as WARNING };
}
/**
 * Core visualization state class that manages all graph elements
 */
export class VisualizationState {
    graphNodes: Map<any, any>;
    graphEdges: Map<any, any>;
    containers: Map<any, any>;
    hyperEdges: Map<any, any>;
    visibleNodes: Map<any, any>;
    visibleEdges: Map<any, any>;
    visibleContainers: Map<any, any>;
    expandedContainers: Map<any, any>;
    containerChildren: Map<any, any>;
    nodeContainers: Map<any, any>;
    /**
     * Add or update a graph node
     */
    setGraphNode(id: any, { label, style, hidden, ...otherProps }: {
        [x: string]: any;
        label: any;
        style?: string;
        hidden?: boolean;
    }): {
        id: any;
        label: any;
        style: string;
        hidden: boolean;
    };
    /**
     * Get a graph node by id
     */
    getGraphNode(id: any): any;
    /**
     * Set hidden flag for a graph node
     */
    setNodeHidden(id: any, hidden: any): void;
    /**
     * Get hidden flag for a graph node
     */
    getNodeHidden(id: any): any;
    /**
     * Remove a graph node
     */
    removeGraphNode(id: any): void;
    /**
     * Add or update a graph edge
     */
    setGraphEdge(id: any, { source, target, style, hidden, ...otherProps }: {
        [x: string]: any;
        source: any;
        target: any;
        style?: string;
        hidden?: boolean;
    }): {
        id: any;
        source: any;
        target: any;
        style: string;
        hidden: boolean;
    };
    /**
     * Get a graph edge by id
     */
    getGraphEdge(id: any): any;
    /**
     * Set hidden flag for a graph edge
     */
    setEdgeHidden(id: any, hidden: any): void;
    /**
     * Get hidden flag for a graph edge
     */
    getEdgeHidden(id: any): any;
    /**
     * Remove a graph edge
     */
    removeGraphEdge(id: any): void;
    /**
     * Add or update a container
     */
    setContainer(id: any, { expandedDimensions, collapsed, hidden, children, ...otherProps }: {
        [x: string]: any;
        expandedDimensions?: {
            width: number;
            height: number;
        };
        collapsed?: boolean;
        hidden?: boolean;
        children?: any[];
    }): {
        id: any;
        expandedDimensions: {
            width: number;
            height: number;
        };
        collapsed: boolean;
        hidden: boolean;
        children: Set<any>;
    };
    /**
     * Get a container by id
     */
    getContainer(id: any): any;
    /**
     * Set collapsed flag for a container
     */
    setContainerCollapsed(id: any, collapsed: any): void;
    /**
     * Get collapsed flag for a container
     */
    getContainerCollapsed(id: any): any;
    /**
     * Set hidden flag for a container
     */
    setContainerHidden(id: any, hidden: any): void;
    /**
     * Get hidden flag for a container
     */
    getContainerHidden(id: any): any;
    /**
     * Add a child to a container
     */
    addContainerChild(containerId: any, childId: any): void;
    /**
     * Remove a child from a container
     */
    removeContainerChild(containerId: any, childId: any): void;
    /**
     * Remove a container
     */
    removeContainer(id: any): void;
    /**
     * Add or update a hyper edge
     */
    setHyperEdge(id: any, { source, target, style, ...otherProps }: {
        [x: string]: any;
        source: any;
        target: any;
        style?: string;
    }): {
        id: any;
        source: any;
        target: any;
        style: string;
    };
    /**
     * Get a hyper edge by id
     */
    getHyperEdge(id: any): any;
    /**
     * Remove a hyper edge
     */
    removeHyperEdge(id: any): void;
    /**
     * Get all visible (non-hidden) nodes
     */
    getVisibleNodes(): any[];
    /**
     * Get all visible (non-hidden) edges
     */
    getVisibleEdges(): any[];
    /**
     * Get all visible (non-hidden) containers
     */
    getVisibleContainers(): any[];
    /**
     * Get all expanded (non-collapsed) containers
     */
    getExpandedContainers(): any[];
    /**
     * Get all hyper edges
     */
    getHyperEdges(): any[];
    /**
     * Get container children for a container id
     */
    getContainerChildren(containerId: any): any;
    /**
     * Get the container that contains a given node
     */
    getNodeContainer(nodeId: any): any;
    /**
     * Clear all data
     */
    clear(): void;
    _updateVisibleNodes(id: any, node: any): void;
    _updateVisibleEdges(id: any, edge: any): void;
    _updateVisibleContainers(id: any, container: any): void;
    _updateExpandedContainers(id: any, container: any): void;
}
//# sourceMappingURL=VisualizationState.d.ts.map