/**
 * Shared configuration and utilities for ReactFlow components
 */

import { generateNodeColors } from './utils.js';

// ELK layout configurations
export const ELK_LAYOUT_CONFIGS = {
  mrtree: {
    'elk.algorithm': 'mrtree',
    'elk.direction': 'DOWN',
    'elk.spacing.nodeNode': 50,
    'elk.spacing.edgeNode': 20,
  },
  layered: {
    'elk.algorithm': 'layered',
    'elk.direction': 'DOWN',
    'elk.spacing.nodeNode': 30,
    'elk.layered.spacing.nodeNodeBetweenLayers': 50,
  },
  force: {
    'elk.algorithm': 'force',
    'elk.spacing.nodeNode': 100,
  },
  stress: {
    'elk.algorithm': 'stress',
    'elk.spacing.nodeNode': 100,
  },
  radial: {
    'elk.algorithm': 'radial',
    'elk.spacing.nodeNode': 100,
  },
};

// Common ReactFlow configuration
export const REACTFLOW_CONFIG = {
  fitView: true,
  nodesDraggable: true,
  nodesConnectable: true,
  elementsSelectable: true,
  maxZoom: 2,
  minZoom: 0.1,
  nodeOrigin: [0.5, 0.5],
  elevateEdgesOnSelect: true,
  disableKeyboardA11y: false,
};

// Common MiniMap configuration
export const MINIMAP_CONFIG = {
  nodeStrokeWidth: 2,
  nodeStrokeColor: "#666",
  maskColor: "rgba(240, 240, 240, 0.6)",
};

// Common Background configuration
export const BACKGROUND_CONFIG = {
  color: "#f5f5f5",
  gap: 20,
};

// Default edge options
export const DEFAULT_EDGE_OPTIONS = {
  type: 'smoothstep',
  animated: false,
  style: {
    strokeWidth: 2,
    stroke: '#666666',
  },
  markerEnd: {
    type: 'arrowclosed',
    width: 20,
    height: 20,
    color: '#666666',
  },
};

// Default node style configuration
export const DEFAULT_NODE_STYLE = {
  borderRadius: '8px',
  padding: '10px',
  color: '#333',
  fontSize: '12px',
  fontWeight: '500',
  width: 200,
  height: 60,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  textAlign: 'center',
};

/**
 * Create styled node from raw node data
 */
export function createStyledNode(node, colorPalette = 'Set3') {
  const nodeColors = generateNodeColors(node.data?.nodeType || 'Transform', colorPalette);
  
  return {
    ...node,
    position: { x: 0, y: 0 }, // Will be set by layout
    style: {
      ...DEFAULT_NODE_STYLE,
      background: nodeColors.gradient,
      border: `2px solid ${nodeColors.border}`,
    },
  };
}

/**
 * Create styled edge from raw edge data
 */
export function createStyledEdge(edge) {
  return {
    ...edge,
    ...DEFAULT_EDGE_OPTIONS,
  };
}

/**
 * Get node color for MiniMap
 */
export function getMiniMapNodeColor(node, colorPalette = 'Set3') {
  const nodeColors = generateNodeColors(
    node.data?.nodeType || node.data?.type || 'Transform', 
    colorPalette
  );
  return nodeColors.primary;
}

/**
 * Process graph data into styled nodes and edges
 */
export async function processGraphData(graphData, colorPalette, currentLayout, applyLayout) {
  if (!graphData?.nodes?.length) {
    return { nodes: [], edges: [] };
  }

  const processedNodes = graphData.nodes.map(node => createStyledNode(node, colorPalette));
  const processedEdges = (graphData.edges || []).map(edge => createStyledEdge(edge));

  // Apply layout
  const layoutResult = await applyLayout(processedNodes, processedEdges, currentLayout);
  
  return {
    nodes: layoutResult.nodes,
    edges: layoutResult.edges,
  };
}
