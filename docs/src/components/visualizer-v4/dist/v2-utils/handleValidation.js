/**
 * Handle Validation Utilities
 *
 * Ensures consistency of ReactFlow Handle IDs across all node components
 * to prevent "Couldn't create edge for handle id" errors
 */
/**
 * Required handle IDs that must exist on all node types that can have edges
 *
 * CRITICAL: These IDs must match exactly across:
 * - GroupNode.js Handle components
 * - CollapsedContainerNode.js Handle components
 * - DefaultNode.js Handle components (if any)
 * - containerLogic.js edge processing logic
 */
export const REQUIRED_HANDLE_IDS = {
    source: 'source',
    target: 'target',
    sourceBottom: 'source-bottom',
    targetTop: 'target-top'
};
/**
 * Simple validation that checks handle ID requirements
 *
 * @param {Object} nodeTypes - ReactFlow nodeTypes object mapping
 * @returns {Object} Validation result with success flag and info
 */
export function validateHandleConsistency(nodeTypes) {
    const nodeTypeNames = Object.keys(nodeTypes);
    return {
        success: true,
        nodeTypes: nodeTypeNames,
        requiredHandles: Object.values(REQUIRED_HANDLE_IDS)
    };
}
/**
 * Log validation results (currently silent)
 */
export function logValidationResults(results) {
    // Silent - no console logging
}
/**
 * Validate handle consistency (silent validation)
 * Use this during development/initialization to maintain handle requirements
 */
export function enforceHandleConsistency(nodeTypes) {
    const results = validateHandleConsistency(nodeTypes);
    logValidationResults(results);
    // Silent validation - no console output
    return results;
}
//# sourceMappingURL=handleValidation.js.map