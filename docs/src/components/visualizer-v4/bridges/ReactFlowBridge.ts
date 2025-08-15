/**
 * @fileoverview ReactFlow Bridge - Pure transformation between VisualizationState and ReactFlow format
 * 
 * This bridge is now a stateless, pure transformation layer:
 * - NO business logic - only format translation  
 * - NO parent-child mapping logic - VisualizationEngine provides the mapping
 * - NO handle assignment logic - VisualizationEngine decides handles
 * - Focuses solely on converting between VisualizationState and ReactFlow formats
 */

import type { VisualizationState } from '../core/VisState';
import { CoordinateTranslator, type ContainerInfo } from './CoordinateTranslator';
import { MarkerType } from '@xyflow/react';

// ReactFlow types
export interface ReactFlowNode {
  id: string;
  type: 'standard' | 'container';
  position: { x: number; y: number };
  data: {
    label: string;
    style: string;
    collapsed?: boolean;
    width?: number;
    height?: number;
    [key: string]: any;
  };
  style?: {
    width?: number;
    height?: number;
  };
  parentId?: string;
}

export interface ReactFlowEdge {
  id: string;
  type: 'standard' | 'hyper';
  source: string;
  target: string;
  sourceHandle?: string;
  targetHandle?: string;
  markerEnd?: {
    type: typeof MarkerType.ArrowClosed;
    width: number;
    height: number;
    color: string;
  };
  data: {
    style: string;
  };
}

export interface ReactFlowData {
  nodes: ReactFlowNode[];
  edges: ReactFlowEdge[];
}

/**
 * Pure ReactFlow transformation functions - no state, no business logic
 */
export class ReactFlowBridge {
  /**
   * Convert positioned VisualizationState to ReactFlow format (pure transformation)
   * Parent-child mapping and handle assignment are provided by VisualizationEngine
   */
  static visStateToReactFlow(
    visState: VisualizationState, 
    parentChildMap: Map<string, string>,
    edgeHandles: Map<string, { sourceHandle?: string; targetHandle?: string }>,
    colorPalette: string = 'Set3'
  ): ReactFlowData {
    const nodes: ReactFlowNode[] = [];
    const edges: ReactFlowEdge[] = [];
    
    // Convert containers to ReactFlow nodes
    ReactFlowBridge.convertContainers(visState, nodes, parentChildMap, colorPalette);
    
    // Convert regular nodes to ReactFlow nodes  
    ReactFlowBridge.convertNodes(visState, nodes, parentChildMap, colorPalette);
    
    // Convert edges to ReactFlow edges
    ReactFlowBridge.convertEdges(visState, edges, edgeHandles);
    
    return { nodes, edges };
  }

  /**
   * Convert containers to ReactFlow container nodes (pure transformation)
   */
  private static convertContainers(
    visState: VisualizationState, 
    nodes: ReactFlowNode[], 
    parentChildMap: Map<string, string>,
    colorPalette: string
  ): void {
    visState.visibleContainers.forEach(container => {
      const parentId = parentChildMap.get(container.id);
      
      // Get coordinates from VisualizationState
      const elkCoords = {
        x: container.x || 0,
        y: container.y || 0
      };
      
      // Convert ELK coordinates to ReactFlow coordinates
      const parentContainer = parentId ? 
        CoordinateTranslator.getContainerInfo(parentId, visState) : 
        undefined;
      
      const position = CoordinateTranslator.elkToReactFlow(elkCoords, parentContainer);
      
      // Use dimensions from VisualizationState
      const effectiveDimensions = visState.getContainerAdjustedDimensions(container.id);
      const width = effectiveDimensions.width;
      const height = effectiveDimensions.height;
      
      // Calculate node count for collapsed containers
      const nodeCount = container.collapsed ? 
        visState.getContainerChildren(container.id)?.size || 0 : 0;
      
      const containerNode: ReactFlowNode = {
        id: container.id,
        type: 'container',
        position,
        data: {
          label: (container as any).data?.label || (container as any).label || container.id,
          style: (container as any).style || 'default',
          collapsed: container.collapsed || false,
          colorPalette,
          nodeCount,
          width,
          height,
          // Pass through any custom properties
          ...ReactFlowBridge.extractCustomProperties(container as any)
        },
        style: {
          width,
          height
        },
        parentId
      };
      
      nodes.push(containerNode);
    });
  }

  /**
   * Convert regular nodes to ReactFlow standard nodes (pure transformation)
   */
  private static convertNodes(
    visState: VisualizationState, 
    nodes: ReactFlowNode[], 
    parentChildMap: Map<string, string>,
    colorPalette: string
  ): void {
    visState.visibleNodes.forEach(node => {
      const parentId = parentChildMap.get(node.id);
      
      // Get coordinates from VisualizationState
      const nodeLayout = visState.getNodeLayout(node.id);
      const elkCoords = {
        x: nodeLayout?.position?.x || node.x || 0,
        y: nodeLayout?.position?.y || node.y || 0
      };
      
      // Convert ELK coordinates to ReactFlow coordinates
      const parentContainer = parentId ? 
        CoordinateTranslator.getContainerInfo(parentId, visState) : 
        undefined;
      
      const position = CoordinateTranslator.elkToReactFlow(elkCoords, parentContainer);
      
      const standardNode: ReactFlowNode = {
        id: node.id,
        type: 'standard',
        position,
        data: {
          label: node.label || node.id,
          style: node.style || 'default',
          colorPalette,
          // Pass through any custom properties
          ...ReactFlowBridge.extractCustomProperties(node)
        },
        parentId
      };
      
      nodes.push(standardNode);
    });
  }

  /**
   * Convert edges to ReactFlow edges (pure transformation)
   */
  private static convertEdges(
    visState: VisualizationState, 
    edges: ReactFlowEdge[],
    edgeHandles: Map<string, { sourceHandle?: string; targetHandle?: string }>
  ): void {
    visState.visibleEdges.forEach(edge => {
      const handles = edgeHandles.get(edge.id) || {};
      
      const reactFlowEdge: ReactFlowEdge = {
        id: edge.id,
        type: 'standard',
        source: edge.source,
        target: edge.target,
        markerEnd: {
          type: MarkerType.ArrowClosed,
          width: 15,
          height: 15,
          color: '#999'
        },
        data: {
          style: edge.style || 'default'
        }
      };
      
      // Apply handles if provided by VisualizationEngine
      if (handles.sourceHandle) {
        reactFlowEdge.sourceHandle = handles.sourceHandle;
      }
      if (handles.targetHandle) {
        reactFlowEdge.targetHandle = handles.targetHandle;
      }
      
      edges.push(reactFlowEdge);
    });
  }

  /**
   * Extract custom properties from graph elements (pure transformation)
   */
  private static extractCustomProperties(element: any): Record<string, any> {
    const customProps: Record<string, any> = {};
    
    // Filter out known properties to get custom ones
    const knownProps = new Set([
      'id', 'label', 'style', 'hidden', 'layout', 
      'source', 'target', 'children', 'collapsed',
      'x', 'y', 'width', 'height'
    ]);
    
    Object.entries(element).forEach(([key, value]) => {
      if (!knownProps.has(key)) {
        customProps[key] = value;
      }
    });
    
    return customProps;
  }
}