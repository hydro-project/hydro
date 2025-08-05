/**
 * Apply full layout for dimension caching (backward compatibility)
 */
export function applyLayout(nodes: any, edges: any, layoutType?: string): Promise<any>;
/**
 * Apply layout readjustment for collapsed containers only (backward compatibility)
 */
export function applyLayoutForCollapsedContainers(displayNodes: any, edges: any, layoutType?: string, changedContainerId?: any): Promise<any>;
/**
 * Central Visual Layout Coordinator
 * This is the single source of truth for what ELK sees and how visual elements are arranged.
 * Handles all visual state filtering and applies layout based on explicit state declarations.
 *
 * @param {Array} allNodes - All nodes in the graph (before filtering)
 * @param {Array} allEdges - All edges in the graph (before filtering)
 * @param {VisualState|Object} visualState - Central visual state or legacy container states object
 * @param {string} layoutType - Layout algorithm type
 * @returns {Object} { nodes, edges } - Filtered and positioned visual elements
 */
export function layoutVisualElements(allNodes: any[], allEdges: any[], visualState: VisualState | any, layoutType?: string): any;
/**
 * Clear the container dimensions cache when graph data changes
 * This should be called whenever new graph data is loaded
 */
export function clearContainerDimensionsCache(): void;
/**
 * Create a new VisualState instance
 * @returns {VisualState} New visual state manager
 */
export function createVisualState(): VisualState;
/**
 * Create VisualState from nodes/edges with default visible states
 * @param {Array} nodes - All nodes in the graph
 * @param {Array} edges - All edges in the graph
 * @param {Object} containerStates - Initial container states (optional)
 * @returns {VisualState} Initialized visual state
 */
export function createVisualStateFromGraph(nodes: any[], edges: any[], containerStates?: any): VisualState;
/**
 * Create a common visual element filter function
 * This can be reused by both ELK and ReactFlow rendering
 * @param {VisualState} visualState - Central visual state
 * @returns {Object} { filterNodes, filterEdges, transformNodes } - Filter and transform functions
 */
export function createVisualFilters(visualState: VisualState): any;
/**
 * Central Visual State Management
 * This structure contains all the mutable visual state for the visualizer
 */
export class VisualState {
    containers: Map<any, any>;
    nodes: Map<any, any>;
    edges: Map<any, any>;
    dimensionsCache: Map<any, any>;
    setContainerState(containerId: any, state: any): void;
    setNodeState(nodeId: any, state: any): void;
    setEdgeState(edgeId: any, state: any): void;
    getContainerState(containerId: any): any;
    getNodeState(nodeId: any): any;
    getEdgeState(edgeId: any): any;
    toContainerStatesObject(): {};
}
//# sourceMappingURL=layout.d.ts.map