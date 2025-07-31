/**
 * Centralized ELK Layout Configuration
 * 
 * This file contains ALL ELK (Eclipse Layout Kernel) configurations in one location.
 * This includes:
 * - Layout algorithm configurations for each algorithm type
 * - Common layout options and spacing settings
 * - Container-specific layout configurations
 * - Layout option constants and utilities
 * 
 * All spacing, padding, and algorithm-specific values are imported from constants.js
 * to maintain consistency across the application.
 * 
 * DO NOT define ELK layout options in individual components - modify them here instead.
 */

import { 
  LAYOUT_SPACING, 
  ANIMATION_TIMINGS 
} from './constants.js';

// ============================================================================
// CORE ELK ALGORITHM CONFIGURATIONS
// Each layout algorithm has its own optimized settings
// ============================================================================

export const ELK_LAYOUT_CONFIGS = {
  // Multi-level tree layout - good for hierarchical data
  mrtree: {
    'elk.algorithm': 'mrtree',
    'elk.direction': 'DOWN',
    'elk.spacing.nodeNode': LAYOUT_SPACING.NODE_TO_NODE_LOOSE,
    'elk.spacing.edgeNode': LAYOUT_SPACING.EDGE_TO_NODE,
  },
  
  // Layered layout - good for directed graphs
  layered: {
    'elk.algorithm': 'layered',
    'elk.direction': 'DOWN',
    'elk.spacing.nodeNode': LAYOUT_SPACING.NODE_TO_NODE_LOOSE,
    'elk.layered.spacing.nodeNodeBetweenLayers': LAYOUT_SPACING.LAYER_SEPARATION,
    'elk.layered.spacing.borderToNode': 20,
  },
  
  // Force-directed layout - good for general graphs
  force: {
    'elk.algorithm': 'force',
    'elk.spacing.nodeNode': LAYOUT_SPACING.NODE_TO_NODE_LOOSE,
  },
  
  // Stress minimization layout - good for complex networks
  stress: {
    'elk.algorithm': 'stress',
    'elk.spacing.nodeNode': LAYOUT_SPACING.NODE_TO_NODE_LOOSE,
  },
  
  // Radial layout - good for tree-like structures
  radial: {
    'elk.algorithm': 'radial',
    'elk.spacing.nodeNode': LAYOUT_SPACING.NODE_TO_NODE_LOOSE,
  },
};

// ============================================================================
// LAYOUT OPTION CONSTANTS
// Common ELK options used across different layout scenarios
// ============================================================================

export const ELK_OPTIONS = {
  // Hierarchy handling
  HIERARCHY_HANDLING: {
    INCLUDE_CHILDREN: 'INCLUDE_CHILDREN',
    SEPARATE_CHILDREN: 'SEPARATE_CHILDREN',
  },
  
  // Node sizing constraints
  NODE_SIZE_CONSTRAINTS: {
    FREE: '',
    FIXED_SIZE: 'FIXED_SIZE',
    FIXED_POS: 'FIXED_POS',
    MINIMUM_SIZE: 'MINIMUM_SIZE',
  },
  
  // Layout directions
  DIRECTIONS: {
    UP: 'UP',
    DOWN: 'DOWN',
    LEFT: 'LEFT',
    RIGHT: 'RIGHT',
  },
  
  // Common spacing values
  SPACING: {
    EDGE_TO_NODE: LAYOUT_SPACING.EDGE_TO_NODE,
    EDGE_TO_EDGE: 10,
    COMPONENT_TO_COMPONENT: 60,
    CONTAINER_PADDING: 20,
    ROOT_PADDING: 20,
  },
};

// ============================================================================
// CONTAINER-SPECIFIC CONFIGURATIONS
// Optimized settings for different container scenarios
// ============================================================================

export const ELK_CONTAINER_CONFIGS = {
  // Configuration for standard hierarchy containers
  hierarchyContainer: {
    'elk.spacing.nodeNode': LAYOUT_SPACING.NODE_TO_NODE_LOOSE,
    'elk.spacing.edgeNode': LAYOUT_SPACING.EDGE_TO_NODE,
    'elk.spacing.edgeEdge': ELK_OPTIONS.SPACING.EDGE_TO_EDGE,
  },
  
  // Configuration for collapsed container repositioning
  collapsedContainer: {
    'elk.spacing.nodeNode': LAYOUT_SPACING.NODE_TO_NODE_LOOSE, // Use consistent spacing
    'elk.spacing.componentComponent': ELK_OPTIONS.SPACING.COMPONENT_TO_COMPONENT,
    'elk.partitioning.activate': 'false',
  },
  
  // Configuration for root-level layout
  rootLevel: {
    'elk.padding': `[top=${ELK_OPTIONS.SPACING.ROOT_PADDING},left=${ELK_OPTIONS.SPACING.ROOT_PADDING},bottom=${ELK_OPTIONS.SPACING.ROOT_PADDING},right=${ELK_OPTIONS.SPACING.ROOT_PADDING}]`,
    'elk.hierarchyHandling': ELK_OPTIONS.HIERARCHY_HANDLING.INCLUDE_CHILDREN,
    'elk.spacing.nodeNode': LAYOUT_SPACING.NODE_TO_NODE_LOOSE, // Use consistent spacing
    'elk.spacing.edgeNode': ELK_OPTIONS.SPACING.CONTAINER_PADDING,
    'elk.spacing.edgeEdge': 15,
  },
};

