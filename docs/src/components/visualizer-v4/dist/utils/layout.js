/**
 * Clean Layout Coordinator
 *
 * Provides graph layout using ELK and ReactFlow state managers with centralized
 * VisualState as the single source of truth.
 */
import { createELKStateManager } from './elkStateManager.js';
import { filterNodesByType, filterNodesByParent, filterNodesExcludingType } from './constants.js';
// Cache for storing original expanded container dimensions
// This ensures we always use the correct expanded dimensions for layout calculations
const containerDimensionsCache = new Map();
// Create ELK state manager instance
const elkStateManager = createELKStateManager();
/**
 * Central Visual State Management
 * This structure contains all the mutable visual state for the visualizer
 */
class VisualState {
    constructor() {
        this.containers = new Map(); // containerId -> 'expanded' | 'collapsed' | 'hidden'
        this.nodes = new Map(); // nodeId -> 'visible' | 'hidden'
        this.edges = new Map(); // edgeId -> 'visible' | 'hidden'
        this.dimensionsCache = containerDimensionsCache; // Reference to shared cache
    }
    setContainerState(containerId, state) {
        if (!['expanded', 'collapsed', 'hidden'].includes(state)) {
            throw new Error(`Invalid container state '${state}'. Must be 'expanded', 'collapsed', or 'hidden'.`);
        }
        this.containers.set(containerId, state);
    }
    setNodeState(nodeId, state) {
        if (!['visible', 'hidden'].includes(state)) {
            throw new Error(`Invalid node state '${state}'. Must be 'visible' or 'hidden'.`);
        }
        this.nodes.set(nodeId, state);
    }
    setEdgeState(edgeId, state) {
        if (!['visible', 'hidden'].includes(state)) {
            throw new Error(`Invalid edge state '${state}'. Must be 'visible' or 'hidden'.`);
        }
        this.edges.set(edgeId, state);
    }
    getContainerState(containerId) {
        return this.containers.get(containerId) || 'expanded'; // Default to expanded
    }
    getNodeState(nodeId) {
        return this.nodes.get(nodeId) || 'visible'; // Default to visible
    }
    getEdgeState(edgeId) {
        return this.edges.get(edgeId) || 'visible'; // Default to visible
    }
    // Convert to the object format for backward compatibility
    toContainerStatesObject() {
        const obj = {};
        this.containers.forEach((state, id) => {
            obj[id] = state;
        });
        return obj;
    }
}
/**
 * Apply full layout for dimension caching (backward compatibility)
 */
export async function applyLayout(nodes, edges, layoutType = 'mrtree') {
    console.log(`[Layout] 🚀 APPLY_LAYOUT: Using ELK state manager for full layout`);
    try {
        const result = await elkStateManager.calculateFullLayout(nodes, edges, layoutType);
        // CRITICAL: Cache the expanded dimensions of all group nodes for later use
        // This ensures we always have the correct expanded dimensions for layout calculations
        result.nodes.forEach(node => {
            if (node.type === 'group') {
                console.log(`[Layout] 💾 CACHING: ${node.id} → ${node.width}x${node.height}`);
                containerDimensionsCache.set(node.id, {
                    width: node.width,
                    height: node.height
                });
            }
        });
        return result;
    }
    catch (error) {
        console.error('[Layout] applyLayout failed:', error);
        throw error;
    }
}
/**
 * Apply layout readjustment for collapsed containers only (backward compatibility)
 */
