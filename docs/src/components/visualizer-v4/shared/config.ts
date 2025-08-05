/**
 * @fileoverview Basic configuration exports
 * 
 * This is a simplified config file that imports what's needed from the current implementation.
 */

// For now, just re-export constants
export * from './constants';

// Additional exports expected by components
export const COMPONENT_COLORS = {
  BACKGROUND_PRIMARY: '#ffffff',
  BACKGROUND_SECONDARY: '#f9fafb',
  PANEL_BACKGROUND: '#ffffff',
  PANEL_HEADER_BACKGROUND: '#f9fafb',
  BORDER_LIGHT: '#e5e7eb',
  BORDER_MEDIUM: '#d1d5db',
  TEXT_PRIMARY: '#111827',
  TEXT_SECONDARY: '#6b7280',
  TEXT_TERTIARY: '#9ca3af',
  TEXT_DISABLED: '#d1d5db',
  BUTTON_HOVER_BACKGROUND: '#f3f4f6'
};

export const COLOR_PALETTES = {
  Set3: [
    { primary: '#8dd3c7', secondary: '#ffffb3', name: 'Light Teal' },
    { primary: '#bebada', secondary: '#fb8072', name: 'Light Purple' },
    { primary: '#80b1d3', secondary: '#fdb462', name: 'Light Blue' },
    { primary: '#fccde5', secondary: '#b3de69', name: 'Light Pink' },
    { primary: '#d9d9d9', secondary: '#fccde5', name: 'Light Gray' },
    { primary: '#bc80bd', secondary: '#ccebc5', name: 'Medium Purple' },
    { primary: '#ccebc5', secondary: '#ffed6f', name: 'Light Green' },
    { primary: '#ffed6f', secondary: '#8dd3c7', name: 'Light Yellow' },
  ]
};

export const SIZES = {
  SMALL: 'small',
  MEDIUM: 'medium',
  LARGE: 'large',
  BORDER_RADIUS_DEFAULT: '6px',
  COLLAPSED_CONTAINER_WIDTH: 200,
  COLLAPSED_CONTAINER_HEIGHT: 100
};

export const SHADOWS = {
  LIGHT: '0 1px 3px 0 rgba(0, 0, 0, 0.1)',
  MEDIUM: '0 4px 6px -1px rgba(0, 0, 0, 0.1)',
  LARGE: '0 10px 15px -3px rgba(0, 0, 0, 0.1)',
  PANEL_DEFAULT: '0 1px 3px 0 rgba(0, 0, 0, 0.1)',
  PANEL_DRAGGING: '0 10px 15px -3px rgba(0, 0, 0, 0.1)'
};

// ELK Layout exports expected by ELKStateManager
export const ELK_ALGORITHMS = {
  MRTREE: 'mrtree',
  LAYERED: 'layered',
  FORCE: 'force',
  STRESS: 'stress',
  RADIAL: 'radial'
};

export const LAYOUT_SPACING = {
  // Updated to match working Visualizer spacing values
  NODE_NODE: 75,                    // Increased for better node separation
  NODE_EDGE: 10,                    // Keep edge spacing tight
  EDGE_EDGE: 10,                    // Keep edge spacing tight
  NODE_TO_NODE_NORMAL: 75,          // Match Visualizer: better readability
  EDGE_TO_EDGE: 10,                 // Keep edge spacing tight
  EDGE_TO_NODE: 0,                  // Match Visualizer: no extra edge-node gap
  COMPONENT_TO_COMPONENT: 60,       // Match Visualizer: better component separation
  ROOT_PADDING: 20,                 // Keep root padding minimal
  CONTAINER_PADDING: 60             // Match Visualizer: proper breathing room in containers
};

export const ELK_LAYOUT_OPTIONS = {
  'elk.algorithm': 'mrtree',
  'elk.direction': 'DOWN',
  'elk.hierarchyHandling': 'INCLUDE_CHILDREN',    // Added: maintain visual hierarchy
  'elk.spacing.nodeNode': LAYOUT_SPACING.NODE_TO_NODE_NORMAL.toString(),
  'elk.spacing.edgeNode': LAYOUT_SPACING.EDGE_TO_NODE.toString(),
  'elk.spacing.edgeEdge': LAYOUT_SPACING.EDGE_TO_EDGE.toString(),
  'elk.spacing.componentComponent': LAYOUT_SPACING.COMPONENT_TO_COMPONENT.toString(),
  'elk.layered.spacing.nodeNodeBetweenLayers': '25'  // Match Visualizer layer separation
};

export type ELKAlgorithm = typeof ELK_ALGORITHMS[keyof typeof ELK_ALGORITHMS];

export function getELKLayoutOptions(algorithm: ELKAlgorithm = ELK_ALGORITHMS.MRTREE) {
  return {
    ...ELK_LAYOUT_OPTIONS,
    'elk.algorithm': algorithm
  };
}

export function createFixedPositionOptions(x?: number, y?: number) {
  const options = {
    ...ELK_LAYOUT_OPTIONS,
    'elk.position': 'FIXED'
  };
  
  if (x !== undefined && y !== undefined) {
    return {
      ...options,
      'elk.position.x': x.toString(),
      'elk.position.y': y.toString()
    };
  }
  
  return options;
}

export function createFreePositionOptions() {
  return {
    ...ELK_LAYOUT_OPTIONS,
    'elk.position': 'FREE'
  };
}
