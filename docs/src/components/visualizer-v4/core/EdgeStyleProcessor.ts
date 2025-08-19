/**
 * Edge Style Processor
 * 
 * Processes edge properties and applies appropriate visual styles based on
 * the edgeStyleConfig from the JSON data.
 */

import { EdgeStyle } from '../shared/config';

export interface EdgeStyleConfig {
  semanticMappings?: Record<string, Record<string, Record<string, string | number>>>;
  // Legacy support for backward compatibility
  booleanPropertyPairs?: Array<{
    pair: [string, string];
    defaultStyle: string;
    altStyle: string;
    description: string;
  }>;
  singlePropertyMappings?: Record<string, string>;
  combinationRules?: {
    description?: string;
    mutualExclusions?: string[][];
    visualGroups?: Record<string, string[]>;
  };
  propertyMappings?: Record<string, string | {
    reactFlowType?: string;
    style?: Record<string, any>;
    animated?: boolean;
    label?: string;
    styleTag?: string;
  }>;
}

// Fixed style categories defined by the visualizer
export const EDGE_STYLE_CATEGORIES = {
  "line-pattern": ["solid", "dashed", "dotted", "dash-dot"],
  "line-width": [1, 2, 3, 4],
  "animation": ["static", "animated"],
  "line-style": ["single", "double"],
  "halo": ["none", "light-blue", "light-red", "light-green"],
  // collection markers; we map to native React Flow markers where possible
  "arrowhead": ["none", "triangle-open", "triangle-filled", "circle-filled", "diamond-open"],
  // ordering
  "waviness": ["none", "wavy"]
} as const;

export interface ProcessedEdgeStyle {
  reactFlowType: string;
  style: Record<string, any>;
  animated: boolean;
  label?: string;
  appliedProperties: string[];
  markerEndSpec?: any;
  lineStyle?: 'single' | 'double';
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
  if (styleConfig && (styleConfig.semanticMappings || styleConfig.propertyMappings)) {
    return processWithMappings(edgeProperties, styleConfig);
  }

  // Otherwise, treat edge properties as direct style tags
  return processDirectStyleTags(edgeProperties);
}

/**
 * Process edges using semantic property mappings and style categories
 */
function processWithMappings(
  edgeProperties: string[],
  styleConfig: EdgeStyleConfig
): ProcessedEdgeStyle {
  // Handle new semantic mappings system
  if (styleConfig.semanticMappings) {
    return processWithSemanticMappings(edgeProperties, styleConfig);
  }

  // Handle legacy boolean property pair system
  if (styleConfig.booleanPropertyPairs || styleConfig.singlePropertyMappings) {
    return processWithBooleanPairs(edgeProperties, styleConfig);
  }

  // Legacy fallback: handle old propertyMappings format
  if (styleConfig.propertyMappings) {
    return processLegacyMappings(edgeProperties, styleConfig);
  }

  // Final fallback
  return processDirectStyleTags(edgeProperties);
}

/**
 * Validate semantic mappings to ensure no style property appears in multiple groups
 * (but it's OK for the same style property to appear in different options within the same group)
 */
function validateSemanticMappings(semanticMappings: Record<string, Record<string, Record<string, string | number>>>): void {
  const stylePropertyToGroups: Record<string, string[]> = {};

  // Track which groups use each style property
  for (const [groupName, group] of Object.entries(semanticMappings)) {
    const groupStyleProperties = new Set<string>();
    
    // Collect all style properties used in this group
    for (const [optionName, styleMapping] of Object.entries(group)) {
      for (const styleProperty of Object.keys(styleMapping)) {
        groupStyleProperties.add(styleProperty);
      }
    }

    // Track group usage for each style property
    for (const styleProperty of groupStyleProperties) {
      if (!stylePropertyToGroups[styleProperty]) {
        stylePropertyToGroups[styleProperty] = [];
      }
      stylePropertyToGroups[styleProperty].push(groupName);
    }
  }

  // Check for conflicts (same style property used in multiple groups)
  const conflicts: string[] = [];
  for (const [styleProperty, groups] of Object.entries(stylePropertyToGroups)) {
    if (groups.length > 1) {
      conflicts.push(`Style property "${styleProperty}" is used in multiple groups: ${groups.join(', ')}`);
    }
  }

  if (conflicts.length > 0) {
    throw new Error(`Conflicting style property assignments detected:\n${conflicts.join('\n')}`);
  }
}

/**
 * Process edges using the new semantic→style mapping system
 */