export async function applyLayoutForCollapsedContainers(displayNodes, edges, layoutType = 'mrtree', changedContainerId = null) {
    console.log(`[Layout] 🔄 CONTAINER_LAYOUT: Using ELK state manager for container repositioning`);
    try {
        return await elkStateManager.calculateContainerRepositioning(displayNodes, edges, layoutType, changedContainerId);
    }
    catch (error) {
        console.error('[Layout] Container layout failed:', error);
        return { nodes: displayNodes, edges }; // Fallback to original
    }
}
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
export async function layoutVisualElements(allNodes, allEdges, visualState, layoutType = 'mrtree') {
    console.log(`[Layout] 🎯 VISUAL_COORDINATOR: Starting layout with ELK state manager`);
    // Handle backward compatibility - convert legacy object to VisualState
    let state;
    if (visualState instanceof VisualState) {
        state = visualState;
    }
    else {
        // Legacy mode: create VisualState from containerStates object
        state = new VisualState();
        Object.entries(visualState).forEach(([containerId, containerState]) => {
            state.setContainerState(containerId, containerState);
        });
        // Default all nodes and edges to visible in legacy mode
        allNodes.forEach(node => {
            if (node.type !== 'group') { // Don't set state for containers, already handled above
                state.setNodeState(node.id, 'visible');
            }
        });
        allEdges.forEach(edge => state.setEdgeState(edge.id, 'visible'));
    }
    try {
        // Use ELK state manager to calculate layout based on visual state
        const elkResult = await elkStateManager.calculateVisualLayout(allNodes, allEdges, state, layoutType, containerDimensionsCache);
        console.log(`[Layout] 🎯 VISUAL_COORDINATOR: ELK calculation complete`);
        // STEP 6: Reroute edges based on final positioned nodes and ELK routing
        console.log(`[Layout] 🔗 VISUAL_COORDINATOR: Rerouting edges for positioned containers...`);
        const { transformAndRerouteEdges, transformNodes } = createVisualFilters(state);
        // Transform nodes to apply collapsed state (hide child nodes, transform containers)
        const transformedNodes = transformNodes(elkResult.nodes);
        // Apply ELK-calculated dimensions to collapsed containers
        const transformedNodesWithELKDimensions = transformedNodes.map(node => {
            if (node.type === 'collapsedContainer') {
                // Find the ELK-calculated dimensions for this container
                const elkContainer = elkResult.elkResult?.children?.find(c => c.id === node.id);
                if (elkContainer) {
                    return {
                        ...node,
                        width: elkContainer.width,
                        height: elkContainer.height,
                        style: {
                            ...node.style,
                            width: elkContainer.width,
                            height: elkContainer.height,
                        }
                    };
                }
            }
            return node;
        });
        // Only use nodes that are actually visible (not hidden by collapsed parents)
        const finalVisibleNodes = transformedNodesWithELKDimensions.filter(node => !node.hidden);
        console.log(`[Layout] 🎯 FINAL_NODES: ${finalVisibleNodes.length} final visible nodes`);
        finalVisibleNodes.forEach(node => {
            console.log(`[Layout] 🎯 FINAL_NODE: ${node.id} (type: ${node.type}, pos: ${node.position?.x || 0},${node.position?.y || 0}, size: ${node.width || '?'}x${node.height || '?'})`);
            console.log(`[Layout] 🎯 NODE_DATA:`, JSON.stringify({
                id: node.id,
                type: node.type,
                position: node.position,
                width: node.width,
                height: node.height,
                style: node.style
            }, null, 2));
        });
        const reroutedEdges = transformAndRerouteEdges(allEdges, allNodes, finalVisibleNodes);
        console.log(`[Layout] 🎯 FINAL_EDGES: ${reroutedEdges.length} rerouted edges`);
        reroutedEdges.forEach(edge => {
            console.log(`[Layout] 🎯 FINAL_EDGE: ${edge.id} (${edge.source} -> ${edge.target})`);
            console.log(`[Layout] 🎯 EDGE_DATA:`, JSON.stringify({
                id: edge.id,
                source: edge.source,
                target: edge.target,
                sourceHandle: edge.sourceHandle,
                targetHandle: edge.targetHandle,
                type: edge.type
            }, null, 2));
        });
        return {
            nodes: finalVisibleNodes,
            edges: reroutedEdges,
        };
    }
    catch (error) {
        console.error(`[Layout] ❌ VISUAL_COORDINATOR_FAILED:`, error);
        throw error;
    }
}
/**
 * Clear the container dimensions cache when graph data changes
 * This should be called whenever new graph data is loaded
 */
export function clearContainerDimensionsCache() {
    containerDimensionsCache.clear();
}
/**
 * Create a new VisualState instance
 * @returns {VisualState} New visual state manager
 */
