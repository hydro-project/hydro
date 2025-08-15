/**
 * @fileoverview ReactFlow Bridge - Refactored to be DRY, stateless, and focused on format translation
 * 
 * This bridge now separates business logic from format transformation while maintaining
 * backward compatibility with existing tests and components.
 */

import type { VisualizationState } from '../core/VisState';
import { CoordinateTranslator, type ContainerInfo } from './CoordinateTranslator';
import { MarkerType } from '@xyflow/react';
import { getHandleConfig } from '../render/handleConfig';

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

export class ReactFlowBridge {
  private colorPalette: string = 'Set3';

  /**
   * Set the color palette for node styling
   */
  setColorPalette(palette: string): void {
    this.colorPalette = palette;
  }

  /**
   * Convert positioned VisState data to ReactFlow format
   * Now cleaner with better separation of concerns
   */
  visStateToReactFlow(visState: VisualizationState): ReactFlowData {
    console.log('[ReactFlowBridge] 🔄 Converting VisState to ReactFlow format');
    
    const nodes: ReactFlowNode[] = [];
    const edges: ReactFlowEdge[] = [];
    
    // Build parent-child mapping using VisualizationState's official API
    const parentMap = this.buildParentMap(visState);
    
    // Convert all components using pure transformation functions
    this.convertContainers(visState, nodes, parentMap);
    this.convertNodes(visState, nodes, parentMap);
    this.convertEdges(visState, edges);
    
    console.log(`[ReactFlowBridge] ✅ Generated ${nodes.length} nodes, ${edges.length} edges`);
    return { nodes, edges };
  }

  /**
   * Build parent-child relationship map using VisualizationState's official API
   * This logic could be moved to VisualizationEngine in the future for better separation
   */
  private buildParentMap(visState: VisualizationState): Map<string, string> {
    const parentMap = new Map<string, string>();
    
    // Map nodes to their parent containers (only if parent is expanded)
    for (const node of visState.visibleNodes) {
      const parentContainer = visState.getNodeContainer(node.id);
      if (parentContainer) {
        const container = visState.getContainer(parentContainer);
        // Only include if parent container is expanded (not collapsed)
        if (container && !container.collapsed && !container.hidden) {
          parentMap.set(node.id, parentContainer);
        }
      }
    }
    
    // Map containers to their parent containers (for nested containers)
    for (const container of visState.visibleContainers) {
      const parentContainer = this.findContainerParent(container.id, visState);
      if (parentContainer) {
        const parentObj = visState.getContainer(parentContainer);
        // Only include if parent container is expanded
        if (parentObj && !parentObj.collapsed && !parentObj.hidden) {
          parentMap.set(container.id, parentContainer);
        }
      }
    }
    
    return parentMap;
  }

  /**
   * Find the parent container for a given container - pure logic helper
   */
  private findContainerParent(containerId: string, visState: VisualizationState): string | undefined {
    for (const container of visState.visibleContainers) {
      const children = visState.getContainerChildren(container.id);
      if (children && children.has(containerId)) {
        return container.id;
      }
    }
    return undefined;
  }

  /**
   * Convert containers to ReactFlow container nodes - pure transformation
   */
  private convertContainers(
    visState: VisualizationState, 
    nodes: ReactFlowNode[], 
    parentMap: Map<string, string>
  ): void {
    visState.visibleContainers.forEach(container => {
      const parentId = parentMap.get(container.id);
      
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
      
      // Use VisualizationState's official API for dimensions
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
          colorPalette: this.colorPalette,
          nodeCount,
          width,
          height,
          // Pass through any custom properties
          ...this.extractCustomProperties(container as any)
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
   * Convert regular nodes to ReactFlow standard nodes - pure transformation
   */
  private convertNodes(
    visState: VisualizationState, 
    nodes: ReactFlowNode[], 
    parentMap: Map<string, string>
  ): void {
    visState.visibleNodes.forEach(node => {
      const parentId = parentMap.get(node.id);
      
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
          colorPalette: this.colorPalette,
          // Pass through any custom properties
          ...this.extractCustomProperties(node)
        },
        parentId
      };
      
      nodes.push(standardNode);
    });
  }

  /**
   * Convert edges to ReactFlow edges - pure transformation
   */
  private convertEdges(visState: VisualizationState, edges: ReactFlowEdge[]): void {
    const handleConfig = getHandleConfig();
    
    visState.visibleEdges.forEach(edge => {
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
      
      // Handle assignment could be moved to VisualizationEngine for better separation
      if (!handleConfig.enableContinuousHandles) {
        reactFlowEdge.sourceHandle = (edge as any).sourceHandle || 'default-out';
        reactFlowEdge.targetHandle = (edge as any).targetHandle || 'default-in';
      }
      // For continuous handles, omit sourceHandle/targetHandle to let ReactFlow auto-connect
      
      edges.push(reactFlowEdge);
    });
  }

  /**
   * Extract custom properties from graph elements - pure transformation
   */
  private extractCustomProperties(element: any): Record<string, any> {
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