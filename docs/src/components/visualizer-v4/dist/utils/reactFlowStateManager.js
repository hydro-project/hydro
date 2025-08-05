/**
 * ReactFlow State Manager
 *
 * This module provides wrapper functions that ensure all ReactFlow interactions
 * are consistent with the VisualState as the single source of truth.
 *
 * Key principle: ReactFlow should only ever see nodes and edges that exactly
 * match what ELK calculated for the current visual state.
 */
import { layoutVisualElements } from './layout.js';
/**
 * Create a ReactFlow state manager that wraps all ReactFlow interactions
 * with VisualState as the single source of truth.
 *
 * @param {Function} setNodes - ReactFlow setNodes function
 * @param {Function} setEdges - ReactFlow setEdges function
 * @returns {Object} ReactFlow wrapper functions
 */
export function createReactFlowStateManager(setNodes, setEdges) {
    /**
     * Update ReactFlow to show the visual state exactly as ELK calculated it.
     * This is the ONLY function that should call setNodes/setEdges.
     *
     * @param {VisualState} visualState - The current visual state
     * @param {Array} allNodes - Complete source nodes (never filtered)
     * @param {Array} allEdges - Complete source edges (never filtered)
     * @param {string} currentLayout - Current layout algorithm
     * @param {string} operation - Description of the operation (for logging)
     * @returns {Promise<Object>} The layout result
     */
    async function applyVisualState(visualState, allNodes, allEdges, currentLayout, operation = 'update') {
        console.log(`[ReactFlowStateManager] 🎯 ${operation.toUpperCase()}: Applying visual state to ReactFlow`);
        try {
            // Get the exact nodes and edges that ELK calculated for this visual state
            const result = await layoutVisualElements(allNodes, allEdges, visualState, currentLayout);
            console.log(`[ReactFlowStateManager] 🎯 ${operation.toUpperCase()}: Setting ${result.nodes.length} nodes, ${result.edges.length} edges`);
            // CRITICAL: Ensure ReactFlow sees exactly what ELK calculated
            setNodes(result.nodes);
            setEdges(result.edges);
            return result;
        }
        catch (error) {
            console.error(`[ReactFlowStateManager] ❌ ${operation.toUpperCase()} FAILED:`, error);
            throw error;
        }
    }
    /**
     * Update ReactFlow when container states change.
     * Automatically recalculates layout with ELK and ensures perfect synchronization.
     */
    async function updateContainerStates(visualState, allNodes, allEdges, currentLayout, operation = 'container-state-change') {
        return applyVisualState(visualState, allNodes, allEdges, currentLayout, operation);
    }
    /**
     * Update ReactFlow when layout algorithm changes.
     * Recalculates with new algorithm while preserving visual state.
     */
    async function updateLayout(visualState, allNodes, allEdges, newLayout, operation = 'layout-change') {
        console.log(`[ReactFlowStateManager] 🔄 LAYOUT_CHANGE: Using layout ${newLayout}`);
        // Apply the visual state with new layout
        return applyVisualState(visualState, allNodes, allEdges, newLayout, operation);
    }
    /**
     * Initialize ReactFlow with a visual state.
     * This should be called once during component initialization.
     */
    async function initializeReactFlow(visualState, allNodes, allEdges, currentLayout, operation = 'initialize') {
        console.log(`[ReactFlowStateManager] 🚀 INITIALIZE: Setting up ReactFlow with initial visual state`);
        return applyVisualState(visualState, allNodes, allEdges, currentLayout, operation);
    }
    /**
     * Force ReactFlow to refresh from visual state.
     * Useful when external data changes but visual state remains the same.
     */
    async function refreshReactFlow(visualState, allNodes, allEdges, currentLayout, operation = 'refresh') {
        console.log(`[ReactFlowStateManager] 🔄 REFRESH: Updating ReactFlow from visual state`);
        return applyVisualState(visualState, allNodes, allEdges, currentLayout, operation);
    }
    return {
        // Primary API - the only function most code should use
        applyVisualState,
        // Specialized wrappers for specific operations
        updateContainerStates,
        updateLayout,
        initializeReactFlow,
        refreshReactFlow,
        // Utility to check if state manager is ready
        isReady: () => Boolean(setNodes && setEdges)
    };
}
/**
 * Validation helper to ensure ReactFlow state matches expected visual state.
 * This can be used in development to catch synchronization issues.
 *
 * @param {Array} reactFlowNodes - Current ReactFlow nodes
 * @param {Array} reactFlowEdges - Current ReactFlow edges
 * @param {VisualState} visualState - Expected visual state
 * @param {Array} allNodes - Source nodes
 * @param {Array} allEdges - Source edges
 * @returns {Object} Validation result
 */
export function validateReactFlowState(reactFlowNodes, reactFlowEdges, visualState, allNodes, allEdges) {
    const issues = [];
    // Check that ReactFlow nodes match visual state expectations
    const expectedContainerStates = new Map();
    allNodes.forEach(node => {
        if (node.type === 'group' || node.type === 'collapsedContainer') {
            expectedContainerStates.set(node.id, visualState.getContainerState(node.id));
        }
    });
    // Validate container states in ReactFlow match visual state
    reactFlowNodes.forEach(node => {
        if (node.type === 'group' || node.type === 'collapsedContainer') {
            const expectedState = expectedContainerStates.get(node.id);
            const actualType = node.type;
            if (expectedState === 'collapsed' && actualType !== 'collapsedContainer') {
                issues.push(`Node ${node.id}: Expected collapsed container, got ${actualType}`);
            }
            else if (expectedState === 'expanded' && actualType !== 'group') {
                issues.push(`Node ${node.id}: Expected expanded group, got ${actualType}`);
            }
        }
    });
    // Check for hidden nodes that shouldn't be visible
    reactFlowNodes.forEach(node => {
        if (node.parentId) {
            const parentState = visualState.getContainerState(node.parentId);
            if (parentState === 'collapsed') {
                issues.push(`Node ${node.id}: Child of collapsed container ${node.parentId} should be hidden`);
            }
        }
        const nodeState = visualState.getNodeState(node.id);
        if (nodeState === 'hidden') {
            issues.push(`Node ${node.id}: Should be hidden according to visual state`);
        }
    });
    return {
        isValid: issues.length === 0,
        issues,
        summary: issues.length === 0
            ? 'ReactFlow state matches visual state'
            : `${issues.length} synchronization issues found`
    };
}
//# sourceMappingURL=reactFlowStateManager.js.map