export function createVisualState() {
    return new VisualState();
}
/**
 * Create VisualState from nodes/edges with default visible states
 * @param {Array} nodes - All nodes in the graph
 * @param {Array} edges - All edges in the graph
 * @param {Object} containerStates - Initial container states (optional)
 * @returns {VisualState} Initialized visual state
 */
export function createVisualStateFromGraph(nodes, edges, containerStates = {}) {
    const state = new VisualState();
    // Set container states
    nodes
        .filter(node => node.type === 'group' || node.type === 'collapsedContainer')
        .forEach(container => {
        const containerState = containerStates[container.id] || 'expanded';
        state.setContainerState(container.id, containerState);
    });
    // Set all regular nodes as visible by default
    nodes
        .filter(node => node.type !== 'group' && node.type !== 'collapsedContainer')
        .forEach(node => {
        state.setNodeState(node.id, 'visible');
    });
    // Set all edges as visible by default
    edges.forEach(edge => {
        state.setEdgeState(edge.id, 'visible');
    });
    return state;
}
/**
 * Create a common visual element filter function
 * This can be reused by both ELK and ReactFlow rendering
 * @param {VisualState} visualState - Central visual state
 * @returns {Object} { filterNodes, filterEdges, transformNodes } - Filter and transform functions
 */
