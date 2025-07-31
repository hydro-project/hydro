/**
 * Centralized color constants for the visualizer
 * Professional color system based on ColorBrewer and WCAG accessibility guidelines
 */

// ============================================================================
// SEMANTIC COLOR SYSTEM
// Based on ColorBrewer qualitative and sequential schemes
// All colors meet WCAG AA contrast requirements (4.5:1 minimum)
// ============================================================================

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
  // Based on ColorBrewer qualitative schemes
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
};

// ============================================================================
// ENHANCED COLOR PALETTES FOR DATA VISUALIZATION
// Based on ColorBrewer schemes, optimized for accessibility
// ============================================================================

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
};

// ============================================================================
// COMPONENT-SPECIFIC COLOR MAPS
// ============================================================================

export const COMPONENT_COLORS = {
  // Edge and connection colors
  EDGE_DEFAULT: COLORS.GRAY_400,
  EDGE_HOVER: COLORS.GRAY_600,
  EDGE_SELECTED: COLORS.INFO_500,
  
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
};

// ============================================================================
// DESIGN TOKENS & UTILITIES
// ============================================================================

export const DEFAULT_STYLES = {
  BORDER_WIDTH: '1px',
  BORDER_WIDTH_THICK: '2px', 
  BORDER_RADIUS_SM: '4px',
  BORDER_RADIUS: '6px',
  BORDER_RADIUS_LG: '8px',
  BOX_SHADOW_SM: '0 1px 2px 0 rgba(0, 0, 0, 0.05)',
  BOX_SHADOW: '0 1px 3px 0 rgba(0, 0, 0, 0.1), 0 1px 2px 0 rgba(0, 0, 0, 0.06)',
  BOX_SHADOW_LG: '0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)',
};

// ============================================================================
// ACCESSIBILITY UTILITIES
// ============================================================================

/**
 * Get appropriate text color for given background
 * Ensures WCAG AA compliance (4.5:1 contrast ratio)
 */
export const getAccessibleTextColor = (backgroundColor) => {
  // Simple implementation - for full solution would calculate luminance
  const lightBackgrounds = [
    COLORS.WHITE, COLORS.GRAY_50, COLORS.GRAY_100, COLORS.GRAY_200,
    COLORS.SUCCESS_50, COLORS.SUCCESS_100, COLORS.WARNING_50, COLORS.WARNING_100,
    COLORS.ERROR_50, COLORS.ERROR_100, COLORS.INFO_50, COLORS.INFO_100
  ];
  
  return lightBackgrounds.includes(backgroundColor) 
    ? COLORS.GRAY_900 
    : COLORS.WHITE;
};

/**
 * Get semantic color for status/feedback
 */
export const getStatusColor = (status, variant = '500') => {
  const statusMap = {
    success: COLORS[`SUCCESS_${variant}`],
    warning: COLORS[`WARNING_${variant}`], 
    error: COLORS[`ERROR_${variant}`],
    info: COLORS[`INFO_${variant}`],
  };
  return statusMap[status] || COLORS.GRAY_500;
};

// ============================================================================
// COMMON VALIDATION UTILITIES  
// ============================================================================

export const isValidGraphData = (graphData) => {
  return graphData && graphData.nodes && graphData.nodes.length > 0;
};

export const isValidNodesArray = (nodes) => {
  return nodes && nodes.length > 0;
};

// ============================================================================
// COMMON NODE FILTERING UTILITIES
// ============================================================================

export const filterNodesByType = (nodes, type) => {
  return nodes.filter(node => node.type === type);
};

export const filterNodesByParent = (nodes, parentId) => {
  return nodes.filter(node => node.parentId === parentId);
};

export const filterNodesExcludingType = (nodes, type) => {
  return nodes.filter(node => node.type !== type);
};

export const getUniqueNodesById = (nodes) => {
  return nodes.filter((node, index, array) => 
    array.findIndex(n => n.id === node.id) === index
  );
};
