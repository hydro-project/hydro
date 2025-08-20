/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * ValidationWrapper - Provides before/after validation for public API methods
 * 
 * This approach moves validation out of internal methods and into public API boundaries,
 * reducing noise and making validation more predictable.
 */

export interface ValidationConfig {
  validateBefore?: boolean;
  validateAfter?: boolean;
  skipValidation?: boolean;
}

/**
 * Default validation configuration for public APIs
 */
const DEFAULT_CONFIG: ValidationConfig = {
  validateBefore: false,  // Usually not needed - assume valid state
  validateAfter: true,    // Validate that our changes maintain invariants
  skipValidation: false
};

/**
 * Wraps a method with before/after validation
 */
export function withValidation<T extends any[], R>(
  instance: any,
  methodName: string,
  originalMethod: (...args: T) => R,
  config: ValidationConfig = DEFAULT_CONFIG
): (...args: T) => R {
  
  return function(this: any, ...args: T): R {
    const shouldValidate = instance._validationEnabled && !config.skipValidation;
    
    // Before validation (optional)
    if (shouldValidate && config.validateBefore) {
      try {
        instance.validateInvariants();
      } catch (error) {
        console.error(`[ValidationWrapper] Pre-validation failed for ${methodName}:`, error);
        throw error;
      }
    }
    
    // Execute the original method
    let result: R;
    try {
      result = originalMethod.apply(this, args);
    } catch (error) {
      console.error(`[ValidationWrapper] Method ${methodName} failed:`, error);
      throw error;
    }
    
    // After validation (default for most public APIs)
    if (shouldValidate && config.validateAfter) {
      try {
        instance.validateInvariants();
      } catch (error) {
        console.error(`[ValidationWrapper] Post-validation failed for ${methodName}:`, error);
        throw error;
      }
    }
    
    return result;
  };
}

/**
 * Specific validation configurations for different types of methods
 */
export const ValidationConfigs = {
  // Read-only getters - no validation needed
  GETTER: {
    validateBefore: false,
    validateAfter: false,
    skipValidation: false
  } as ValidationConfig,
  
  // State mutation methods - validate after
  MUTATOR: {
    validateBefore: false,
    validateAfter: true,
    skipValidation: false
  } as ValidationConfig,
  
  // Critical operations - validate before and after
  CRITICAL: {
    validateBefore: true,
    validateAfter: true,
    skipValidation: false
  } as ValidationConfig,
  
  // Internal/performance-sensitive operations - skip validation
  INTERNAL: {
    validateBefore: false,
    validateAfter: false,
    skipValidation: true
  } as ValidationConfig
};

/**
 * Helper to wrap multiple methods at once
 */
export function wrapPublicMethods(instance: any, methodConfigs: Record<string, ValidationConfig>) {
  for (const [methodName, config] of Object.entries(methodConfigs)) {
    const originalMethod = instance[methodName];
    if (typeof originalMethod === 'function') {
      instance[methodName] = withValidation(instance, methodName, originalMethod.bind(instance), config);
    }
  }
}