export function createVisualFilters(visualState) {
    return {
        filterNodes: (nodes) => {
            return nodes.filter(node => {
                if (node.type === 'group' || node.type === 'collapsedContainer') {
                    return visualState.getContainerState(node.id) !== 'hidden';
                }
                else {
                    return visualState.getNodeState(node.id) !== 'hidden';
                }
            });
        },
        filterEdges: (edges, visibleNodeIds) => {
            const nodeSet = new Set(visibleNodeIds);
            return edges.filter(edge => {
                // Check edge's own visibility
                if (visualState.getEdgeState(edge.id) === 'hidden') {
                    return false;
                }
                // Check if both endpoints are visible
                return nodeSet.has(edge.source) && nodeSet.has(edge.target);
            });
        },
        transformAndRerouteEdges: (edges, nodes, transformedNodes) => {
            // Build maps for efficient lookups
            const nodeMap = new Map(nodes.map(node => [node.id, node]));
            const transformedNodeMap = new Map(transformedNodes.map(node => [node.id, node]));
            const visibleNodeIds = new Set(transformedNodes.filter(node => !node.hidden).map(node => node.id));
            console.log(`[Layout] 🔗 EDGE_REROUTING: Processing ${edges.length} edges, ${visibleNodeIds.size} visible nodes`);
            // Helper function to find the top-level visible container for a node
            function findVisibleContainer(nodeId, visited = new Set()) {
                if (visited.has(nodeId))
                    return null; // Prevent infinite loops
                visited.add(nodeId);
                const node = nodeMap.get(nodeId);
                if (!node)
                    return null;
                // If this node is visible, return it
                if (visibleNodeIds.has(nodeId)) {
                    return nodeId;
                }
                // If this node has a parent, check if the parent is a collapsed container
                if (node.parentId) {
                    const parent = nodeMap.get(node.parentId);
                    if (parent && (parent.type === 'group' || parent.type === 'collapsedContainer')) {
                        const parentState = visualState.getContainerState(parent.id);
                        if (parentState === 'collapsed' && visibleNodeIds.has(parent.id)) {
                            return parent.id; // Parent is collapsed and visible
                        }
                    }
                    // Recursively check parent's container
                    return findVisibleContainer(node.parentId, visited);
                }
                return null;
            }
            const reroutedEdges = [];
            const edgeSet = new Set(); // To prevent duplicate edges
            edges.forEach(edge => {
                // Check edge's own visibility
                if (visualState.getEdgeState(edge.id) === 'hidden') {
                    return;
                }
                const sourceContainer = findVisibleContainer(edge.source);
                const targetContainer = findVisibleContainer(edge.target);
                console.log(`[Layout] 🔗 EDGE: ${edge.source} -> ${edge.target} | Containers: ${sourceContainer} -> ${targetContainer}`);
                // Only include edge if both endpoints have visible containers
                if (sourceContainer && targetContainer) {
                    const newSource = sourceContainer;
                    const newTarget = targetContainer;
                    // Avoid self-loops
                    if (newSource === newTarget) {
                        console.log(`[Layout] 🔗 SKIP: Self-loop ${newSource} -> ${newTarget}`);
                        return;
                    }
                    // Create unique edge ID to prevent duplicates
                    const edgeKey = `${newSource}->${newTarget}`;
                    if (!edgeSet.has(edgeKey)) {
                        edgeSet.add(edgeKey);
                        // Check if we have ELK routing information for this edge
                        // Use clean edge ID format to avoid collision
                        const reroutedEdgeId = `${edge.id.split('_rerouted_')[0]}_rerouted_${newSource}_${newTarget}`;
                        const reroutedEdge = {
                            ...edge,
                            id: reroutedEdgeId,
                            source: newSource,
                            target: newTarget,
                            sourceHandle: 'source', // Use standard handle IDs
                            targetHandle: 'target', // Use standard handle IDs
                            data: {
                                ...edge.data,
                                isRerouted: true,
                                originalSource: edge.source,
                                originalTarget: edge.target,
                                originalSourceHandle: edge.sourceHandle,
                                originalTargetHandle: edge.targetHandle,
                            }
                        };
                        console.log(`[Layout] 🔗 REROUTED: ${reroutedEdge.id} (${newSource} -> ${newTarget})`);
                        reroutedEdges.push(reroutedEdge);
                    }
                }
            });
            console.log(`[Layout] 🔗 EDGE_REROUTING_COMPLETE: ${reroutedEdges.length} rerouted edges`);
            return reroutedEdges;
        },
        transformNodes: (nodes) => {
            return nodes.map(node => {
                // Transform containers based on their state
                if (node.type === 'group' || node.type === 'collapsedContainer') {
                    const containerState = visualState.getContainerState(node.id);
                    if (containerState === 'collapsed') {
                        // Transform to collapsed container but preserve ELK-calculated dimensions
                        return {
                            ...node,
                            type: 'collapsedContainer',
                            // Keep the original width/height from ELK positioning - don't override with hardcoded values
                            data: {
                                ...node.data,
                                label: node.data?.label || node.id,
                                // Add node count if available
                                nodeCount: countChildNodes(node.id, nodes)
                            },
                            hidden: false
                        };
                    }
                    else if (containerState === 'expanded') {
                        // Transform to expanded group - preserve ELK dimensions
                        return {
                            ...node,
                            type: 'group',
                            hidden: false
                        };
                    }
                }
                // For child nodes, check if their parent is collapsed
                if (node.parentId) {
                    const parentState = visualState.getContainerState(node.parentId);
                    if (parentState === 'collapsed') {
                        console.log(`[Layout] 🎯 TRANSFORM: Child node ${node.id} → HIDDEN (parent ${node.parentId} is collapsed)`);
                        return {
                            ...node,
                            hidden: true
                        };
                    }
                    else {
                        // Parent is expanded, so child should be visible
                        console.log(`[Layout] 🎯 TRANSFORM: Child node ${node.id} → VISIBLE (parent ${node.parentId} is expanded)`);
                        return {
                            ...node,
                            hidden: false
                        };
                    }
                }
                // Regular nodes without parents are always visible
                console.log(`[Layout] 🎯 TRANSFORM: Regular node ${node.id} → VISIBLE (no parent)`);
                return {
                    ...node,
                    hidden: false
                };
            });
        }
    };
}
// Helper function to count child nodes
function countChildNodes(containerId, nodes) {
    let count = 0;
    nodes.forEach(node => {
        if (node.parentId === containerId) {
            if (node.type === 'group') {
                // Recursively count child nodes in nested containers
                count += countChildNodes(node.id, nodes);
            }
            else {
                count += 1;
            }
        }
    });
    return count;
}
// Export the VisualState class for external use
export { VisualState };
//# sourceMappingURL=layout.js.map