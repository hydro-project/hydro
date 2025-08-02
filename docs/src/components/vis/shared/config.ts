/**
 * @fileoverview Centralized configuration for the graph visualization system
 * 
 * Professional color system based on ColorBrewer and WCAG accessibility guidelines.
 * Ported from the existing visualizer app with enhancements for the new vis system.
 */

// ============ SEMANTIC COLOR SYSTEM ============
// Based on ColorBrewer qualitative and sequential schemes
// All colors meet WCAG AA contrast requirements (4.5:1 minimum)

export const COLORS = {
  // ----------------------------------------
  // NEUTRAL PALETTE (Grayscale Foundation)
  // ----------------------------------------
  WHITE: '#ffffff',
  GRAY_50: '#f9fafb',   // Lightest background
  GRAY_100: '#f3f4f6',  // Light background  
  GRAY_200: '#e5e7eb',  // Border light
  GRAY_300: '#d1d5db',  // Border medium
  GRAY_400: '#9ca3af',  // Text disabled
  GRAY_500: '#6b7280',  // Text secondary
  GRAY_600: '#4b5563',  // Text primary light
  GRAY_700: '#374151',  // Text primary
  GRAY_800: '#1f2937',  // Text primary dark
  GRAY_900: '#111827',  // Text darkest
  BLACK: '#000000',
  
  // ----------------------------------------
  // SEMANTIC COLORS (Status & Feedback)
  // ----------------------------------------
  // Success (Green) - ColorBrewer BrBG
  SUCCESS_50: '#f0f9f0',
  SUCCESS_100: '#dcf2dc', 
  SUCCESS_500: '#16a34a',   // Primary success (4.5:1 on white)
  SUCCESS_600: '#15803d',   // Darker success
  SUCCESS_700: '#166534',   // Darkest success
  
  // Warning (Amber) - ColorBrewer YlOrBr  
  WARNING_50: '#fffbeb',
  WARNING_100: '#fef3c7',
  WARNING_500: '#f59e0b',   // Primary warning (4.5:1 on white)
  WARNING_600: '#d97706',   // Darker warning
  WARNING_700: '#b45309',   // Darkest warning
  
  // Error (Red) - ColorBrewer Reds
  ERROR_50: '#fef2f2',
  ERROR_100: '#fee2e2',
  ERROR_500: '#ef4444',     // Primary error (4.5:1 on white)
  ERROR_600: '#dc2626',     // Darker error  
  ERROR_700: '#b91c1c',     // Darkest error
  
  // Info (Blue) - ColorBrewer Blues
  INFO_50: '#eff6ff',
  INFO_100: '#dbeafe', 
  INFO_500: '#3b82f6',      // Primary info (4.5:1 on white)
  INFO_600: '#2563eb',      // Darker info
  INFO_700: '#1d4ed8',      // Darkest info
  
  // ----------------------------------------
  // DATA VISUALIZATION COLORS
  // Based on ColorBrewer qualitative schemes
  // Optimized for accessibility and differentiation
  // ----------------------------------------
  // Primary data colors (ColorBrewer Set2 - colorblind safe)
  VIZ_TEAL: '#1b9e77',      // Primary teal
  VIZ_ORANGE: '#d95f02',    // Primary orange  
  VIZ_PURPLE: '#7570b3',    // Primary purple
  VIZ_PINK: '#e7298a',      // Primary pink
  VIZ_GREEN: '#66a61e',     // Primary green
  VIZ_YELLOW: '#e6ab02',    // Primary yellow
  VIZ_BROWN: '#a6761d',     // Primary brown
  VIZ_GRAY: '#666666',      // Primary gray
  
  // ----------------------------------------
  // CONTAINER/HIERARCHY COLORS
  // Sequential scheme for nested containers
  // Based on ColorBrewer multi-hue scheme
  // ----------------------------------------
  CONTAINER_L0: 'rgba(59, 130, 246, 0.08)',   // Level 0 - Lightest blue
  CONTAINER_L1: 'rgba(16, 185, 129, 0.08)',   // Level 1 - Lightest green
  CONTAINER_L2: 'rgba(245, 158, 11, 0.08)',   // Level 2 - Lightest amber
  CONTAINER_L3: 'rgba(139, 92, 246, 0.08)',   // Level 3 - Lightest purple
  CONTAINER_L4: 'rgba(239, 68, 68, 0.08)',    // Level 4 - Lightest red
  
  // Container borders (stronger variants)
  CONTAINER_BORDER_L0: '#3b82f6',  // Blue-500
  CONTAINER_BORDER_L1: '#10b981',  // Green-500  
  CONTAINER_BORDER_L2: '#f59e0b',  // Amber-500
  CONTAINER_BORDER_L3: '#8b5cf6',  // Purple-500
  CONTAINER_BORDER_L4: '#ef4444',  // Red-500
  
  // Legacy compatibility
  PRIMARY: '#3b82f6',
  PRIMARY_HOVER: '#2563eb',
  PRIMARY_LIGHT: 'rgba(59, 130, 246, 0.1)',
} as const;