function processWithSemanticMappings(
  edgeProperties: string[],
  styleConfig: EdgeStyleConfig
): ProcessedEdgeStyle {
  if (!styleConfig.semanticMappings) {
    return getDefaultStyle();
  }

  // Validate the semantic mappings configuration
  validateSemanticMappings(styleConfig.semanticMappings);

  // Collect all style settings from matching semantic properties
  const styleSettings: Record<string, string | number> = {};
  const appliedProperties: string[] = [];

  // Process each group
  for (const [groupName, group] of Object.entries(styleConfig.semanticMappings)) {
    // Find which option in this group matches our edge properties (if any)
    for (const [optionName, styleMapping] of Object.entries(group)) {
      if (edgeProperties.includes(optionName)) {
        // Apply all style settings from this option
        Object.assign(styleSettings, styleMapping);
        appliedProperties.push(optionName);
        break; // Only one option per group can match
      }
    }
  }

  // Convert style settings to ReactFlow format
  if (Object.keys(styleSettings).length > 0) {
    return convertStyleSettingsToReactFlow(styleSettings, appliedProperties);
  }

  return getDefaultStyle();
}

/**
 * Convert semantic style settings to ReactFlow visual format
 */
function convertStyleSettingsToReactFlow(
  styleSettings: Record<string, string | number>,
  appliedProperties: string[]
): ProcessedEdgeStyle {
  let style: Record<string, any> = {};
  let animated = false;
  let label = appliedProperties.join(',');
  let markerEndSpec: any | undefined = undefined;

  // Apply line-pattern
  const linePattern = styleSettings['line-pattern'] as string;
  if (linePattern) {
    switch (linePattern) {
      case 'solid':
        style.strokeDasharray = undefined;
        break;
      case 'dashed':
        style.strokeDasharray = '8,4';
        break;
      case 'dotted':
        style.strokeDasharray = '2,2';
        break;
      case 'dash-dot':
        style.strokeDasharray = '8,2,2,2';
        break;
    }
  }

  // Apply line-width
  const lineWidth = styleSettings['line-width'] as number;
  if (lineWidth) {
    style.strokeWidth = lineWidth;
  }

  // Apply animation
  const animation = styleSettings['animation'] as string;
  if (animation === 'animated') {
    animated = true;
  }

  // Apply line-style (single/double)
  const lineStyle = styleSettings['line-style'] as string;
  if (lineStyle === 'double') {
    // For double lines, we'll use a dash pattern that looks like double lines
    style.strokeDasharray = '10,2,2,2';
  }

  // Apply waviness (path-based approach)
  const waviness = styleSettings['waviness'] as string;
  if (waviness === 'wavy') {
    // Use custom path generation instead of SVG filter
    style.waviness = 'wavy';
  }

  // Apply halo (pass halo info to edge component)
  const halo = styleSettings['halo'] as string;
  let strokeColor = '#666666'; // default color
  let haloColor: string | undefined = undefined;
  
  if (halo && halo !== 'none') {
    // Map halo types to colors
    const haloColors = {
      'light-blue': '#4a90e2',
      'light-red': '#e74c3c',
      'light-green': '#27ae60'
    };
    
    // Store halo color for edge component to use
    haloColor = haloColors[halo as keyof typeof haloColors];
    // Keep default stroke color for the main edge
  }

    // Apply arrowhead
  const arrowhead = styleSettings['arrowhead'] as string;
  if (arrowhead && arrowhead !== 'none') {
    // Map to React Flow marker types including custom markers
    switch (arrowhead) {
      case 'triangle-open':
        markerEndSpec = { type: 'arrow' }; // Built-in open triangle
        break;
      case 'triangle-filled':
        markerEndSpec = { type: 'arrowclosed' }; // Built-in filled triangle
        break;
      case 'circle-filled':
        // Use custom SVG marker for filled circles (Singleton)
        markerEndSpec = 'url(#circle-filled)';
        break;
      case 'diamond-open':
        // Use custom SVG marker for open diamonds (Optional)
        markerEndSpec = 'url(#diamond-open)';
        break;
    }
  }

  return {
    reactFlowType: 'standard',
    style: {
      stroke: strokeColor,
      strokeWidth: 2,
      ...(haloColor && { haloColor }), // Pass halo color to edge component
      ...style
    },
    animated,
    label,
    appliedProperties,
  markerEndSpec,
  lineStyle: (lineStyle === 'double' ? 'double' : 'single')
  };
}

/**
 * Process edges using the new boolean property pair system
 */
function processWithBooleanPairs(
  edgeProperties: string[],
  styleConfig: EdgeStyleConfig
): ProcessedEdgeStyle {
  const styleTags: string[] = [];
  const appliedProperties: string[] = [];

  // First, handle boolean property pairs
  if (styleConfig.booleanPropertyPairs) {
    for (const pairConfig of styleConfig.booleanPropertyPairs) {
      const [defaultProp, altProp] = pairConfig.pair;
      
      if (edgeProperties.includes(altProp)) {
        styleTags.push(pairConfig.altStyle);
        appliedProperties.push(altProp);
      } else if (edgeProperties.includes(defaultProp)) {
        styleTags.push(pairConfig.defaultStyle);
        appliedProperties.push(defaultProp);
      }
    }
  }

  // Then, handle single property mappings
  if (styleConfig.singlePropertyMappings) {
    for (const property of edgeProperties) {
      const mapping = styleConfig.singlePropertyMappings[property];
      if (mapping && !appliedProperties.includes(property)) {
        styleTags.push(mapping);
        appliedProperties.push(property);
      }
    }
  }

  // Combine all style tags
  if (styleTags.length > 0) {
    return combineStyleTagsIntelligently(styleTags, appliedProperties);
  }

  // Fallback to default
  return getDefaultStyle();
}

