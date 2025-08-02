/**
 * @fileoverview Runtime Validation Utilities
 * 
 * Simplified validation utilities for ReactFlow integration.
 */

import { LayoutResult } from '../layout/types.js';
import { TypedReactFlowData } from './types.js';

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
export function validateELKResult(layoutResult: any): ValidationReport {
  const report: ValidationReport = {
    isValid: true,
    errors: [],
    warnings: []
  };

  if (!layoutResult || typeof layoutResult !== 'object') {
    report.isValid = false;
    report.errors.push('Layout result is null or not an object');
    return report;
  }

  if (!Array.isArray(layoutResult.nodes)) {
    report.isValid = false;
    report.errors.push('Layout result missing nodes array');
  }

  if (!Array.isArray(layoutResult.edges)) {
    report.isValid = false;
    report.errors.push('Layout result missing edges array');
  }

  return report;
}

/**
 * Basic validation for ReactFlow data
 */
export function validateReactFlowResult(reactFlowData: any): ValidationReport {
  const report: ValidationReport = {
    isValid: true,
    errors: [],
    warnings: []
  };

  if (!reactFlowData || typeof reactFlowData !== 'object') {
    report.isValid = false;
    report.errors.push('ReactFlow data is null or not an object');
    return report;
  }

  if (!Array.isArray(reactFlowData.nodes)) {
    report.isValid = false;
    report.errors.push('ReactFlow data missing nodes array');
  }

  if (!Array.isArray(reactFlowData.edges)) {
    report.isValid = false;
    report.errors.push('ReactFlow data missing edges array');
  }

  return report;
}

/**
 * Log validation report
 */
export function logValidationReport(report: ValidationReport, stage: string): void {
  if (report.isValid) {
    console.log(`✅ ${stage} validation passed`);
  } else {
    console.error(`❌ ${stage} validation failed:`, report.errors);
  }
  
  if (report.warnings.length > 0) {
    console.warn(`⚠️ ${stage} warnings:`, report.warnings);
  }
}
