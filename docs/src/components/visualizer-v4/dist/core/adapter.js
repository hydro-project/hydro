/**
 * @fileoverview VisualizationState Adapter
 *
 * Bridges the existing VisualizationState implementation with the ReactFlow interface.
 */
/**
 * Adapter that implements the VisualizationState interface using the core implementation
 */
export class VisualizationStateAdapter {
    constructor(core) {
        this.core = core;
    }
    // Node methods
    setGraphNode(id, props) {
        this.core.setGraphNode(id, props);
        return this.core.getGraphNode(id);
    }
    getGraphNode(id) {
        return this.core.getGraphNode(id);
    }
    setNodeHidden(id, hidden) {
        this.core.updateNode(id, { hidden });
    }
    getNodeHidden(id) {
        const node = this.core.getGraphNode(id);
        return node?.hidden;
    }
    removeGraphNode(id) {
        this.core.removeGraphNode(id);
    }
    // Edge methods
    setGraphEdge(id, props) {
        this.core.setGraphEdge(id, props);
        return this.core.getGraphEdge(id);
    }
    getGraphEdge(id) {
        return this.core.getGraphEdge(id);
    }
    setEdgeHidden(id, hidden) {
        this.core.updateEdge(id, { hidden });
    }
    getEdgeHidden(id) {
        const edge = this.core.getGraphEdge(id);
        return edge?.hidden;
    }
    removeGraphEdge(id) {
        this.core.removeGraphEdge(id);
    }
    // Container methods
    setContainer(id, props) {
        this.core.setContainer(id, props);
        return this.core.getContainer(id);
    }
    getContainer(id) {
        return this.core.getContainer(id);
    }
    setContainerCollapsed(id, collapsed) {
        this.core.updateContainer(id, { collapsed });
    }
    getContainerCollapsed(id) {
        const container = this.core.getContainer(id);
        return container?.collapsed;
    }
    setContainerHidden(id, hidden) {
        this.core.updateContainer(id, { hidden });
    }
    getContainerHidden(id) {
        const container = this.core.getContainer(id);
        return container?.hidden;
    }
    // Visibility methods
    getVisibleNodes() {
        return this.core.visibleNodes;
    }
    getVisibleEdges() {
        return this.core.visibleEdges;
    }
    getVisibleContainers() {
        // Cast is safe: VisState never exposes expandedDimensions externally
        return this.core.visibleContainers;
    }
    getHyperEdges() {
        // HyperEdges are now completely encapsulated within VisState
        // External code should not access them directly
        return [];
    }
    // Container hierarchy methods
    addContainerChild(containerId, childId) {
        this.core.addContainerChild(containerId, childId);
    }
    removeContainerChild(containerId, childId) {
        this.core.removeContainerChild(containerId, childId);
    }
    getContainerChildren(containerId) {
        const children = this.core.getContainerChildren(containerId);
        return children ? new Set(children) : undefined;
    }
    getNodeContainer(nodeId) {
        return this.core.getNodeContainer(nodeId);
    }
    // Container operations
    collapseContainer(containerId) {
        this.core.collapseContainer(containerId);
    }
    expandContainer(containerId) {
        this.core.expandContainer(containerId);
    }
    // Layout methods - delegate to core
    setNodeLayout(id, layout) {
        this.core.setNodeLayout(id, layout);
    }
    getNodeLayout(id) {
        return this.core.getNodeLayout(id);
    }
    setEdgeLayout(id, layout) {
        this.core.setEdgeLayout(id, layout);
    }
    getEdgeLayout(id) {
        return this.core.getEdgeLayout(id);
    }
    setContainerLayout(id, layout) {
        this.core.setContainerLayout(id, layout);
    }
    getContainerLayout(id) {
        return this.core.getContainerLayout(id);
    }
    // ELK integration methods
    setContainerELKFixed(id, fixed) {
        this.core.setContainerELKFixed(id, fixed);
    }
    getContainerELKFixed(id) {
        return this.core.getContainerELKFixed(id);
    }
    getContainersRequiringLayout(changedContainerId) {
        return this.core.getContainersRequiringLayout(changedContainerId);
    }
    // Visibility properties (readonly getters)
    get visibleNodes() {
        return this.core.visibleNodes;
    }
    get visibleEdges() {
        return this.core.visibleEdges;
    }
    get visibleContainers() {
        // Cast is safe: VisState never exposes expandedDimensions externally
        return this.core.visibleContainers;
    }
    get allHyperEdges() {
        // HyperEdges are now completely encapsulated within VisState
        // External code should not access them directly
        return [];
    }
}
/**
 * Creates a VisualizationState adapter from a core implementation
 */
export function createVisualizationStateAdapter(core) {
    return new VisualizationStateAdapter(core);
}
//# sourceMappingURL=adapter.js.map