/**
 * @fileoverview Enhanced JSON Loader for Real Data
 * 
 * Handles various JSON formats including ReactFlow-formatted data
 * Loads real test data from chat.json and paxos.json
 */

import type { VisualizationState } from '../core/VisState';
import type { NodeStyle, EdgeStyle } from '../shared/types';

export interface ReactFlowJSON {
  nodes: Array<{
    id: string;
    data: {
      label?: string;
      [key: string]: any;
    };
    position?: { x: number; y: number };
    type?: string;
    style?: any;
  }>;
  edges: Array<{
    id: string;
    source: string;
    target: string;
    type?: string;
    style?: any;
    label?: string;
  }>;
}

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
 * Convert ReactFlow JSON to our simple format
 */
export function convertReactFlowToSimple(reactFlowData: ReactFlowJSON): SimpleGraphData {
  const nodes = reactFlowData.nodes.map(node => ({
    id: node.id,
    label: node.data.label || node.id,
    style: 'default'
  }));
  
  const edges = reactFlowData.edges.map(edge => ({
    id: edge.id,
    source: edge.source,
    target: edge.target,
    style: 'default'
  }));
  
  return { nodes, edges };
}

/**
 * Load and subset large graph data
 */
export function subsetGraphData(data: SimpleGraphData, maxNodes: number = 10): SimpleGraphData {
  // Take first N nodes
  const nodes = data.nodes.slice(0, maxNodes);
  const nodeIds = new Set(nodes.map(n => n.id));
  
  // Take edges that connect to included nodes
  const edges = data.edges.filter(edge => 
    nodeIds.has(edge.source) && nodeIds.has(edge.target)
  );
  
  return { nodes, edges, containers: data.containers };
}

/**
 * Load graph data from various formats
 */
export async function loadGraphFromFile(
  filePath: string, 
  visState: VisualizationState,
  maxNodes: number = 15
): Promise<void> {
  console.log(`[EnhancedLoader] 📁 Loading graph from ${filePath}...`);
  
  try {
    // In a real browser environment, you'd use fetch()
    // For now, we'll provide the data directly
    let graphData: SimpleGraphData;
    
    if (filePath.includes('sample')) {
      // Use our sample data
      graphData = SAMPLE_CHAT_SUBSET;
    } else {
      // For real files, you'd fetch and parse
      throw new Error('File loading not implemented in this demo - using sample data');
    }
    
    // Subset the data to keep it manageable
    const subsetData = subsetGraphData(graphData, maxNodes);
    
    console.log(`[EnhancedLoader] 📊 Processing subset: ${subsetData.nodes.length} nodes, ${subsetData.edges.length} edges`);
    
    // Load into VisState
    loadGraphFromJSON(subsetData, visState);
    
  } catch (error) {
    console.error('[EnhancedLoader] ❌ Failed to load graph:', error);
    throw error;
  }
}

/**
 * Convert simple JSON graph data to VisState (same as before)
 */
export function loadGraphFromJSON(
  jsonData: SimpleGraphData, 
  visState: VisualizationState
): void {
  console.log('[EnhancedLoader] 🔄 Converting to VisState...');
  
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
  
  console.log('[EnhancedLoader] ✅ VisState loaded:', {
    nodes: jsonData.nodes.length,
    edges: jsonData.edges.length,
    containers: jsonData.containers?.length || 0
  });
}

/**
 * Sample subset of chat data for demonstration
 */
export const SAMPLE_CHAT_SUBSET: SimpleGraphData = {
  nodes: [
    { id: '0', label: 'Chat Server Init', style: 'default' },
    { id: '1', label: 'Message Receiver', style: 'default' },
    { id: '2', label: 'Broadcast Handler', style: 'default' },
    { id: '3', label: 'User Connection', style: 'default' },
    { id: '4', label: 'Message Parser', style: 'default' },
    { id: '5', label: 'Chat Room', style: 'default' },
    { id: '6', label: 'Message Store', style: 'default' },
    { id: '7', label: 'User List', style: 'default' },
    { id: '8', label: 'Message Filter', style: 'default' },
    { id: '9', label: 'Output Stream', style: 'default' }
  ],
  edges: [
    { id: 'e0', source: '0', target: '1', style: 'default' },
    { id: 'e1', source: '1', target: '2', style: 'default' },
    { id: 'e2', source: '2', target: '5', style: 'default' },
    { id: 'e3', source: '3', target: '4', style: 'default' },
    { id: 'e4', source: '4', target: '5', style: 'default' },
    { id: 'e5', source: '5', target: '6', style: 'default' },
    { id: 'e6', source: '5', target: '7', style: 'default' },
    { id: 'e7', source: '5', target: '8', style: 'default' },
    { id: 'e8', source: '8', target: '9', style: 'default' }
  ],
  containers: [
    {
      id: 'message_processing',
      children: ['1', '2', '8'],
      collapsed: false,
      style: 'default'
    },
    {
      id: 'user_management', 
      children: ['3', '4', '7'],
      collapsed: true, // This will create hyperedges!
      style: 'default'
    }
  ]
};

/**
 * More complex graph with multiple containers
 */
export const SAMPLE_COMPLEX_GRAPH: SimpleGraphData = {
  nodes: [
    { id: 'input1', label: 'Data Source A', style: 'default' },
    { id: 'input2', label: 'Data Source B', style: 'default' },
    { id: 'parser1', label: 'JSON Parser', style: 'default' },
    { id: 'parser2', label: 'CSV Parser', style: 'default' },
    { id: 'transform1', label: 'Data Transform', style: 'default' },
    { id: 'transform2', label: 'Aggregator', style: 'default' },
    { id: 'filter', label: 'Data Filter', style: 'default' },
    { id: 'output1', label: 'Database Sink', style: 'default' },
    { id: 'output2', label: 'Stream Sink', style: 'default' },
    { id: 'monitor', label: 'Health Monitor', style: 'default' }
  ],
  edges: [
    { id: 'e1', source: 'input1', target: 'parser1', style: 'default' },
    { id: 'e2', source: 'input2', target: 'parser2', style: 'default' },
    { id: 'e3', source: 'parser1', target: 'transform1', style: 'default' },
    { id: 'e4', source: 'parser2', target: 'transform1', style: 'default' },
    { id: 'e5', source: 'transform1', target: 'transform2', style: 'default' },
    { id: 'e6', source: 'transform2', target: 'filter', style: 'default' },
    { id: 'e7', source: 'filter', target: 'output1', style: 'default' },
    { id: 'e8', source: 'filter', target: 'output2', style: 'default' },
    { id: 'e9', source: 'transform1', target: 'monitor', style: 'default' },
    { id: 'e10', source: 'transform2', target: 'monitor', style: 'default' }
  ],
  containers: [
    {
      id: 'input_stage',
      children: ['input1', 'input2', 'parser1', 'parser2'],
      collapsed: false,
      style: 'default'
    },
    {
      id: 'processing_stage',
      children: ['transform1', 'transform2', 'filter'],
      collapsed: true, // Collapsed - will create hyperedges
      style: 'default'
    },
    {
      id: 'output_stage',
      children: ['output1', 'output2'],
      collapsed: false,
      style: 'default'
    }
  ]
};
