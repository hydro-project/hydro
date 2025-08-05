/**
 * Create an ELK state manager that wraps all ELK layout interactions
 * with VisualState as the single source of truth.
 *
 * @returns {Object} ELK wrapper functions
 */
export function createELKStateManager(): any;
/**
 * Validation helper to ensure ELK input matches expected visual state.
 * This can be used in development to catch input preparation issues.
 *
 * @param {Array} elkNodes - ELK input nodes
 * @param {Array} elkEdges - ELK input edges
 * @param {VisualState} visualState - Expected visual state
 * @returns {Object} Validation result
 */
export function validateELKInput(elkNodes: any[], elkEdges: any[], visualState: VisualState): any;
//# sourceMappingURL=elkStateManager.d.ts.map