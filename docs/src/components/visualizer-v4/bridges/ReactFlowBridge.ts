/**
 * @fileoverview ReactFlow Bridge - Converts VisualizationState to ReactFlow format
 * 
 * This bridge converts VisualizationState to ReactFlow's expected data structures.
 * ReactFlow only sees unified edges (hyperedges are included transparently).
 * Uses configurable handle system for maximum layout flexibility.
 */

import type { VisualizationState } from '../core/VisState';
import type { GraphNode, GraphEdge, Container } from '../shared/types';
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
  // extent?: 'parent'; // REMOVED: Causes drag coordinate issues in ReactFlow
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
   * Pure data transformation - no layout logic
   */
  visStateToReactFlow(visState: VisualizationState): ReactFlowData {
    // // console.log((('[ReactFlowBridge] 🔄 Converting VisState to ReactFlow format')));
    
    const nodes: ReactFlowNode[] = [];
    const edges: ReactFlowEdge[] = [];
    
    // Create parent-child mapping for hierarchical layout
    const parentMap = this.buildParentMap(visState);
    
    // Convert containers to ReactFlow nodes
    this.convertContainers(visState, nodes, parentMap);
    
    // Convert regular nodes to ReactFlow nodes  
    this.convertNodes(visState, nodes, parentMap);
    
    // Convert all edges to ReactFlow edges (now includes hyperedges transparently)
    console.log(`[ReactFlowBridge] 🔗 Converting ${visState.visibleEdges.length} edges to ReactFlow format...`);
    this.convertEdges(visState, edges);
    console.log(`[ReactFlowBridge] ✅ Converted ${edges.length} ReactFlow edges`);
    
    // Debug: Sample a few edges to see their state
    if (edges.length > 0) {
      console.log('[ReactFlowBridge] 🔍 Sample edge conversion results:');
      edges.slice(0, 3).forEach(edge => {
        const visEdge = visState.getGraphEdge(edge.id);
        console.log(`[ReactFlowBridge] 🔗 Edge ${edge.id}: source=${edge.source}, target=${edge.target}, hasLayout=${!!visEdge?.layout}, sections=${visEdge?.layout?.sections?.length || 0}`);
      });
    }
    
    return { nodes, edges };
  }

  /**
   * Build parent-child relationship map
   * NOTE: VisualizationState should provide this logic via getParentChildMap()
   */
  private buildParentMap(visState: VisualizationState): Map<string, string> {
    const parentMap = new Map<string, string>();
    
    // TODO: Move this business logic to VisualizationState
    // Map nodes to their parent containers
    visState.visibleContainers.forEach(container => {
      if (!container.collapsed) {
        // BUSINESS LOGIC VIOLATION: VisualizationState should determine which containers can have children
        container.children.forEach(childId => {
          parentMap.set(childId, container.id);
        });
      }
    });
    
    return parentMap;
  }

  /**
   * Convert containers to ReactFlow container nodes
   */
  private convertContainers(
    visState: VisualizationState, 
    nodes: ReactFlowNode[], 
    parentMap: Map<string, string>
  ): void {
    visState.visibleContainers.forEach(container => {
      const parentId = parentMap.get(container.id);
      
      // Get coordinates from VisState computed view (already canonical coordinates)
      const elkCoords = {
        x: container.x || 0,
        y: container.y || 0
      };
      
      // Convert ELK coordinates to ReactFlow coordinates
      const parentContainer = parentId ? 
        CoordinateTranslator.getContainerInfo(parentId, visState) : 
        undefined;
      
      const position = CoordinateTranslator.elkToReactFlow(elkCoords, parentContainer);
      
      // Use computed dimensions from VisState (includes ELK-calculated sizes via expandedDimensions)
      // IMPORTANT: Use VisualizationState API to get proper dimensions (handles collapsed containers)
      const effectiveDimensions = visState.getContainerAdjustedDimensions(container.id);
      const width = effectiveDimensions.width;
      const height = effectiveDimensions.height;
      
      // Calculate node count for collapsed containers
      const nodeCount = container.collapsed ? 
        visState.getContainerChildren(container.id)?.size || 0 : 0;
      
      console.log(`[ReactFlowBridge] 📦 Container ${container.id}: collapsed=${container.collapsed}, ELK=(${elkCoords.x}, ${elkCoords.y}), ReactFlow=(${position.x}, ${position.y}), size=${width}x${height}, nodeCount=${nodeCount}`);
      
      const containerNode: ReactFlowNode = {
        id: container.id,
        type: 'container',
        position,
        data: {
          label: (container as any).data?.label || (container as any).label || container.id, // Use proper label like InfoPanel
          style: (container as any).style || 'default',
          collapsed: container.collapsed || false,
          colorPalette: this.colorPalette, // Pass color palette for styling
          nodeCount, // Number of child nodes for collapsed containers
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
        // FIX: Remove extent: 'parent' - causes ReactFlow drag coordinate issues
        // extent: parentId ? 'parent' : undefined
      };
      
      nodes.push(containerNode);
    });
  }

  /**
   * Convert regular nodes to ReactFlow standard nodes
   */
  private convertNodes(
    visState: VisualizationState, 
    nodes: ReactFlowNode[], 
    parentMap: Map<string, string>
  ): void {
    visState.visibleNodes.forEach(node => {
      const parentId = parentMap.get(node.id);
      
      // Get ELK coordinates from node layout (where ELKBridge stores them)
      const nodeLayout = visState.getNodeLayout(node.id);
      // // console.log(((`[ReactFlowBridge] 🔍 Node ${node.id} layout:`, nodeLayout, 'raw node coords:', { x: node.x, y: node.y })));
      const elkCoords = {
        x: nodeLayout?.position?.x || node.x || 0,
        y: nodeLayout?.position?.y || node.y || 0
      };
      
      // Convert ELK coordinates to ReactFlow coordinates
      const parentContainer = parentId ? 
        CoordinateTranslator.getContainerInfo(parentId, visState) : 
        undefined;
      
      // if (parentContainer) {
      //   console.log(`[ReactFlowBridge] 🔍 Parent container ${parentId} info:`, {
      //     id: parentContainer.id,
      //     x: parentContainer.x,
      //     y: parentContainer.y,
      //     width: parentContainer.width,
      //     height: parentContainer.height
      //   });
      // }
      
      const position = CoordinateTranslator.elkToReactFlow(elkCoords, parentContainer);
      
      // // console.log(((`[ReactFlowBridge] 🔘 Node ${node.id}: parent=${parentId || 'none'}, ELK=(${elkCoords.x}, ${elkCoords.y}), ReactFlow=(${position.x}, ${position.y})`)));
      
      const standardNode: ReactFlowNode = {
        id: node.id,
        type: 'standard',
        position,
        data: {
          label: node.label || node.id,
          style: node.style || 'default',
          colorPalette: this.colorPalette, // Pass the current color palette
          // Pass through any custom properties
          ...this.extractCustomProperties(node)
        },
        parentId
        // FIX: Remove extent: 'parent' - causes ReactFlow drag coordinate issues
        // extent: parentId ? 'parent' : undefined
      };
      
      nodes.push(standardNode);
    });
  }

  /**
   * Convert regular edges to ReactFlow edges
   */
  private convertEdges(visState: VisualizationState, edges: ReactFlowEdge[]): void {
    visState.visibleEdges.forEach(edge => {
    //   // Debug: log the actual edge data to see what we're getting
    //   console.log(`[ReactFlowBridge] 🔍 Debug edge ${edge.id}:`, {
    //     source: edge.source,
    //     target: edge.target,
    //     sourceHandle: edge.sourceHandle,
    //     targetHandle: edge.targetHandle,
    //     sourceHandleType: typeof edge.sourceHandle,
    //     targetHandleType: typeof edge.targetHandle
    //   });
      
      const handleConfig = getHandleConfig();
      
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
        } as any
      };
      
      // Check if this edge has layout/routing information from ELK
      if (edge.layout && edge.layout.sections && edge.layout.sections.length > 0) {
        console.log(`[ReactFlowBridge] 🔗 Edge ${edge.id} has ${edge.layout.sections.length} routing sections`);
        
        // Transform edge coordinates from container-relative to absolute coordinates
        const transformedSections = edge.layout.sections.map((section, i) => {
          const startPoint = section.startPoint;
          const endPoint = section.endPoint;
          const bendPoints = section.bendPoints || [];
          
          // Get source and target container positions to transform coordinates
          let sourceOffset = { x: 0, y: 0 };
          let targetOffset = { x: 0, y: 0 };
          
          // For edges between containers, we need to offset by container positions
          try {
            const sourceContainer = visState.getContainer(edge.source);
            if (sourceContainer && sourceContainer.x !== undefined && sourceContainer.y !== undefined) {
              sourceOffset = { x: sourceContainer.x, y: sourceContainer.y };
            }
          } catch (e) {
            // Source is not a container, check if it's a node inside a container
            // For now, use zero offset
          }
          
          try {
            const targetContainer = visState.getContainer(edge.target);
            if (targetContainer && targetContainer.x !== undefined && targetContainer.y !== undefined) {
              targetOffset = { x: targetContainer.x, y: targetContainer.y };
            }
          } catch (e) {
            // Target is not a container
          }
          
          // Transform the section coordinates
          const transformedSection = {
            ...section,
            startPoint: startPoint ? {
              x: startPoint.x + sourceOffset.x,
              y: startPoint.y + sourceOffset.y
            } : undefined,
            endPoint: endPoint ? {
              x: endPoint.x + targetOffset.x,
              y: endPoint.y + targetOffset.y
            } : undefined,
            bendPoints: bendPoints.map(point => ({
              x: point.x + sourceOffset.x, // Use source offset for bend points
              y: point.y + sourceOffset.y
            }))
          };
          
          console.log(`[ReactFlowBridge] 📍 Section ${i} transformed: start=(${transformedSection.startPoint?.x},${transformedSection.startPoint?.y}), end=(${transformedSection.endPoint?.x},${transformedSection.endPoint?.y})`);
          return transformedSection;
        });
        
        // Store transformed routing in ReactFlow edge data for custom edge renderer
        (reactFlowEdge.data as any).routing = transformedSections;
      } else {
        console.log(`[ReactFlowBridge] 🔗 Edge ${edge.id} has no routing sections - will use automatic ReactFlow routing`);
      }
      
      // Handle strategy should be determined by VisualizationState, not ReactFlowBridge
      // TODO: Move handle logic to VisualizationState.getEdgeHandles(edgeId)
      if (!handleConfig.enableContinuousHandles) {
        // BUSINESS LOGIC VIOLATION: VisualizationState should determine handle assignments
        reactFlowEdge.sourceHandle = edge.sourceHandle || 'default-out';
        reactFlowEdge.targetHandle = edge.targetHandle || 'default-in';
      }
      // For continuous handles, omit sourceHandle/targetHandle to let ReactFlow auto-connect
      
      // // console.log(((`[ReactFlowBridge] ✅ Created ReactFlow edge ${edge.id}:`, reactFlowEdge)));
      
      edges.push(reactFlowEdge);
    });
  }

  /**
   * Extract custom properties from graph elements
   */
  private extractCustomProperties(element: GraphNode | GraphEdge | Container): Record<string, any> {
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
