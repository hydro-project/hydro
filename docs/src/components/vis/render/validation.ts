/**
 * @fileoverview Runtime Validation Utilities
 * 
 * Utilities to validate data flow from ELK to ReactFlow at runtime,
 * ensuring type safety beyond compile-time checks.
 */

import { 
  StrongLayoutResult, 
  TypedReactFlowData, 
  ELKPositionedContainer,
  validateELKLayoutResult,
  validateReactFlowData,
  isValidELKContainer
} from './types';

/**
 * Comprehensive validation report
 */
export interface ValidationReport {
  isValid: boolean;
  errors: string[];
  warnings: string[];
  containerDimensionCheck: {
    totalContainers: number;
    validContainers: number;
    invalidContainers: string[];
  };
}

/**
 * Validates ELK layout result and provides detailed feedback
 */
export function validateELKResult(layoutResult: any): ValidationReport {
  const report: ValidationReport = {
    isValid: true,
    errors: [],
    warnings: [],
    containerDimensionCheck: {
      totalContainers: 0,
      validContainers: 0,
      invalidContainers: []
    }
  };

  // Basic structure validation
  if (!layoutResult || typeof layoutResult !== 'object') {
    report.isValid = false;
    report.errors.push('Layout result is null or not an object');
    return report;
  }

  if (!Array.isArray(layoutResult.containers)) {
    report.isValid = false;
    report.errors.push('Layout result missing containers array');
    return report;
  }

  // Container dimension validation
  report.containerDimensionCheck.totalContainers = layoutResult.containers.length;
  
  layoutResult.containers.forEach((container: any) => {
    if (isValidELKContainer(container)) {
      report.containerDimensionCheck.validContainers++;
      
      // Warn about very small containers
      if (container.width < 50 || container.height < 50) {
        report.warnings.push(`Container ${container.id} has very small dimensions: ${container.width}x${container.height}`);
      }
      
      // Warn about very large containers
      if (container.width > 2000 || container.height > 2000) {
        report.warnings.push(`Container ${container.id} has very large dimensions: ${container.width}x${container.height}`);
      }
    } else {
      report.isValid = false;
      report.containerDimensionCheck.invalidContainers.push(container.id || 'unknown');
      report.errors.push(`Container ${container.id || 'unknown'} missing required dimensions (x, y, width, height)`);
    }
  });

  // Node validation
  if (!Array.isArray(layoutResult.nodes)) {
    report.isValid = false;
    report.errors.push('Layout result missing nodes array');
  } else {
    layoutResult.nodes.forEach((node: any, index: number) => {
      if (!node.id || typeof node.x !== 'number' || typeof node.y !== 'number') {
        report.errors.push(`Node at index ${index} missing required positioning data`);
        report.isValid = false;
      }
    });
  }

  return report;
}

/**
 * Validates ReactFlow data after conversion
 */
export function validateReactFlowResult(reactFlowData: any): ValidationReport {
  const report: ValidationReport = {
    isValid: true,
    errors: [],
    warnings: [],
    containerDimensionCheck: {
      totalContainers: 0,
      validContainers: 0,
      invalidContainers: []
    }
  };

  if (!validateReactFlowData(reactFlowData)) {
    report.isValid = false;
    report.errors.push('ReactFlow data failed type validation');
    return report;
  }

  // Container-specific validation
  const containerNodes = reactFlowData.nodes.filter((node: any) => node.type === 'container');
  report.containerDimensionCheck.totalContainers = containerNodes.length;

  containerNodes.forEach((node: any) => {
    if (node.data && typeof node.data.width === 'number' && typeof node.data.height === 'number') {
      report.containerDimensionCheck.validContainers++;
      
      // Check data/style consistency
      if (node.style && (node.data.width !== node.style.width || node.data.height !== node.style.height)) {
        report.warnings.push(`Container ${node.id}: data dimensions (${node.data.width}x${node.data.height}) don't match style dimensions (${node.style.width}x${node.style.height})`);
      }
    } else {
      report.isValid = false;
      report.containerDimensionCheck.invalidContainers.push(node.id);
      report.errors.push(`Container node ${node.id} missing dimension data`);
    }
  });

  return report;
}

/**
 * Logs a detailed validation report
 */
export function logValidationReport(report: ValidationReport, stage: string): void {
  const emoji = report.isValid ? '✅' : '❌';
  console.group(`${emoji} ${stage} Validation Report`);
  
  if (report.errors.length > 0) {
    console.error('🔴 Errors:');
    report.errors.forEach(error => console.error(`  - ${error}`));
  }
  
  if (report.warnings.length > 0) {
    console.warn('🟡 Warnings:');
    report.warnings.forEach(warning => console.warn(`  - ${warning}`));
  }
  
  console.log('📊 Container Dimension Check:');
  console.log(`  Total: ${report.containerDimensionCheck.totalContainers}`);
  console.log(`  Valid: ${report.containerDimensionCheck.validContainers}`);
  
  if (report.containerDimensionCheck.invalidContainers.length > 0) {
    console.log(`  Invalid: ${report.containerDimensionCheck.invalidContainers.join(', ')}`);
  }
  
  console.groupEnd();
}

/**
 * Full pipeline validation from ELK to ReactFlow
 */
export function validateFullPipeline(elkResult: any, reactFlowResult: any): boolean {
  console.log('🔍 Running full pipeline validation...');
  
  const elkReport = validateELKResult(elkResult);
  logValidationReport(elkReport, 'ELK Layout');
  
  const reactFlowReport = validateReactFlowResult(reactFlowResult);
  logValidationReport(reactFlowReport, 'ReactFlow Conversion');
  
  const isValid = elkReport.isValid && reactFlowReport.isValid;
  
  if (isValid) {
    console.log('🎉 Full pipeline validation passed!');
  } else {
    console.error('💥 Pipeline validation failed - check errors above');
  }
  
  return isValid;
}
