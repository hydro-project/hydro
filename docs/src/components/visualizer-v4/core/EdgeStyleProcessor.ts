/**
 * Edge Style Processor
 * 
 * Processes edge properties and applies appropriate visual styles based on
 * the edgeStyleConfig from the JSON data.
 */

import { EdgeStyle } from '../shared/config';

export interface EdgeStyleConfig {
  propertyMappings: Record<string, {
    reactFlowType: string;
    style: Record<string, any>;
    animated?: boolean;
    label?: string;
  }>;
  defaultStyle: {
    reactFlowType: string;
    style: Record<string, any>;
    animated?: boolean;
  };
  combinationRules: {
    priority: string[];
    description: string;
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
  if (!styleConfig || !edgeProperties || edgeProperties.length === 0) {
    return {
      reactFlowType: 'default',
      style: {
        stroke: '#999999',
        strokeWidth: 2
      },
      animated: false,
      appliedProperties: []
    };
  }

  // Find the highest priority property that has a mapping
  const priorityOrder = styleConfig.combinationRules.priority;
  let selectedProperty: string | null = null;
  
  for (const priority of priorityOrder) {
    if (edgeProperties.includes(priority) && styleConfig.propertyMappings[priority]) {
      selectedProperty = priority;
      break;
    }
  }

  // If no priority property found, use the first property that has a mapping
  if (!selectedProperty) {
    selectedProperty = edgeProperties.find(prop => styleConfig.propertyMappings[prop]) || null;
  }

  // Apply the selected property's style or default
  if (selectedProperty && styleConfig.propertyMappings[selectedProperty]) {
    const mapping = styleConfig.propertyMappings[selectedProperty];
    return {
      reactFlowType: mapping.reactFlowType,
      style: { ...mapping.style },
      animated: mapping.animated || false,
      label: mapping.label,
      appliedProperties: [selectedProperty]
    };
  }

  // Fallback to default style
  return {
    reactFlowType: styleConfig.defaultStyle.reactFlowType,
    style: { ...styleConfig.defaultStyle.style },
    animated: styleConfig.defaultStyle.animated || false,
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