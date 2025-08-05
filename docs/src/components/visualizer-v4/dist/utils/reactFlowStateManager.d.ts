/**
 * Create a ReactFlow state manager that wraps all ReactFlow interactions
 * with VisualState as the single source of truth.
 *
 * @param {Function} setNodes - ReactFlow setNodes function
 * @param {Function} setEdges - ReactFlow setEdges function
 * @returns {Object} ReactFlow wrapper functions
 */
export function createReactFlowStateManager(setNodes: Function, setEdges: Function): any;
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
export function validateReactFlowState(reactFlowNodes: any[], reactFlowEdges: any[], visualState: VisualState, allNodes: any[], allEdges: any[]): any;
//# sourceMappingURL=reactFlowStateManager.d.ts.map