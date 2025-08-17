/**
 * Edge Bridge
 * 
 * Converts visualization state edges to ReactFlow edges with proper styling
 * based on edge properties and style configuration.
 */

import { Edge as ReactFlowEdge } from '@xyflow/react';
import { GraphEdge, HyperEdge, Edge } from '../core/types';
import { processEdgeStyle, createEdgeLabel, EdgeStyleConfig } from '../core/EdgeStyleProcessor';

export interface EdgeBridgeOptions {
  edgeStyleConfig?: EdgeStyleConfig;
  showPropertyLabels?: boolean;
  enableAnimations?: boolean;
}

/**
 * Convert a visualization state edge to a ReactFlow edge
 */
export function convertEdgeToReactFlow(
  edge: Edge,
  options: EdgeBridgeOptions = {}
): ReactFlowEdge {
  const { edgeStyleConfig, showPropertyLabels = true, enableAnimations = true } = options;
  
  // Extract edge properties from the edge data
  const edgeProperties = (edge as any).edgeProperties || (edge as any).semanticTags || [];
  const originalLabel = (edge as any).label;
  
  // Process the edge style based on properties
  const processedStyle = processEdgeStyle(edgeProperties, edgeStyleConfig);
  
  // Create label if requested
  const label = showPropertyLabels 
    ? createEdgeLabel(edgeProperties, edgeStyleConfig, originalLabel)
    : originalLabel;
  
  // Build the ReactFlow edge
  const reactFlowEdge: ReactFlowEdge = {
    id: edge.id,
    source: edge.source,
    target: edge.target,
    type: processedStyle.reactFlowType,
    style: processedStyle.style,
    animated: enableAnimations && processedStyle.animated,
    label: label,
    data: {
      edgeProperties,
      appliedProperties: processedStyle.appliedProperties,
      originalEdge: edge
    }
  };
  
  // Add any additional properties from the original edge
  if (edge.hidden) {
    reactFlowEdge.hidden = edge.hidden;
  }
  
  return reactFlowEdge;
}

/**
 * Convert multiple edges to ReactFlow format
 */
export function convertEdgesToReactFlow(
  edges: Edge[],
  options: EdgeBridgeOptions = {}
): ReactFlowEdge[] {
  return edges.map(edge => convertEdgeToReactFlow(edge, options));
}

/**
 * Get edge style statistics for debugging/analysis
 */
export function getEdgeStyleStats(
  edges: Edge[],
  edgeStyleConfig?: EdgeStyleConfig
): {
  totalEdges: number;
  propertyCounts: Record<string, number>;
  styleCounts: Record<string, number>;
  unmappedProperties: string[];
} {
  const propertyCounts: Record<string, number> = {};
  const styleCounts: Record<string, number> = {};
  const unmappedProperties = new Set<string>();
  
  for (const edge of edges) {
    const edgeProperties = (edge as any).edgeProperties || (edge as any).semanticTags || [];
    
    // Count properties
    for (const prop of edgeProperties) {
      propertyCounts[prop] = (propertyCounts[prop] || 0) + 1;
      
      // Check if property has a mapping
      if (edgeStyleConfig && !edgeStyleConfig.propertyMappings[prop]) {
        unmappedProperties.add(prop);
      }
    }
    
    // Count applied styles
    const processedStyle = processEdgeStyle(edgeProperties, edgeStyleConfig);
    const styleKey = `${processedStyle.reactFlowType}:${JSON.stringify(processedStyle.style)}`;
    styleCounts[styleKey] = (styleCounts[styleKey] || 0) + 1;
  }
  
  return {
    totalEdges: edges.length,
    propertyCounts,
    styleCounts,
    unmappedProperties: Array.from(unmappedProperties)
  };
}