/**
 * Process edges using legacy propertyMappings format
 */
function processLegacyMappings(
  edgeProperties: string[],
  styleConfig: EdgeStyleConfig
): ProcessedEdgeStyle {
  if (!styleConfig.propertyMappings) {
    return getDefaultStyle();
  }

  // Collect all style tags from the properties
  const styleTags: string[] = [];
  
  for (const property of edgeProperties) {
    const mapping = styleConfig.propertyMappings[property];
    if (mapping) {
      if (typeof mapping === 'string') {
        styleTags.push(mapping);
      } else if (mapping.styleTag) {
        styleTags.push(mapping.styleTag);
      }
    }
  }
  
  // If we have style tags, combine them with intelligent CSS property handling
  if (styleTags.length > 0) {
    return combineStyleTagsIntelligently(styleTags, edgeProperties);
  }
  
  // Fallback: find the first property with any mapping
  const selectedProperty = edgeProperties.find(prop => styleConfig.propertyMappings![prop]) || null;

  if (selectedProperty && styleConfig.propertyMappings[selectedProperty]) {
    const mapping = styleConfig.propertyMappings[selectedProperty];
    
    // Handle backward compatibility with full style objects
    if (typeof mapping === 'object' && mapping.style) {
      return {
        reactFlowType: 'standard',
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
    // New numbered edge style system with boolean pairs
    // Each pair uses different visual properties that can merge cleanly
    
    // Style 1 pair: Line pattern (ordering)
    'edge_style_1': {
      style: { strokeDasharray: undefined }, // solid line
      animated: false,
      label: '1'
    },
    'edge_style_1_alt': {
      style: { strokeDasharray: '4,4' }, // dashed line  
      animated: false,
      label: '1*'
    },
    
    // Style 2 pair: Line thickness (bounds)
    'edge_style_2': {
      style: { strokeWidth: 1 }, // thin
      animated: false,
      label: '2'
    },
    'edge_style_2_alt': {
      style: { strokeWidth: 3 }, // thick
      animated: false,
      label: '2*'
    },
    
    // Style 3 pair: Animation (scope)
    'edge_style_3': {
      style: {},
      animated: false,
      label: '3'
    },
    'edge_style_3_alt': {
      style: {},
      animated: true,
      label: '3*'
    },
    
    // Single properties: Double line (keyed), wavy (cycle)
    'edge_style_4': {
      style: { strokeDasharray: '8,2,2,2' }, // double-line pattern
      animated: false,
      label: '4'
    },
    'edge_style_5': {
      style: { strokeDasharray: '2,2' }, // dotted for cycles
      animated: true,
      label: '5'
    },
    
    // Legacy compound visual styles (for backward compatibility)
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
      reactFlowType: 'standard',
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
    reactFlowType: 'standard',
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
 * Combine style tags using intelligent merging rules to handle conflicts
 */
function combineStyleTagsWithPriority(styleTags: string[], originalProperties: string[], styleConfig: EdgeStyleConfig): ProcessedEdgeStyle {
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
    
    // Assign priority based on boolean pair order (lower index = higher priority)
    let priority = 999;
    if (styleConfig.booleanPropertyPairs) {
      const pairIndex = styleConfig.booleanPropertyPairs.findIndex(pair => 
        pair.defaultStyle === tag || pair.altStyle === tag
      );
      if (pairIndex !== -1) {
        priority = pairIndex;
      }
    }
    
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
  
  // For each CSS property, use the tag with highest priority (lowest index)
  for (const [cssProp, candidates] of Object.entries(styleGroups)) {
    // Sort by priority (lowest number = highest priority)
    candidates.sort((a, b) => a.priority - b.priority);
    const winningTag = candidates[0].tag;
    
    // Apply the winning tag's style for this property
    const tagStyle = mapStyleTagToVisual(winningTag, []);
    if (tagStyle.style[cssProp] !== undefined) {
      combinedStyle[cssProp] = tagStyle.style[cssProp];
    }
  }
  
  return {
    reactFlowType: 'standard',
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
  
  return {
    reactFlowType: 'standard',
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
    reactFlowType: 'standard',
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
    reactFlowType: 'standard',
    style: combinedStyle,
    animated: animated,
    label: labels.join(''),
    appliedProperties: originalProperties
  };
}