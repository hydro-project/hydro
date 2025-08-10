/**
 * @fileoverview Unified Configuration and Constants
 * 
 * All configuration constants, styling, colors, typography, and layout settings
 * for the visualizer-v4 system. This replaces the previous split between
 * config.ts and constants.ts for better organization.
 */

// ============================================================================
// STYLING CONSTANTS (from constants.ts)
// ============================================================================

// Node styling constants
export const NODE_STYLES = {
  DEFAULT: 'default',
  HIGHLIGHTED: 'highlighted',
  SELECTED: 'selected',
  WARNING: 'warning',
  ERROR: 'error'
} as const;

// Edge styling constants
export const EDGE_STYLES = {
  DEFAULT: 'default',
  HIGHLIGHTED: 'highlighted',
  DASHED: 'dashed',
  THICK: 'thick',
  WARNING: 'warning'
} as const;

// Container styling constants
export const CONTAINER_STYLES = {
  DEFAULT: 'default',
  HIGHLIGHTED: 'highlighted',
  SELECTED: 'selected',
  MINIMIZED: 'minimized'
} as const;

// Layout constants
export const LAYOUT_CONSTANTS = {
  DEFAULT_NODE_WIDTH: 180,
  DEFAULT_NODE_HEIGHT: 60,
  DEFAULT_CONTAINER_PADDING: 20,
  MIN_CONTAINER_WIDTH: 200,
  MIN_CONTAINER_HEIGHT: 150,
  
  // Container label positioning and sizing
  CONTAINER_LABEL_HEIGHT: 32,           // Height reserved for container labels
  CONTAINER_LABEL_PADDING: 12,          // Padding around container labels
  CONTAINER_LABEL_FONT_SIZE: 12,        // Font size for container labels
  
  // Validation and warning thresholds
  MAX_HYPEREDGE_WARNINGS: 10,           // Maximum hyperEdge warnings before summary
  
  // Large container detection thresholds (derived from base constants)
  get LARGE_CONTAINER_WIDTH_THRESHOLD() { 
    return this.MIN_CONTAINER_WIDTH * 1.5; // 300 (1.5x minimum width)
  },
  get LARGE_CONTAINER_HEIGHT_THRESHOLD() { 
    return this.MIN_CONTAINER_HEIGHT + this.CONTAINER_LABEL_HEIGHT; // 182 (min height + label)
  },
} as const;

// HyperEdge constants
export const HYPEREDGE_CONSTANTS = {
  PREFIX: 'hyper_',                     // Prefix for hyperEdge IDs
  SEPARATOR: '_to_',                    // Separator for hyperEdge naming
} as const;

// Type exports
export type NodeStyle = typeof NODE_STYLES[keyof typeof NODE_STYLES];
export type EdgeStyle = typeof EDGE_STYLES[keyof typeof EDGE_STYLES];
export type ContainerStyle = typeof CONTAINER_STYLES[keyof typeof CONTAINER_STYLES];

// ============================================================================
// UI CONFIGURATION
// ============================================================================

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
  ],
  Pastel1: [
    { primary: '#fbb4ae', secondary: '#b3cde3', name: 'Soft Red' },
    { primary: '#b3cde3', secondary: '#ccebc5', name: 'Soft Blue' },
    { primary: '#ccebc5', secondary: '#decbe4', name: 'Soft Green' },
    { primary: '#decbe4', secondary: '#fed9a6', name: 'Soft Lavender' },
    { primary: '#fed9a6', secondary: '#ffffcc', name: 'Soft Orange' },
    { primary: '#ffffcc', secondary: '#e5d8bd', name: 'Soft Yellow' },
    { primary: '#e5d8bd', secondary: '#fddaec', name: 'Soft Beige' },
    { primary: '#fddaec', secondary: '#f2f2f2', name: 'Soft Pink' },
  ],
  Dark2: [
    { primary: '#1b9e77', secondary: '#d95f02', name: 'Dark Teal' },
    { primary: '#d95f02', secondary: '#7570b3', name: 'Dark Orange' },
    { primary: '#7570b3', secondary: '#e7298a', name: 'Dark Purple' },
    { primary: '#e7298a', secondary: '#66a61e', name: 'Dark Pink' },
    { primary: '#66a61e', secondary: '#e6ab02', name: 'Dark Green' },
    { primary: '#e6ab02', secondary: '#a6761d', name: 'Dark Gold' },
    { primary: '#a6761d', secondary: '#666666', name: 'Dark Brown' },
    { primary: '#666666', secondary: '#1b9e77', name: 'Dark Gray' },
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

// Typography and font size constants
export const TYPOGRAPHY = {
  // InfoPanel font sizes - increased from tiny sizes for better readability
  INFOPANEL_BASE: '14px',           // Main InfoPanel content (was 10px)
  INFOPANEL_TITLE: '16px',          // Section titles
  INFOPANEL_HIERARCHY_NODE: '13px', // Hierarchy tree nodes (was 9-10px)
  INFOPANEL_HIERARCHY_DETAILS: '12px', // Node details and counts (was 9px)
  INFOPANEL_LEGEND: '13px',         // Legend items (was 10-11px)
  
  // General UI font sizes
  UI_SMALL: '12px',
  UI_MEDIUM: '14px',
  UI_LARGE: '16px',
  UI_HEADING: '18px',
  
  // Page-level typography
  PAGE_TITLE: '24px',
  PAGE_SUBTITLE: '14px',
  BUTTON_SMALL: '14px',
  BUTTON_MEDIUM: '16px'
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
  'elk.layered.spacing.nodeNodeBetweenLayers': '25',  // Match Visualizer layer separation
  
  // CRITICAL: Force ELK to respect container dimensions (prevents size explosion)
  'elk.nodeSize.constraints': 'FIXED_SIZE',           // Don't resize containers to fit content
  'elk.nodeSize.options': 'DEFAULT_MINIMUM_SIZE'      // Respect our specified dimensions
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