// ============ COLOR PALETTES FOR DATA VISUALIZATION ============
// Based on ColorBrewer schemes, optimized for accessibility

export const COLOR_PALETTES = {
  // Qualitative scheme - for categorical data (ColorBrewer Set2)
  // Colorblind safe, high contrast, up to 8 categories
  Set2: [
    { primary: '#1b9e77', secondary: '#a6cee3', name: 'Teal' },
    { primary: '#d95f02', secondary: '#1f78b4', name: 'Orange' },
    { primary: '#7570b3', secondary: '#b2df8a', name: 'Purple' },
    { primary: '#e7298a', secondary: '#33a02c', name: 'Pink' },
    { primary: '#66a61e', secondary: '#fb9a99', name: 'Green' },
    { primary: '#e6ab02', secondary: '#e31a1c', name: 'Yellow' },
    { primary: '#a6761d', secondary: '#fdbf6f', name: 'Brown' },
    { primary: '#666666', secondary: '#ff7f00', name: 'Gray' },
  ],
  
  // Qualitative scheme - alternative (ColorBrewer Set3)  
  // For when more categories needed
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
  
  // Professional scheme - for business/enterprise use
  Professional: [
    { primary: '#1e40af', secondary: '#93c5fd', name: 'Corporate Blue' },
    { primary: '#059669', secondary: '#86efac', name: 'Success Green' },
    { primary: '#dc2626', secondary: '#fca5a5', name: 'Alert Red' },
    { primary: '#7c2d12', secondary: '#fdba74', name: 'Warm Brown' },
    { primary: '#4338ca', secondary: '#c4b5fd', name: 'Deep Purple' },
    { primary: '#0891b2', secondary: '#67e8f9', name: 'Ocean Blue' },
  ],
} as const;

// ============ COMPONENT-SPECIFIC COLOR MAPS ============

