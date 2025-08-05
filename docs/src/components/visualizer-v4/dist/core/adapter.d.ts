/**
 * @fileoverview VisualizationState Adapter
 *
 * Bridges the existing VisualizationState implementation with the ReactFlow interface.
 */
import { VisualizationState as CoreVisualizationState } from '../core/VisState';
import type { VisualizationState, GraphNode, GraphEdge, HyperEdge, CreateNodeProps, CreateEdgeProps, CreateContainerProps, ExternalContainer } from '../shared/types';
/**
 * Adapter that implements the VisualizationState interface using the core implementation
 */
export declare class VisualizationStateAdapter implements VisualizationState {
    private core;
    constructor(core: CoreVisualizationState);
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
    getVisibleNodes(): GraphNode[];
    getVisibleEdges(): GraphEdge[];
    getVisibleContainers(): ExternalContainer[];
    getHyperEdges(): HyperEdge[];
    addContainerChild(containerId: string, childId: string): void;
    removeContainerChild(containerId: string, childId: string): void;
    getContainerChildren(containerId: string): Set<string> | undefined;
    getNodeContainer(nodeId: string): string | undefined;
    collapseContainer(containerId: string): void;
    expandContainer(containerId: string): void;
    setNodeLayout(id: string, layout: Partial<any>): void;
    getNodeLayout(id: string): any;
    setEdgeLayout(id: string, layout: Partial<any>): void;
    getEdgeLayout(id: string): any;
    setContainerLayout(id: string, layout: Partial<any>): void;
    getContainerLayout(id: string): any;
    setContainerELKFixed(id: string, fixed: boolean): void;
    getContainerELKFixed(id: string): boolean | undefined;
    getContainersRequiringLayout(changedContainerId?: string): ExternalContainer[];
    get visibleNodes(): GraphNode[];
    get visibleEdges(): GraphEdge[];
    get visibleContainers(): ExternalContainer[];
    get allHyperEdges(): HyperEdge[];
}
/**
 * Creates a VisualizationState adapter from a core implementation
 */
export declare function createVisualizationStateAdapter(core: CoreVisualizationState): VisualizationState;
//# sourceMappingURL=adapter.d.ts.map