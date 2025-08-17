/**
 * Edge Style Processor
 * 
 * Processes edge properties and applies appropriate visual styles based on
 * the edgeStyleConfig from the JSON data.
 */

import { EdgeStyle } from '../shared/config';

export interface EdgeStyleConfig {
  propertyMappings: Record<string, string | {
    reactFlowType?: string;
    style?: Record<string, any>;
    animated?: boolean;
    label?: string;
    styleTag?: string;
  }>;
  combinationRules?: {
    description?: string;
    // Legacy support for backward compatibility
    priority?: string[];
    mutuallyExclusiveGroups?: any;
    visualGroups?: any;
  };
}

export interface ProcessedEdgeStyle {
  reactFlowType: string;
  style: Record<string, any>;
  animated: boolean;
  label?: string;
  appliedProperties: string[];
}

/**
 * Process edge properties and return the appropriate visual style
 */
export function processEdgeStyle(
  edgeProperties: string[],
  styleConfig?: EdgeStyleConfig
): ProcessedEdgeStyle {
  if (!edgeProperties || edgeProperties.length === 0) {
    return getDefaultStyle();
  }

  // If we have a style config from JSON, use it to map semantic tags to style tags
  if (styleConfig && styleConfig.propertyMappings) {
    return processWithMappings(edgeProperties, styleConfig);
  }

  // Otherwise, treat edge properties as direct style tags
  return processDirectStyleTags(edgeProperties);
}

/**
 * Process edges using semantic -> style tag mappings from JSON
 */
function processWithMappings(
  edgeProperties: string[],
  styleConfig: EdgeStyleConfig
): ProcessedEdgeStyle {
  // Collect all style tags from the properties
  const styleTags: string[] = [];
  
  console.log('🔍 Processing edge properties:', edgeProperties);
  console.log('🔍 Available property mappings:', styleConfig.propertyMappings);
  
  for (const property of edgeProperties) {
    const mapping = styleConfig.propertyMappings[property];
    console.log(`🔍 Property "${property}" maps to:`, mapping);
    if (mapping) {
      if (typeof mapping === 'string') {
        styleTags.push(mapping);
        console.log(`🔍 Added style tag: "${mapping}"`);
      } else if (mapping.styleTag) {
        styleTags.push(mapping.styleTag);
        console.log(`🔍 Added style tag from object: "${mapping.styleTag}"`);
      }
    }
  }
  
  console.log('🔍 Final style tags:', styleTags);
  
  // If we have style tags, combine them with intelligent CSS property handling
  if (styleTags.length > 0) {
    return combineStyleTagsIntelligently(styleTags, edgeProperties);
  }
  
  // Fallback: find the highest priority property with any mapping
  const priorityOrder = styleConfig.combinationRules?.priority || [];
  let selectedProperty: string | null = null;
  
  for (const priority of priorityOrder) {
    if (edgeProperties.includes(priority) && styleConfig.propertyMappings[priority]) {
      selectedProperty = priority;
      break;
    }
  }

  if (!selectedProperty) {
    selectedProperty = edgeProperties.find(prop => styleConfig.propertyMappings[prop]) || null;
  }

  if (selectedProperty && styleConfig.propertyMappings[selectedProperty]) {
    const mapping = styleConfig.propertyMappings[selectedProperty];
    
    // Handle backward compatibility with full style objects
    if (typeof mapping === 'object' && mapping.style) {
      return {
        reactFlowType: 'floating',
        style: { ...mapping.style },
        animated: mapping.animated || false,
        label: mapping.label,
        appliedProperties: [selectedProperty]
      };
    }
  }

  // Fallback to treating semantic properties as direct style tags
  return processDirectStyleTags(edgeProperties);
}

/**
 * Process edge properties as direct style tag names
 */
function processDirectStyleTags(edgeProperties: string[]): ProcessedEdgeStyle {
  // Use first property as style tag
  const styleTag = edgeProperties[0];
  return mapStyleTagToVisual(styleTag, edgeProperties);
}

/**
 * Map a style tag name to actual ReactFlow visual style
 */
