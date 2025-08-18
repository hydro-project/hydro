/**
 * @fileoverview Runtime Validation Utilities
 *
 * Simplified validation utilities for ReactFlow integration.
 */
/**
 * Simple validation report
 */
export interface ValidationReport {
    isValid: boolean;
    errors: string[];
    warnings: string[];
}
/**
 * Basic validation for layout results
 */
export declare function validateELKResult(layoutResult: any): ValidationReport;
/**
 * Basic validation for ReactFlow data
 */
export declare function validateReactFlowResult(reactFlowData: any): ValidationReport;
/**
 * Log validation report
 */
export declare function logValidationReport(report: ValidationReport, stage: string): void;
//# sourceMappingURL=validation.d.ts.map