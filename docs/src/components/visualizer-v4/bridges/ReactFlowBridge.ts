/**
 * @fileoverview ReactFlow Bridge - Converts VisualizationState to ReactFlow format
 * 
 * This bridge converts VisualizationState to ReactFlow's expected data structures.
 * ReactFlow only sees unified edges (hyperedges are included transparently).
 * Uses configurable handle system for maximum layout flexibility.
 */

import type { VisualizationState } from '../core/VisState';
import type { GraphNode, GraphEdge, Container } from '../shared/types';
import { MarkerType } from '@xyflow/react';
import { getHandleConfig, CURRENT_HANDLE_STRATEGY } from '../render/handleConfig';

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
  connectable?: boolean; // For floating handles strategy
  // extent?: 'parent'; // REMOVED: Causes drag coordinate issues in ReactFlow
}

export interface ReactFlowEdge {
  id: string;
  type: 'standard' | 'hyper' | 'floating';
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
   * HIERARCHICAL: Standard ELK + ReactFlow pattern with proper parent-child relationships
   */
  visStateToReactFlow(visState: VisualizationState): ReactFlowData {
    const nodes: ReactFlowNode[] = [];
    const edges: ReactFlowEdge[] = [];
    
    // Build parent-child mapping for hierarchy
    const parentMap = this.buildParentMap(visState);
    
    // Convert containers first (so they exist when children reference them)
    this.convertContainers(visState, nodes, parentMap);
    
    // Convert regular nodes with parent relationships
    this.convertNodes(visState, nodes, parentMap);
    
    // Convert edges using simple source/target mapping with discrete handles
    this.convertEdges(visState, edges);
    
    return { nodes, edges };
  }

  /**
   * CANONICAL PATTERN: Convert nodes to flat ReactFlow nodes (no hierarchy)
   */
  private convertNodesToFlat(visState: VisualizationState, nodes: ReactFlowNode[]): void {
    visState.visibleNodes.forEach(node => {
      // CANONICAL: Use ELK coordinates if available, otherwise fall back to node coordinates
      let position;
      try {
        const nodeLayout = visState.getNodeLayout(node.id);
        position = {
          x: nodeLayout?.position?.x || node.x || 0,
          y: nodeLayout?.position?.y || node.y || 0
        };
      } catch {
        // Fallback for test environments or when layout isn't available
        position = {
          x: node.x || 0,
          y: node.y || 0
        };
      }
      
      const flatNode: ReactFlowNode = {
        id: node.id,
        type: 'standard',
        position,
        data: {
          label: node.label || node.id,
          style: node.style || 'default',
          colorPalette: this.colorPalette,
          ...this.extractCustomProperties(node)
        }
        // NO parentId - completely flat
      };
      
      nodes.push(flatNode);
    });
  }