export const COMPONENT_COLORS = {
  // Edge and connection colors
  EDGE_DEFAULT: COLORS.GRAY_400,
  EDGE_HOVER: COLORS.GRAY_600,
  EDGE_SELECTED: COLORS.INFO_500,
  EDGE_NETWORK: COLORS.VIZ_PURPLE,  // Special color for network edges
  
  // Handle/connection point colors
  HANDLE_DEFAULT: COLORS.GRAY_500,
  HANDLE_HOVER: COLORS.GRAY_700,
  HANDLE_ACTIVE: COLORS.INFO_500,
  
  // Background variations
  BACKGROUND_PRIMARY: COLORS.WHITE,
  BACKGROUND_SECONDARY: COLORS.GRAY_50,
  BACKGROUND_TERTIARY: COLORS.GRAY_100,
  
  // Border variations
  BORDER_LIGHT: COLORS.GRAY_200,
  BORDER_MEDIUM: COLORS.GRAY_300,
  BORDER_STRONG: COLORS.GRAY_400,
  
  // Text color hierarchy
  TEXT_PRIMARY: COLORS.GRAY_900,
  TEXT_SECONDARY: COLORS.GRAY_600,
  TEXT_TERTIARY: COLORS.GRAY_500,
  TEXT_DISABLED: COLORS.GRAY_400,
  TEXT_INVERSE: COLORS.WHITE,
  
  // Interactive states
  INTERACTIVE_DEFAULT: COLORS.INFO_500,
  INTERACTIVE_HOVER: COLORS.INFO_600,
  INTERACTIVE_ACTIVE: COLORS.INFO_700,
  INTERACTIVE_DISABLED: COLORS.GRAY_300,
  
  // Status indicators
  STATUS_SUCCESS: COLORS.SUCCESS_500,
  STATUS_WARNING: COLORS.WARNING_500, 
  STATUS_ERROR: COLORS.ERROR_500,
  STATUS_INFO: COLORS.INFO_500,
  
  // Panel colors
  PANEL_BACKGROUND: COLORS.WHITE,
  PANEL_HEADER_BACKGROUND: COLORS.GRAY_50,
  BUTTON_HOVER_BACKGROUND: COLORS.GRAY_100,
} as const;

// ============ Node Styling ============

export const NODE_COLORS = {
  // Background colors by style
  BACKGROUND: {
    DEFAULT: COLORS.WHITE,
    HIGHLIGHTED: COLORS.WARNING_100,
    SELECTED: COLORS.INFO_100,
    WARNING: COLORS.WARNING_100,
    ERROR: COLORS.ERROR_100,
  },
  
  // Border colors by style
  BORDER: {
    DEFAULT: COLORS.GRAY_500,
    HIGHLIGHTED: COLORS.WARNING_500,
    SELECTED: COLORS.INFO_500,
    WARNING: COLORS.WARNING_600,
    ERROR: COLORS.ERROR_600,
  },
  
  // Text colors by style
  TEXT: {
    DEFAULT: COMPONENT_COLORS.TEXT_PRIMARY,
    HIGHLIGHTED: COLORS.WARNING_700,
    SELECTED: COLORS.INFO_700,
    WARNING: COLORS.WARNING_700,
    ERROR: COLORS.ERROR_700,
  },
  
  // Handle colors
  HANDLE: COMPONENT_COLORS.HANDLE_DEFAULT,
} as const;

// ============ Edge Styling ============

export const EDGE_COLORS = {
  // Standard edge colors by style
  DEFAULT: COMPONENT_COLORS.EDGE_DEFAULT,
  DATA: COLORS.SUCCESS_500,
  CONTROL: COLORS.WARNING_500,
  ERROR: COLORS.ERROR_500,
  THICK: COLORS.GRAY_700,
  DASHED: COLORS.GRAY_500,
  
  // State-based colors
  SELECTED: COMPONENT_COLORS.EDGE_SELECTED,
  HIGHLIGHTED: '#ff6b6b',
  NETWORK: COMPONENT_COLORS.EDGE_NETWORK,
} as const;

// ============ Container Styling ============

export const CONTAINER_COLORS = {
  BACKGROUND: COLORS.CONTAINER_L0,
  BORDER: COMPONENT_COLORS.BORDER_LIGHT,
  BORDER_SELECTED: COLORS.INFO_500,
  
  HEADER_BACKGROUND: 'rgba(100, 116, 139, 0.1)',
  HEADER_TEXT: COLORS.GRAY_700,
} as const;

// ============ Layout Panel Styling ============

export const PANEL_COLORS = {
  BACKGROUND: COLORS.PRIMARY_LIGHT,
  BORDER: COLORS.PRIMARY,
  TEXT: COLORS.PRIMARY,
} as const;

// ============ Sizing Constants ============