// ============================================================================
// LAYOUT UTILITIES
// Helper functions for working with ELK configurations
// ============================================================================

/**
 * Get the full ELK configuration for a specific layout type
 * @param {string} layoutType - The layout algorithm to use
 * @returns {Object} Complete ELK configuration object
 */
export function getELKConfig(layoutType = 'mrtree') {
  const baseConfig = ELK_LAYOUT_CONFIGS[layoutType];
  if (!baseConfig) {
    console.warn(`Unknown layout type: ${layoutType}, falling back to mrtree`);
    return ELK_LAYOUT_CONFIGS.mrtree;
  }
  return { ...baseConfig };
}

/**
 * Get ELK configuration for container nodes
 * @param {string} layoutType - The layout algorithm to use
 * @param {string} containerType - Type of container ('hierarchy', 'collapsed', 'root')
 * @returns {Object} Container-specific ELK configuration
 */
export function getContainerELKConfig(layoutType = 'mrtree', containerType = 'hierarchy') {
  const baseConfig = getELKConfig(layoutType);
  
  const containerConfigs = {
    hierarchy: ELK_CONTAINER_CONFIGS.hierarchyContainer,
    collapsed: ELK_CONTAINER_CONFIGS.collapsedContainer,
    root: ELK_CONTAINER_CONFIGS.rootLevel,
  };
  
  const containerConfig = containerConfigs[containerType] || containerConfigs.hierarchy;
  
  return {
    ...baseConfig,
    ...containerConfig,
  };
}

/**
 * Create ELK node sizing options for fixed positioning
 * @param {number} x - X position to fix
 * @param {number} y - Y position to fix
 * @returns {Object} ELK layout options for fixed positioning
 */
export function createFixedPositionOptions(x, y) {
  return {
    'elk.position.x': x.toString(),
    'elk.position.y': y.toString(),
    'elk.nodeSize.constraints': ELK_OPTIONS.NODE_SIZE_CONSTRAINTS.FIXED_POS,
    'elk.nodeSize.options': ELK_OPTIONS.NODE_SIZE_CONSTRAINTS.FIXED_POS,
  };
}

/**
 * Create ELK node sizing options for free positioning
 * @returns {Object} ELK layout options for free positioning
 */
export function createFreePositionOptions() {
  return {
    'elk.nodeSize.constraints': ELK_OPTIONS.NODE_SIZE_CONSTRAINTS.FREE,
    'elk.nodeSize.options': ELK_OPTIONS.NODE_SIZE_CONSTRAINTS.FREE,
  };
}

// ============================================================================
// LAYOUT ALGORITHM METADATA
// Information about each layout algorithm for UI display
// ============================================================================

export const LAYOUT_ALGORITHM_INFO = {
  mrtree: {
    name: 'MR Tree',
    description: 'Multi-level tree layout, ideal for hierarchical data structures',
    bestFor: ['hierarchical data', 'tree structures', 'clear parent-child relationships'],
  },
  layered: {
    name: 'Layered',
    description: 'Layered layout with nodes arranged in discrete levels',
    bestFor: ['directed graphs', 'workflow diagrams', 'dependency graphs'],
  },
  force: {
    name: 'Force',
    description: 'Force-directed layout with natural node positioning',
    bestFor: ['general graphs', 'network visualization', 'organic layouts'],
  },
  stress: {
    name: 'Stress',
    description: 'Stress minimization layout for complex networks',
    bestFor: ['complex networks', 'large graphs', 'minimizing edge crossings'],
  },
  radial: {
    name: 'Radial',
    description: 'Radial layout with nodes arranged in concentric circles',
    bestFor: ['tree structures', 'central node focus', 'symmetric layouts'],
  },
};

/**
 * Get available layout options for UI controls
 * @returns {Object} Layout options formatted for UI consumption
 */
export function getLayoutOptions() {
  const options = {};
  Object.keys(LAYOUT_ALGORITHM_INFO).forEach(key => {
    options[key] = LAYOUT_ALGORITHM_INFO[key].name;
  });
  return options;
}