function mapStyleTagToVisual(styleTag: string, originalProperties: string[]): ProcessedEdgeStyle {
  const styleTagMappings: Record<string, any> = {
    // Compound visual styles (for boolean pairs)
    'dashed-animated': {
      style: { strokeDasharray: '8,4' },
      animated: true,
      label: '- ->'
    },
    'thin-stroke': {
      style: { strokeWidth: 1 },
      animated: false,
      label: 'thin'
    },
    'thick-stroke': {
      style: { strokeWidth: 3 },
      animated: false,
      label: 'thick'
    },
    'wavy-line': {
      style: { strokeDasharray: '5,5' },
      animated: true,
      label: '~'
    },
    'smooth-line': {
      style: { strokeDasharray: undefined },
      animated: false,
      label: '—'
    },
    'double-line': {
      style: { strokeDasharray: '10,2,2,2' },
      animated: false,
      label: '='
    },
    
    // Basic line patterns
    'solid': {
      style: { strokeDasharray: undefined },
      animated: false,
      label: '—'
    },
    'dashed': {
      style: { strokeDasharray: '8,4' },
      animated: false,
      label: '- -'
    },
    'dotted': {
      style: { strokeDasharray: '2,2' },
      animated: false,
      label: '...'
    },
    'wavy': {
      style: { strokeDasharray: '5,5' },
      animated: true,
      label: '~'
    },
    'double': {
      style: { strokeDasharray: '10,2,2,2' },
      animated: false,
      label: '='
    },
    
    // Line thickness
    'thin': {
      style: { strokeWidth: 1 },
      animated: false,
      label: 'T'
    },
    'normal': {
      style: { strokeWidth: 2 },
      animated: false,
      label: 'N'
    },
    'thick': {
      style: { strokeWidth: 3 },
      animated: false,
      label: 'B'
    },
    'extra-thick': {
      style: { strokeWidth: 4 },
      animated: false,
      label: 'BB'
    },
    
    // Animation
    'animated': {
      style: {},
      animated: true,
      label: '>'
    },
    'static': {
      style: {},
      animated: false,
      label: ''
    },
    
    // Colors (for when semantic tags directly specify colors)
    'blue': {
      style: { stroke: '#2563eb' },
      animated: false,
      label: 'B'
    },
    'red': {
      style: { stroke: '#dc2626' },
      animated: false,
      label: 'R'
    },
    'green': {
      style: { stroke: '#16a34a' },
      animated: false,
      label: 'G'
    },
    'orange': {
      style: { stroke: '#ea580c' },
      animated: false,
      label: 'O'
    },
    'purple': {
      style: { stroke: '#9333ea' },
      animated: false,
      label: 'P'
    },
    'gray': {
      style: { stroke: '#6b7280' },
      animated: false,
      label: 'GY'
    }
  };

  const normalizedTag = styleTag.toLowerCase().replace(/[_\s]/g, '-');
  const visualStyle = styleTagMappings[normalizedTag];
  
  if (visualStyle) {
    return {
      reactFlowType: 'floating',
      style: { 
        stroke: '#666666', // Default color
        strokeWidth: 2,    // Default width
        ...visualStyle.style 
      },
      animated: visualStyle.animated,
      label: visualStyle.label,
      appliedProperties: originalProperties
    };
  }

  // Unknown style tag - generate style based on hash
  const hash = styleTag.split('').reduce((a, b) => a + b.charCodeAt(0), 0);
  const hue = hash % 360;
  
  return {
    reactFlowType: 'floating',
    style: {
      stroke: `hsl(${hue}, 60%, 50%)`,
      strokeWidth: 2
    },
    animated: false,
    label: styleTag.substring(0, 3).toUpperCase(),
    appliedProperties: originalProperties
  };
}

/**
 * Combine style tags using priority rules to handle conflicts
 */
function combineStyleTagsWithPriority(styleTags: string[], originalProperties: string[], styleConfig: EdgeStyleConfig): ProcessedEdgeStyle {
  console.log('🎯 Combining with priority. Style tags:', styleTags);
  console.log('🎯 Priority order:', styleConfig.combinationRules?.priority);
  
  // Start with default style
  let combinedStyle: any = {
    stroke: '#666666',
    strokeWidth: 2
  };
  let animated = false;
  let labels: string[] = [];
  
  // Group style tags by the CSS property they affect
  const styleGroups: Record<string, {tag: string, priority: number}[]> = {};
  
  for (const tag of styleTags) {
    const tagStyle = mapStyleTagToVisual(tag, []);
    console.log(`🎯 Style tag "${tag}" affects:`, Object.keys(tagStyle.style));
    
    // Find the original property that created this tag
    const originalProperty = originalProperties.find(prop => 
      styleConfig.propertyMappings[prop] === tag
    );
    const priority = styleConfig.combinationRules?.priority?.indexOf(originalProperty || '') ?? 999;
    
    // Group by CSS property
    for (const cssProp of Object.keys(tagStyle.style)) {
      if (!styleGroups[cssProp]) {
        styleGroups[cssProp] = [];
      }
      styleGroups[cssProp].push({ tag, priority });
    }
    
    // Handle non-style properties
    if (tagStyle.animated) {
      animated = true;
    }
    if (tagStyle.label) {
      labels.push(tagStyle.label);
    }
  }
  
  console.log('🎯 Style groups by CSS property:', styleGroups);
  
  // For each CSS property, use the tag with highest priority (lowest index)
  for (const [cssProp, candidates] of Object.entries(styleGroups)) {
    // Sort by priority (lowest number = highest priority)
    candidates.sort((a, b) => a.priority - b.priority);
    const winningTag = candidates[0].tag;
    
    console.log(`🎯 For CSS property "${cssProp}", winner is "${winningTag}" (priority ${candidates[0].priority})`);
    
    // Apply the winning tag's style for this property
    const tagStyle = mapStyleTagToVisual(winningTag, []);
    if (tagStyle.style[cssProp] !== undefined) {
      combinedStyle[cssProp] = tagStyle.style[cssProp];
    }
  }
  
  console.log('🎯 Final prioritized style:', combinedStyle);
  
  return {
    reactFlowType: 'floating',
    style: combinedStyle,
    animated: animated,
    label: labels.join(''),
    appliedProperties: originalProperties
  };
}