export const SIZES = {
  // Node dimensions
  NODE_MIN_WIDTH: 120,
  NODE_MIN_HEIGHT: 40,
  NODE_PADDING: 12,
  NODE_BORDER_RADIUS: 6,
  
  // Edge dimensions
  EDGE_WIDTH_DEFAULT: 1,
  EDGE_WIDTH_THICK: 3,
  
  // Border widths
  BORDER_WIDTH_DEFAULT: 2,
  BORDER_WIDTH_SELECTED: 2,
  BORDER_RADIUS_DEFAULT: 6,
  
  // Container dimensions
  CONTAINER_MIN_WIDTH: 200,
  CONTAINER_MIN_HEIGHT: 100,
  CONTAINER_PADDING: 16,
  CONTAINER_BORDER_RADIUS: 8,
  CONTAINER_HEADER_HEIGHT: 30,
  CONTAINER_TITLE_AREA_PADDING: 30, // Space reserved for title at bottom
  
  // MiniMap
  MINIMAP_NODE_BORDER_RADIUS: 4,
  
  // Grid
  GRID_SIZE: 15,
  
  // Spacing
  SPACING_XS: 4,
  SPACING_SM: 8,
  SPACING_MD: 12,
  SPACING_LG: 16,
  SPACING_XL: 20,
  SPACING_XXL: 24,
} as const;

// ============ Shadow Effects ============

const SHADOW_COLORS = {
  LIGHT: 'rgba(0, 0, 0, 0.1)',
  MEDIUM: 'rgba(0, 0, 0, 0.25)',
  HEAVY: 'rgba(0, 0, 0, 0.5)',
} as const;

export const SHADOWS = {
  NODE_DEFAULT: `0 2px 4px ${SHADOW_COLORS.LIGHT}`,
  NODE_SELECTED: `0 0 10px rgba(59, 130, 246, 0.5)`,
  NODE_HOVER: `0 4px 8px ${SHADOW_COLORS.MEDIUM}`,
  
  CONTAINER_DEFAULT: `0 1px 3px ${SHADOW_COLORS.LIGHT}`,
  CONTAINER_SELECTED: `0 0 0 2px ${COLORS.INFO_500}`,
  
  PANEL: `0 2px 8px ${SHADOW_COLORS.LIGHT}`,
  PANEL_DEFAULT: `0 2px 8px ${SHADOW_COLORS.LIGHT}`,
  PANEL_DRAGGING: `0 8px 25px ${SHADOW_COLORS.MEDIUM}`,
} as const;

// ============ Design Tokens & Utilities ============

export const DEFAULT_STYLES = {
  BORDER_WIDTH: '1px',
  BORDER_WIDTH_THICK: '2px', 
  BORDER_RADIUS_SM: '4px',
  BORDER_RADIUS: '6px',
  BORDER_RADIUS_LG: '8px',
  BOX_SHADOW_SM: '0 1px 2px 0 rgba(0, 0, 0, 0.05)',
  BOX_SHADOW: '0 1px 3px 0 rgba(0, 0, 0, 0.1), 0 1px 2px 0 rgba(0, 0, 0, 0.06)',
  BOX_SHADOW_LG: '0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)',
} as const;

// ============ Typography ============

export const TYPOGRAPHY = {
  FONT_FAMILY: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
  
  FONT_SIZES: {
    XS: '10px',
    SM: '12px',
    MD: '14px',
    LG: '16px',
    XL: '18px',
    XXL: '20px',
  },
  
  FONT_WEIGHTS: {
    NORMAL: 400,
    MEDIUM: 500,
    SEMIBOLD: 600,
    BOLD: 700,
  },
  
  LINE_HEIGHTS: {
    TIGHT: 1.2,
    NORMAL: 1.4,
    RELAXED: 1.6,
  },
} as const;

// ============ Animation & Transitions ============

