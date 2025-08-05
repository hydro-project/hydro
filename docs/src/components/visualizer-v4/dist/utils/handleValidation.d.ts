/**
 * Simple validation that checks handle ID requirements
 *
 * @param {Object} nodeTypes - ReactFlow nodeTypes object mapping
 * @returns {Object} Validation result with success flag and info
 */
export function validateHandleConsistency(nodeTypes: any): any;
/**
 * Log validation results (currently silent)
 */
export function logValidationResults(results: any): void;
/**
 * Validate handle consistency (silent validation)
 * Use this during development/initialization to maintain handle requirements
 */
export function enforceHandleConsistency(nodeTypes: any): any;
export namespace REQUIRED_HANDLE_IDS {
    let source: string;
    let target: string;
    let sourceBottom: string;
    let targetTop: string;
}
//# sourceMappingURL=handleValidation.d.ts.map