/**
 * Combine multiple style tags into a single visual style (old method - kept for fallback)
 */
function combineStyleTags(styleTags: string[], originalProperties: string[]): ProcessedEdgeStyle {
  // Start with default style
  let combinedStyle: any = {
    stroke: '#666666',
    strokeWidth: 2
  };
  let animated = false;
  let labels: string[] = [];
  
  // Apply each style tag
  for (const tag of styleTags) {
    const tagStyle = mapStyleTagToVisual(tag, []);
    console.log('🔧 Processing style tag:', tag, '→', tagStyle);
    
    // Merge styles (later tags can override earlier ones)
    combinedStyle = { ...combinedStyle, ...tagStyle.style };
    
    // Animation is true if any tag enables it
    if (tagStyle.animated) {
      animated = true;
    }
    
    // Collect labels
    if (tagStyle.label) {
      labels.push(tagStyle.label);
    }
  }
  
  console.log('🔧 Final combined style:', combinedStyle);
  
  return {
    reactFlowType: 'floating',
    style: combinedStyle,
    animated: animated,
    label: labels.join(''),
    appliedProperties: originalProperties
  };
}

/**
 * Get default style for edges with no properties
 */
function getDefaultStyle(): ProcessedEdgeStyle {
  return {
    reactFlowType: 'floating',
    style: {
      stroke: '#999999',
      strokeWidth: 2
    },
    animated: false,
    appliedProperties: []
  };
}

/**
 * Combine multiple edge properties into a single label
 */
export function createEdgeLabel(
  edgeProperties: string[],
  styleConfig?: EdgeStyleConfig,
  originalLabel?: string
): string | undefined {
  if (!edgeProperties || edgeProperties.length === 0) {
    return originalLabel;
  }

  // Create abbreviated labels for common properties
  const abbreviations: Record<string, string> = {
    'Network': 'N',
    'Cycle': 'C',
    'Bounded': 'B',
    'Unbounded': 'U',
    'NoOrder': '~',
    'TotalOrder': 'O',
    'Keyed': 'K'
  };

  const propertyLabels = edgeProperties
    .map(prop => abbreviations[prop] || prop.charAt(0))
    .join('');

  if (originalLabel) {
    return `${originalLabel} [${propertyLabels}]`;
  }

  return propertyLabels.length > 0 ? propertyLabels : undefined;
}

/**
 * Get a human-readable description of edge properties
 */
export function getEdgePropertiesDescription(
  edgeProperties: string[],
  styleConfig?: EdgeStyleConfig
): string {
  if (!edgeProperties || edgeProperties.length === 0) {
    return 'No properties';
  }

  const descriptions: Record<string, string> = {
    'Network': 'Network communication',
    'Cycle': 'Cyclic data flow',
    'Bounded': 'Finite data stream',
    'Unbounded': 'Infinite data stream',
    'NoOrder': 'Unordered data',
    'TotalOrder': 'Ordered data',
    'Keyed': 'Key-value pairs'
  };

  return edgeProperties
    .map(prop => descriptions[prop] || prop)
    .join(', ');
}

/**
 * Combine style tags intelligently - properties affecting same CSS attribute are mutually exclusive
 */
function combineStyleTagsIntelligently(styleTags: string[], originalProperties: string[]): ProcessedEdgeStyle {
  // Start with default style
  let combinedStyle: any = {
    stroke: '#666666',
    strokeWidth: 2
  };
  let animated = false;
  let labels: string[] = [];
  
  // Group style effects by CSS property they affect
  const cssPropertyEffects: Record<string, string> = {};
  
  // Process each style tag and collect its effects
  for (const tag of styleTags) {
    const tagStyle = mapStyleTagToVisual(tag, []);
    
    // For each CSS property this tag affects, track the latest value
    // This naturally handles mutual exclusion (later tags override earlier ones)
    for (const [cssProp, value] of Object.entries(tagStyle.style)) {
      cssPropertyEffects[cssProp] = value;
    }
    
    // Handle non-style properties
    if (tagStyle.animated) {
      animated = true;
    }
    if (tagStyle.label) {
      labels.push(tagStyle.label);
    }
  }
  
  // Apply all collected CSS effects
  combinedStyle = { ...combinedStyle, ...cssPropertyEffects };
  
  return {
    reactFlowType: 'floating',
    style: combinedStyle,
    animated: animated,
    label: labels.join(''),
    appliedProperties: originalProperties
  };
}