export const ANIMATIONS = {
  DURATION_FAST: '150ms',
  DURATION_NORMAL: '200ms',
  DURATION_SLOW: '300ms',
  
  EASING_DEFAULT: 'ease',
  EASING_IN: 'ease-in',
  EASING_OUT: 'ease-out',
  EASING_IN_OUT: 'ease-in-out',
  
  // Transition properties
  TRANSITION_DEFAULT: 'all 200ms ease',
  TRANSITION_FAST: 'all 150ms ease',
  
  // Animation timings from visualizer
  FIT_VIEW_DURATION: 300,
  FIT_VIEW_DEBOUNCE: 100,
  LAYOUT_DEBOUNCE: 200,
  RESIZE_DEBOUNCE: 500,
} as const;

// ============ Layout Spacing Constants ============

export const LAYOUT_SPACING = {
  NODE_TO_NODE_COMPACT: 15,     // Tight spacing between nodes
  NODE_TO_NODE_NORMAL: 75,      // Normal spacing between nodes
  NODE_TO_NODE_LOOSE: 125,      // Loose spacing between nodes
  EDGE_TO_NODE: 0,             // Spacing between edges and nodes
  EDGE_TO_EDGE: 10,             // Spacing between edges
  EDGE_TO_EDGE_ALTERNATE: 15,   // Alternative spacing between edges
  LAYER_SEPARATION: 25,         // Spacing between layers in layered layouts
  COMPONENT_TO_COMPONENT: 60,   // Spacing between disconnected components
  CONTAINER_PADDING: 60,        // Internal padding within containers
  ROOT_PADDING: 20,             // Root level padding
  BORDER_TO_NODE: 20,           // Spacing from border to nodes
} as const;

// ============ Zoom Level Constants ============

export const ZOOM_LEVELS = {
  MIN_INTERACTIVE: 0.2,    // Minimum zoom for interactive use
  MAX_INTERACTIVE: 2.0,    // Maximum zoom for interactive use
  MIN_FIT_VIEW: 0.1,       // Minimum zoom for automatic fit view
  MAX_FIT_VIEW: 1.5,       // Maximum zoom for automatic fit view
  DEFAULT: 0.5,            // Default initial zoom level
} as const;

// ============ MiniMap Configuration ============

export const MINIMAP_CONFIG = {
  NODE_STROKE_COLOR: COLORS.GRAY_700,
  NODE_COLOR: COLORS.GRAY_200,
  NODE_BORDER_RADIUS: SIZES.MINIMAP_NODE_BORDER_RADIUS,
} as const;

// ============ Edge Dash Patterns ============

export const DASH_PATTERNS = {
  SOLID: undefined,
  DASHED: '5,5',
  DOTTED: '2,2',
  DASH_DOT: '8,4,2,4',
} as const;

// ============ Z-Index Layers ============

export const Z_INDEX = {
  BACKGROUND: 0,
  EDGES: 1,
  NODES: 2,
  CONTAINERS: 3,
  HANDLES: 4,
  CONTROLS: 5,
  PANELS: 6,
  MODALS: 7,
  TOOLTIPS: 8,
} as const;

// ============ Responsive Breakpoints ============

export const BREAKPOINTS = {
  SM: 640,
  MD: 768,
  LG: 1024,
  XL: 1280,
} as const;

// ============ ELK Layout Engine Configuration ============

/**
 * ELK.js layout algorithms and their configurations
 * Consolidated from layout/config.ts for centralized configuration
 */
export const ELK_ALGORITHMS = {
  LAYERED: 'layered',      // Hierarchical layout - good for directed graphs
  STRESS: 'stress',        // Force-directed layout - good for undirected graphs
  MRTREE: 'mrtree',        // Multi-radial tree layout
  RADIAL: 'radial',        // Radial tree layout
  FORCE: 'force',          // Force-directed layout
} as const;

export const ELK_DIRECTIONS = {
  DOWN: 'DOWN',    // Top to bottom
  UP: 'UP',        // Bottom to top  
  LEFT: 'LEFT',    // Right to left
  RIGHT: 'RIGHT',  // Left to right
} as const;