  /**
   * CANONICAL PATTERN: Convert containers to flat ReactFlow nodes (no hierarchy)
   */
  private convertContainersToFlat(visState: VisualizationState, nodes: ReactFlowNode[]): void {
    visState.visibleContainers.forEach(container => {
      // CANONICAL: Use ELK coordinates if available, otherwise fall back to container coordinates
      let position;
      try {
        const containerLayout = visState.getContainerLayout(container.id);
        position = {
          x: containerLayout?.position?.x || container.x || 0,
          y: containerLayout?.position?.y || container.y || 0
        };
      } catch {
        // Fallback for test environments or when layout isn't available
        position = {
          x: container.x || 0,
          y: container.y || 0
        };
      }
      
      const width = container.width;
      const height = container.height;
      const nodeCount = container.collapsed ? 
        visState.getContainerChildren(container.id)?.size || 0 : 0;
      
      const flatContainer: ReactFlowNode = {
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
          ...this.extractCustomProperties(container as any)
        },
        style: {
          width,
          height
        }
        // NO parentId - completely flat
      };
      
      nodes.push(flatContainer);
    });
  }

  /**
   * CANONICAL PATTERN: Convert edges using simple source/target mapping
   */
  private convertEdgesToFlat(visState: VisualizationState, edges: ReactFlowEdge[]): void {
    visState.visibleEdges.forEach(edge => {
      const flatEdge: ReactFlowEdge = {
        id: edge.id,
        type: 'standard',
        source: edge.source,
        target: edge.target,
        sourceHandle: 'out-bottom', // Force edges to come out the bottom of source nodes
        targetHandle: 'in-top',     // Force edges to go into the top of target nodes
        markerEnd: {
          type: MarkerType.ArrowClosed,
          width: 15,
          height: 15,
          color: '#999'
        },
        data: {
          style: edge.style || 'default'
        }
        // NO custom routing - ReactFlow handles positioning automatically
      };
      
      edges.push(flatEdge);
    });
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
      
      // STANDARD PRACTICE: ELK configured with ROOT coordinates, use directly
      const containerLayout = visState.getContainerLayout(container.id);
      const position = {
        x: containerLayout?.position?.x || container.x || 0,
        y: containerLayout?.position?.y || container.y || 0
      };
      
      // visibleContainers already includes adjusted dimensions
      const width = container.width;
      const height = container.height;
      
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
          ...this.extractCustomProperties(container as any)
        },
        style: {
          width,
          height
        },
        parentId,
        connectable: CURRENT_HANDLE_STRATEGY === 'floating'
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
      
      // Get node layout from ELK (absolute coordinates)
      const nodeLayout = visState.getNodeLayout(node.id);
      let position = {
        x: nodeLayout?.position?.x || node.x || 0,
        y: nodeLayout?.position?.y || node.y || 0
      };
      
      // HIERARCHICAL COORDINATE TRANSFORMATION: Convert absolute to relative
      if (parentId) {
        const parentLayout = visState.getContainerLayout(parentId);
        if (parentLayout?.position) {
          // Make coordinates relative to parent container
          position = {
            x: position.x - parentLayout.position.x,
            y: position.y - parentLayout.position.y
          };
        }
      }
      
      const standardNode: ReactFlowNode = {
        id: node.id,
        type: 'standard',
        position,
        data: {
          label: node.label || node.id,
          style: node.style || 'default',
          colorPalette: this.colorPalette,
          ...this.extractCustomProperties(node)
        },
        parentId,
        connectable: CURRENT_HANDLE_STRATEGY === 'floating'
      };
      
      nodes.push(standardNode);
    });
  }

  /**
   * Convert regular edges to ReactFlow edges
   */
  private convertEdges(visState: VisualizationState, edges: ReactFlowEdge[]): void {
    visState.visibleEdges.forEach((edge, index) => {
      const handleConfig = getHandleConfig();
      
      // Determine edge type based on handle strategy
      const edgeType: 'standard' | 'floating' = CURRENT_HANDLE_STRATEGY === 'floating' ? 'floating' : 'standard';
      
      // For floating edges, create edge without handle properties
      // For other edges, create edge with handle properties that can be set
      let reactFlowEdge: ReactFlowEdge;
      
      if (CURRENT_HANDLE_STRATEGY === 'floating') {
        // Floating edges: completely omit handle properties (don't set them to undefined)
        reactFlowEdge = {
          id: edge.id,
          type: edgeType,
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
        
        // Extra safety: ensure no handle properties exist
        delete (reactFlowEdge as any).sourceHandle;
        delete (reactFlowEdge as any).targetHandle;
        
      } else {
        // Standard edges: include handle properties
        reactFlowEdge = {
          id: edge.id,
          type: edgeType,
          source: edge.source,
          target: edge.target,
          sourceHandle: undefined,
          targetHandle: undefined,
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
      }
      
      // Check if this edge has layout/routing information from ELK
      if (edge.layout && edge.layout.sections && edge.layout.sections.length > 0) {
        // Handle existing routing sections
      } else if (edge.sections && edge.sections.length > 0) {
        // Use the sections directly if layout.sections is not available
        const sections = edge.sections.map((section, i) => {
          const startPoint = section.startPoint;
          const endPoint = section.endPoint;
          const bendPoints = section.bendPoints || [];
          
          return {
            ...section,
            startPoint,
            endPoint,
            bendPoints
          };
        });
        
        // Store routing sections in ReactFlow edge data for custom edge renderer
        (reactFlowEdge.data as any).routing = sections;
      }
      
      // Handle strategy should be determined by VisualizationState, not ReactFlowBridge
      // TODO: Move handle logic to VisualizationState.getEdgeHandles(edgeId)
      if (edgeType === 'floating') {
        // For floating edges, use actual handle IDs but let FloatingEdge component calculate positions
        // React Flow v12 requires valid handle IDs, even for floating edges
        reactFlowEdge.sourceHandle = 'out-bottom'; // Default handles that exist on nodes
        reactFlowEdge.targetHandle = 'in-top';
      } else if (CURRENT_HANDLE_STRATEGY === 'discrete' || !handleConfig.enableContinuousHandles) {
        // Use discrete handles with forced top-down flow for consistent edge positioning
        reactFlowEdge.sourceHandle = edge.sourceHandle || 'out-bottom';
        reactFlowEdge.targetHandle = edge.targetHandle || 'in-top';
      }
      
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
