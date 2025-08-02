/**
 * @fileoverview VisualizationState Adapter
 * 
 * Bridges the existing VisualizationState implementation with the ReactFlow interface.
 */

import { VisualizationState as CoreVisualizationState } from '../core/VisState';
import type { 
  VisualizationState,
  GraphNode,
  GraphEdge,
  Container,
  HyperEdge,
  CreateNodeProps,
  CreateEdgeProps,
  CreateContainerProps
} from '../shared/types';

/**
 * Adapter that implements the VisualizationState interface using the core implementation
 */
export class VisualizationStateAdapter implements VisualizationState {
  private core: CoreVisualizationState;

  constructor(core: CoreVisualizationState) {
    this.core = core;
  }

  // Node methods
  setGraphNode(id: string, props: CreateNodeProps): GraphNode {
    this.core.setGraphNode(id, props);
    return this.core.getGraphNode(id);
  }

  getGraphNode(id: string): GraphNode | undefined {
    return this.core.getGraphNode(id);
  }

  setNodeHidden(id: string, hidden: boolean): void {
    this.core.updateNode(id, { hidden });
  }

  getNodeHidden(id: string): boolean | undefined {
    const node = this.core.getGraphNode(id);
    return node?.hidden;
  }

  removeGraphNode(id: string): void {
    this.core.removeGraphNode(id);
  }

  // Edge methods
  setGraphEdge(id: string, props: CreateEdgeProps): GraphEdge {
    this.core.setGraphEdge(id, props);
    return this.core.getGraphEdge(id);
  }

  getGraphEdge(id: string): GraphEdge | undefined {
    return this.core.getGraphEdge(id);
  }

  setEdgeHidden(id: string, hidden: boolean): void {
    this.core.updateEdge(id, { hidden });
  }

  getEdgeHidden(id: string): boolean | undefined {
    const edge = this.core.getGraphEdge(id);
    return edge?.hidden;
  }

  removeGraphEdge(id: string): void {
    this.core.removeGraphEdge(id);
  }

  // Container methods
  setContainer(id: string, props: CreateContainerProps): Container {
    this.core.setContainer(id, props);
    return this.core.getContainer(id);
  }

  getContainer(id: string): Container | undefined {
    return this.core.getContainer(id);
  }

  setContainerCollapsed(id: string, collapsed: boolean): void {
    this.core.updateContainer(id, { collapsed });
  }

  getContainerCollapsed(id: string): boolean | undefined {
    const container = this.core.getContainer(id);
    return container?.collapsed;
  }

  setContainerHidden(id: string, hidden: boolean): void {
    this.core.updateContainer(id, { hidden });
  }

  getContainerHidden(id: string): boolean | undefined {
    const container = this.core.getContainer(id);
    return container?.hidden;
  }

  // Visibility methods
  getVisibleNodes(): GraphNode[] {
    return this.core.visibleNodes;
  }

  getVisibleEdges(): GraphEdge[] {
    return this.core.visibleEdges;
  }

  getVisibleContainers(): Container[] {
    return this.core.visibleContainers;
  }

  getHyperEdges(): HyperEdge[] {
    return this.core.allHyperEdges;
  }

  // Container hierarchy methods
  addContainerChild(containerId: string, childId: string): void {
    this.core.addContainerChild(containerId, childId);
  }

  removeContainerChild(containerId: string, childId: string): void {
    this.core.removeContainerChild(containerId, childId);
  }

  getContainerChildren(containerId: string): Set<string> | undefined {
    const children = this.core.getContainerChildren(containerId);
    return children ? new Set(children) : undefined;
  }

  getNodeContainer(nodeId: string): string | undefined {
    return this.core.getNodeContainer(nodeId);
  }

  // Container operations
  collapseContainer(containerId: string): void {
    this.core.updateContainer(containerId, { collapsed: true });
  }

  expandContainer(containerId: string): void {
    this.core.updateContainer(containerId, { collapsed: false });
  }

  // Visibility properties (readonly getters)
  get visibleNodes(): GraphNode[] {
    return this.core.visibleNodes;
  }

  get visibleEdges(): GraphEdge[] {
    return this.core.visibleEdges;
  }

  get visibleContainers(): Container[] {
    return this.core.visibleContainers;
  }

  get allHyperEdges(): HyperEdge[] {
    return this.core.allHyperEdges;
  }
}

/**
 * Creates a VisualizationState adapter from a core implementation
 */
export function createVisualizationStateAdapter(core: CoreVisualizationState): VisualizationState {
  return new VisualizationStateAdapter(core);
}