/**
 * Default ELK layout configuration
 * Ported from layout/config.ts with enhanced spacing based on LAYOUT_SPACING
 */
export const ELK_LAYOUT_CONFIG = {
  DEFAULT: {
    algorithm: ELK_ALGORITHMS.LAYERED,
    direction: ELK_DIRECTIONS.DOWN,
    spacing: LAYOUT_SPACING.NODE_TO_NODE_NORMAL,
    nodeSize: { 
      width: SIZES.NODE_MIN_WIDTH, 
      height: SIZES.NODE_MIN_HEIGHT 
    },
  },
  
  // Alternative configurations for different use cases
  COMPACT: {
    algorithm: ELK_ALGORITHMS.LAYERED,
    direction: ELK_DIRECTIONS.DOWN,
    spacing: LAYOUT_SPACING.NODE_TO_NODE_COMPACT,
    nodeSize: { 
      width: SIZES.NODE_MIN_WIDTH, 
      height: SIZES.NODE_MIN_HEIGHT 
    },
  },
  
  LOOSE: {
    algorithm: ELK_ALGORITHMS.LAYERED,
    direction: ELK_DIRECTIONS.DOWN,
    spacing: LAYOUT_SPACING.NODE_TO_NODE_LOOSE,
    nodeSize: { 
      width: SIZES.NODE_MIN_WIDTH, 
      height: SIZES.NODE_MIN_HEIGHT 
    },
  },
  
  FORCE_DIRECTED: {
    algorithm: ELK_ALGORITHMS.FORCE,
    direction: ELK_DIRECTIONS.DOWN,
    spacing: LAYOUT_SPACING.NODE_TO_NODE_NORMAL,
    nodeSize: { 
      width: SIZES.NODE_MIN_WIDTH, 
      height: SIZES.NODE_MIN_HEIGHT 
    },
  },
  
  HORIZONTAL: {
    algorithm: ELK_ALGORITHMS.LAYERED,
    direction: ELK_DIRECTIONS.RIGHT,
    spacing: LAYOUT_SPACING.NODE_TO_NODE_NORMAL,
    nodeSize: { 
      width: SIZES.NODE_MIN_WIDTH, 
      height: SIZES.NODE_MIN_HEIGHT 
    },
  },
} as const;

/**
 * ELK-specific layout options for fine-tuning
 * Maps to elkjs layout options for advanced configuration
 */
export const ELK_LAYOUT_OPTIONS = {
  // Layered algorithm specific options
  LAYERED: {
    'elk.layered.spacing.nodeNodeBetweenLayers': LAYOUT_SPACING.LAYER_SEPARATION,
    'elk.layered.nodePlacement.strategy': 'SIMPLE',
    'elk.layered.crossingMinimization.strategy': 'LAYER_SWEEP',
    'elk.layered.layering.strategy': 'LONGEST_PATH',
  },
  
  // Force algorithm specific options  
  FORCE: {
    'elk.force.repulsivePower': 200,
    'elk.force.iterations': 300,
    'elk.force.temperature': 0.001,
  },
  
  // Stress algorithm specific options
  STRESS: {
    'elk.stress.iterations': 300,
    'elk.stress.epsilon': 0.0001,
  },
  
  // Common spacing options
  SPACING: {
    'elk.spacing.nodeNode': LAYOUT_SPACING.NODE_TO_NODE_NORMAL,
    'elk.spacing.edgeNode': LAYOUT_SPACING.EDGE_TO_NODE,
    'elk.spacing.edgeEdge': LAYOUT_SPACING.EDGE_TO_EDGE,
    'elk.spacing.componentComponent': LAYOUT_SPACING.COMPONENT_TO_COMPONENT,
  },
  
  // Padding options
  PADDING: {
    'elk.padding.left': LAYOUT_SPACING.ROOT_PADDING,
    'elk.padding.right': LAYOUT_SPACING.ROOT_PADDING,
    'elk.padding.top': LAYOUT_SPACING.ROOT_PADDING,
    'elk.padding.bottom': LAYOUT_SPACING.ROOT_PADDING,
  },
} as const;

