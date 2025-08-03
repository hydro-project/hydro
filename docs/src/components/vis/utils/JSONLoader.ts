/**
 * @fileoverview Simple JSON Data Loader
 * 
 * Loads graph data from JSON and converts it to VisState format
 * Minimal implementation for demonstration purposes
 */

import type { VisualizationState } from '../core/VisState';
import type { NodeStyle, EdgeStyle } from '../shared/types';

export interface SimpleGraphData {
  nodes: Array<{
    id: string;
    label?: string;
    style?: string;
  }>;
  edges: Array<{
    id: string;
    source: string;
    target: string;
    style?: string;
  }>;
  containers?: Array<{
    id: string;
    children: string[];
    collapsed?: boolean;
    style?: string;
  }>;
}

/**
 * Convert simple JSON graph data to VisState
 */
export function loadGraphFromJSON(
  jsonData: SimpleGraphData, 
  visState: VisualizationState
): void {
  console.log('[JSONLoader] 📁 Loading graph data...');
  
  // Clear existing data
  visState.clear();
  
  // Add nodes
  jsonData.nodes.forEach(nodeData => {
    visState.setGraphNode(nodeData.id, {
      label: nodeData.label || nodeData.id,
      x: 0, // Will be set by layout
      y: 0, // Will be set by layout
      width: 180,
      height: 60,
      hidden: false,
      style: (nodeData.style || 'default') as NodeStyle
    });
  });
  
  // Add edges
  jsonData.edges.forEach(edgeData => {
    visState.setGraphEdge(edgeData.id, {
      source: edgeData.source,
      target: edgeData.target,
      hidden: false,
      style: (edgeData.style || 'default') as EdgeStyle
    });
  });
  
  // Add containers if provided
  if (jsonData.containers) {
    jsonData.containers.forEach(containerData => {
      visState.setContainer(containerData.id, {
        collapsed: containerData.collapsed || false,
        hidden: false,
        children: containerData.children,
        style: containerData.style || 'default'
      });
    });
  }
  
  console.log('[JSONLoader] ✅ Graph data loaded:', {
    nodes: jsonData.nodes.length,
    edges: jsonData.edges.length,
    containers: jsonData.containers?.length || 0
  });
}

/**
 * Sample data for testing
 */
export const SAMPLE_GRAPH_DATA: SimpleGraphData = {
  nodes: [
    { id: 'source1', label: 'Source A', style: 'default' },
    { id: 'source2', label: 'Source B', style: 'default' },
    { id: 'transform1', label: 'Transform', style: 'default' },
    { id: 'sink1', label: 'Sink', style: 'default' }
  ],
  edges: [
    { id: 'edge1', source: 'source1', target: 'transform1', style: 'default' },
    { id: 'edge2', source: 'source2', target: 'transform1', style: 'default' },
    { id: 'edge3', source: 'transform1', target: 'sink1', style: 'default' }
  ],
  containers: [
    {
      id: 'input_container',
      children: ['source1', 'source2'],
      collapsed: false,
      style: 'default'
    }
  ]
};

/**
 * Sample data with collapsed container (to test hyperedge fix)
 */
export const SAMPLE_COLLAPSED_GRAPH: SimpleGraphData = {
  nodes: [
    { id: 'source1', label: 'Source A', style: 'default' },
    { id: 'source2', label: 'Source B', style: 'default' },
    { id: 'external_node', label: 'External Node', style: 'default' }
  ],
  edges: [
    { id: 'edge1', source: 'source1', target: 'source2', style: 'default' },
    { id: 'edge2', source: 'source2', target: 'external_node', style: 'default' }
  ],
  containers: [
    {
      id: 'collapsed_container',
      children: ['source1', 'source2'],
      collapsed: true, // This should create a hyperedge to external_node
      style: 'default'
    }
  ]
};
