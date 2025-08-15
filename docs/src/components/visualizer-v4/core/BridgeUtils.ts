/**
 * @fileoverview Bridge Utilities - Common functionality for bridges
 * 
 * Provides pure utility functions that can be shared between bridges
 * to eliminate code duplication and maintain consistency.
 */

/**
 * Validates that a coordinate is a valid number
 */
export function isValidCoordinate(value: any): value is number {
  return typeof value === 'number' && !isNaN(value) && isFinite(value);
}

/**
 * Validates and cleans coordinate values, providing fallbacks
 */
export function validateCoordinate(value: any, fallback: number = 0): number {
  return isValidCoordinate(value) ? value : fallback;
}

/**
 * Validates dimension values (must be positive)
 */
export function isValidDimension(value: any): value is number {
  return typeof value === 'number' && !isNaN(value) && isFinite(value) && value > 0;
}

/**
 * Validates and cleans dimension values, providing fallbacks
 */
export function validateDimension(value: any, fallback: number): number {
  return isValidDimension(value) ? value : fallback;
}

/**
 * Extract custom properties from graph elements
 * Filters out known properties to get only custom ones
 */
export function extractCustomProperties(element: any): Record<string, any> {
  const customProps: Record<string, any> = {};
  
  // Filter out known properties to get custom ones
  const knownProps = new Set([
    'id', 'label', 'style', 'hidden', 'layout', 
    'source', 'target', 'children', 'collapsed',
    'x', 'y', 'width', 'height', 'containerId'
  ]);
  
  Object.entries(element).forEach(([key, value]) => {
    if (!knownProps.has(key)) {
      customProps[key] = value;
    }
  });
  
  return customProps;
}

/**
 * Creates a layout update object with validated coordinates and dimensions
 */
export function createLayoutUpdate(data: {
  x?: any;
  y?: any;
  width?: any;
  height?: any;
}): any {
  const layoutUpdates: any = {};
  
  // Validate and set position
  if (data.x !== undefined || data.y !== undefined) {
    layoutUpdates.position = {};
    
    if (data.x !== undefined) {
      layoutUpdates.position.x = validateCoordinate(data.x, 0);
    }
    
    if (data.y !== undefined) {
      layoutUpdates.position.y = validateCoordinate(data.y, 0);
    }
  }
  
  // Validate and set dimensions
  if (data.width !== undefined || data.height !== undefined) {
    layoutUpdates.dimensions = {};
    
    if (data.width !== undefined) {
      layoutUpdates.dimensions.width = validateDimension(data.width, 200);
    }
    
    if (data.height !== undefined) {
      layoutUpdates.dimensions.height = validateDimension(data.height, 150);
    }
  }
  
  return layoutUpdates;
}