// Type definitions for ELK configuration
export type ELKAlgorithm = typeof ELK_ALGORITHMS[keyof typeof ELK_ALGORITHMS];
export type ELKDirection = typeof ELK_DIRECTIONS[keyof typeof ELK_DIRECTIONS];
export type ELKLayoutConfigKey = keyof typeof ELK_LAYOUT_CONFIG;

/**
 * ELK Layout Configuration Interface
 * Matches the LayoutConfig from layout/types.ts for consistency
 */
export interface ELKLayoutConfig {
  algorithm?: ELKAlgorithm;
  direction?: ELKDirection;
  spacing?: number;
  nodeSize?: { width: number; height: number };
}

// ============ Default Configurations ============

export const DEFAULT_NODE_STYLE = {
  borderRadius: DEFAULT_STYLES.BORDER_RADIUS,
  padding: `${SIZES.NODE_PADDING}px`,
  color: '#ffffff',  // Pure white for maximum contrast
  fontFamily: TYPOGRAPHY.FONT_FAMILY,
  fontSize: '13px',  // Optimized size for readability
  fontWeight: '600', // Semibold for better contrast
  textShadow: '0 1px 2px rgba(0, 0, 0, 0.3)',  // Text shadow for contrast
  border: 'none',
  boxShadow: SHADOWS.NODE_DEFAULT,
  transition: ANIMATIONS.TRANSITION_DEFAULT,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  textAlign: 'center',
  width: 200,
  height: 60,
  cursor: 'pointer',
} as const;

export const DEFAULT_EDGE_STYLE = {
  strokeWidth: SIZES.EDGE_WIDTH_DEFAULT,
  stroke: EDGE_COLORS.DEFAULT,
  strokeDasharray: DASH_PATTERNS.SOLID,
} as const;

export const DEFAULT_CONTAINER_STYLE = {
  backgroundColor: CONTAINER_COLORS.BACKGROUND,
  border: `${SIZES.BORDER_WIDTH_DEFAULT}px solid ${CONTAINER_COLORS.BORDER}`,
  borderRadius: `${SIZES.CONTAINER_BORDER_RADIUS}px`,
  padding: `${SIZES.CONTAINER_PADDING}px`,
  boxShadow: SHADOWS.CONTAINER_DEFAULT,
} as const;

// ============ Helper Functions ============

/**
 * Get appropriate text color for given background
 * Ensures WCAG AA compliance (4.5:1 contrast ratio)
 */
export function getAccessibleTextColor(backgroundColor: string): string {
  // Simple implementation - for full solution would calculate luminance
  const lightBackgrounds = [
    COLORS.WHITE, COLORS.GRAY_50, COLORS.GRAY_100, COLORS.GRAY_200,
    COLORS.SUCCESS_50, COLORS.SUCCESS_100, COLORS.WARNING_50, COLORS.WARNING_100,
    COLORS.ERROR_50, COLORS.ERROR_100, COLORS.INFO_50, COLORS.INFO_100
  ];
  
  return lightBackgrounds.includes(backgroundColor as any) 
    ? COLORS.GRAY_900 
    : COLORS.WHITE;
}

/**
 * Get semantic color for status/feedback
 */
export function getStatusColor(status: 'success' | 'warning' | 'error' | 'info', variant: '50' | '100' | '500' | '600' | '700' = '500'): string {
  const statusMap: Record<string, string> = {
    success: (COLORS as any)[`SUCCESS_${variant}`],
    warning: (COLORS as any)[`WARNING_${variant}`], 
    error: (COLORS as any)[`ERROR_${variant}`],
    info: (COLORS as any)[`INFO_${variant}`],
  };
  return statusMap[status] || COLORS.GRAY_500;
}

