/**
 * @fileoverview VisualizationState Adapter
 *
 * Bridges the existing VisualizationState implementation with the ReactFlow interface.
 */
import { VisualizationState as CoreVisualizationState } from '../core/VisState';
import type { VisualizationState, GraphNode, GraphEdge, Container, HyperEdge, CreateNodeProps, CreateEdgeProps, CreateContainerProps } from '../shared/types';
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
    get visibleNodes(): GraphNode[];
    get visibleEdges(): GraphEdge[];
    get visibleContainers(): Container[];
    get allHyperEdges(): HyperEdge[];
}
/**
 * Creates a VisualizationState adapter from a core implementation
 */
export declare function createVisualizationStateAdapter(core: CoreVisualizationState): VisualizationState;
//# sourceMappingURL=adapter.d.ts.map