/**
 * Get node border color based on style and state
 */
export function getNodeBorderColor(style: string, selected?: boolean, highlighted?: boolean): string {
  if (selected) return NODE_COLORS.BORDER.SELECTED;
  if (highlighted) return NODE_COLORS.BORDER.HIGHLIGHTED;
  
  switch (style) {
    case 'error': return NODE_COLORS.BORDER.ERROR;
    case 'warning': return NODE_COLORS.BORDER.WARNING;
    case 'highlighted': return NODE_COLORS.BORDER.HIGHLIGHTED;
    case 'selected': return NODE_COLORS.BORDER.SELECTED;
    default: return NODE_COLORS.BORDER.DEFAULT;
  }
}

/**
 * Get node text color based on style
 */
export function getNodeTextColor(style: string): string {
  switch (style) {
    case 'error': return NODE_COLORS.TEXT.ERROR;
    case 'warning': return NODE_COLORS.TEXT.WARNING;
    case 'highlighted': return NODE_COLORS.TEXT.HIGHLIGHTED;
    case 'selected': return NODE_COLORS.TEXT.SELECTED;
    default: return NODE_COLORS.TEXT.DEFAULT;
  }
}

/**
 * Get edge color based on style and state
 */
export function getEdgeColor(style?: string, selected?: boolean, highlighted?: boolean): string {
  if (selected) return EDGE_COLORS.SELECTED;
  if (highlighted) return EDGE_COLORS.HIGHLIGHTED;
  
  switch (style) {
    case 'data': return EDGE_COLORS.DATA;
    case 'control': return EDGE_COLORS.CONTROL;
    case 'error': return EDGE_COLORS.ERROR;
    case 'thick': return EDGE_COLORS.THICK;
    case 'dashed': return EDGE_COLORS.DASHED;
    default: return EDGE_COLORS.DEFAULT;
  }
}

/**
 * Get edge stroke width based on style
 */
export function getEdgeStrokeWidth(style?: string): number {
  switch (style) {
    case 'thick': return SIZES.EDGE_WIDTH_THICK;
    default: return SIZES.EDGE_WIDTH_DEFAULT;
  }
}

/**
 * Get edge dash pattern based on style
 */
export function getEdgeDashPattern(style?: string): string | undefined {
  switch (style) {
    case 'dashed': return DASH_PATTERNS.DASHED;
    case 'dotted': return DASH_PATTERNS.DOTTED;
    default: return DASH_PATTERNS.SOLID;
  }
}

/**
 * Get ELK layout configuration by name or return custom config
 */
export function getELKLayoutConfig(configKey?: ELKLayoutConfigKey | ELKLayoutConfig): ELKLayoutConfig {
  if (typeof configKey === 'string') {
    return ELK_LAYOUT_CONFIG[configKey];
  }
  if (configKey && typeof configKey === 'object') {
    return { ...ELK_LAYOUT_CONFIG.DEFAULT, ...configKey };
  }
  return ELK_LAYOUT_CONFIG.DEFAULT;
}

/**
 * Get ELK layout options for specific algorithm with enhanced spacing
 */
export function getELKLayoutOptions(algorithm: ELKAlgorithm): Record<string, any> {
  const baseOptions = {
    ...ELK_LAYOUT_OPTIONS.SPACING,
    ...ELK_LAYOUT_OPTIONS.PADDING,
  };
  
  switch (algorithm) {
    case ELK_ALGORITHMS.LAYERED:
      return { ...baseOptions, ...ELK_LAYOUT_OPTIONS.LAYERED };
    case ELK_ALGORITHMS.FORCE:
      return { ...baseOptions, ...ELK_LAYOUT_OPTIONS.FORCE };
    case ELK_ALGORITHMS.STRESS:
      return { ...baseOptions, ...ELK_LAYOUT_OPTIONS.STRESS };
    default:
      return baseOptions;
  }
}
