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
    console.log('[ReactFlowBridge] 🔄 Using HIERARCHICAL ELK + ReactFlow pattern');
    
    const nodes: ReactFlowNode[] = [];
    const edges: ReactFlowEdge[] = [];
    
    // Build parent-child mapping for hierarchy
    const parentMap = this.buildParentMap(visState);
    console.log(`[ReactFlowBridge] 🗺️ Parent map has ${parentMap.size} relationships:`, Array.from(parentMap.entries()));
    
    // Log container and node counts before conversion
    console.log(`[ReactFlowBridge] 📊 Before conversion: ${visState.visibleContainers.length} containers, ${visState.visibleNodes.length} nodes`);
    
    // Convert containers first (so they exist when children reference them)
    this.convertContainers(visState, nodes, parentMap);
    console.log(`[ReactFlowBridge] 📦 After container conversion: ${nodes.filter(n => n.type === 'container').length} container nodes`);
    
    // Convert regular nodes with parent relationships
    this.convertNodes(visState, nodes, parentMap);
    console.log(`[ReactFlowBridge] 🔷 After node conversion: ${nodes.filter(n => n.type === 'standard').length} standard nodes`);
    
    // Convert edges using simple source/target mapping with discrete handles
    this.convertEdges(visState, edges);
    
    console.log(`[ReactFlowBridge] ✅ Hierarchical pattern: ${nodes.length} nodes, ${edges.length} edges`);
    
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
      
      console.log(`[ReactFlowBridge] 🔷 FLAT Node ${node.id}: position=(${position.x}, ${position.y})`);
      
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
      
      console.log(`[ReactFlowBridge] 📦 FLAT Container ${container.id}: position=(${position.x}, ${position.y}), size=${width}x${height}`);
      
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
    console.log(`[ReactFlowBridge] 🔗 Converting ${visState.visibleEdges.length} edges using CANONICAL pattern`);
    
    visState.visibleEdges.forEach(edge => {
      console.log(`[ReactFlowBridge] 🔗 FLAT Edge ${edge.id}: ${edge.source} -> ${edge.target}`);
      
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
      
      console.log(`[ReactFlowBridge] 📦 Container ${container.id}: collapsed=${container.collapsed}, position=(${position.x}, ${position.y}), size=${width}x${height}, nodeCount=${nodeCount}`);
      
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
          console.log(`[ReactFlowBridge] 🔷 Node ${node.id}: absolute=(${nodeLayout?.position?.x}, ${nodeLayout?.position?.y}) -> relative=(${position.x}, ${position.y}) parent=${parentId}`);
        } else {
          console.log(`[ReactFlowBridge] 🔷 Node ${node.id}: position=(${position.x}, ${position.y}), parent=${parentId} (no parent layout found)`);
        }
      } else {
        console.log(`[ReactFlowBridge] 🔷 Node ${node.id}: position=(${position.x}, ${position.y}), parent=none`);
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
      
      console.log(`[ReactFlowBridge] 🔍 BROWSER DEBUG - Edge ${edge.id}: CURRENT_HANDLE_STRATEGY=${CURRENT_HANDLE_STRATEGY}`);
      
      // Determine edge type based on handle strategy
      const edgeType: 'standard' | 'floating' = CURRENT_HANDLE_STRATEGY === 'floating' ? 'floating' : 'standard';
      
      // For floating edges, create edge without handle properties
      // For other edges, create edge with handle properties that can be set
      let reactFlowEdge: ReactFlowEdge;
      
      if (CURRENT_HANDLE_STRATEGY === 'floating') {
        // Floating edges: no handle properties at all
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
        console.log(`[ReactFlowBridge] 🔗 Edge ${edge.id} has ${edge.layout.sections.length} routing sections`);
      } else if (edge.sections && edge.sections.length > 0) {
        console.log(`[ReactFlowBridge] 🔗 Edge ${edge.id} has ${edge.sections.length} routing sections (direct sections property)`);
        // Use the sections directly if layout.sections is not available
        const sections = edge.sections.map((section, i) => {
          const startPoint = section.startPoint;
          const endPoint = section.endPoint;
          const bendPoints = section.bendPoints || [];
          
          console.log(`[ReactFlowBridge] 📍 Section ${i} (already corrected): start=(${startPoint?.x},${startPoint?.y}), end=(${endPoint?.x},${endPoint?.y})`);
          
          return {
            ...section,
            startPoint,
            endPoint,
            bendPoints
          };
        });
        
        // Store routing sections in ReactFlow edge data for custom edge renderer
        (reactFlowEdge.data as any).routing = sections;
      } else {
        console.log(`[ReactFlowBridge] 🔗 Edge ${edge.id} has no routing sections - will use automatic ReactFlow routing`);
      }
      
      // Handle strategy should be determined by VisualizationState, not ReactFlowBridge
      // TODO: Move handle logic to VisualizationState.getEdgeHandles(edgeId)
      if (CURRENT_HANDLE_STRATEGY === 'floating') {
        // For floating edges, the edge object already has no handle properties
        // Nothing to do - the custom FloatingEdge component will calculate attachment points
        console.log(`[ReactFlowBridge] ✅ BROWSER - Floating edge ${edge.id} - no handle properties`);
      } else if (CURRENT_HANDLE_STRATEGY === 'discrete' || !handleConfig.enableContinuousHandles) {
        // Use discrete handles with forced top-down flow for consistent edge positioning
        reactFlowEdge.sourceHandle = edge.sourceHandle || 'out-bottom';
        reactFlowEdge.targetHandle = edge.targetHandle || 'in-top';
        console.log(`[ReactFlowBridge] ✅ BROWSER - Discrete edge ${edge.id} - added handles: ${reactFlowEdge.sourceHandle}→${reactFlowEdge.targetHandle}`);
      } else {
        // For continuous handles, sourceHandle/targetHandle remain undefined
        console.log(`[ReactFlowBridge] ✅ BROWSER - Continuous edge ${edge.id} - no handle properties needed`);
      }
      
      console.log(`[ReactFlowBridge] 🔍 BROWSER - Final edge ${edge.id}:`, {
        type: reactFlowEdge.type,
        hasSourceHandle: 'sourceHandle' in reactFlowEdge,
        hasTargetHandle: 'targetHandle' in reactFlowEdge,
        sourceHandle: reactFlowEdge.sourceHandle,
        targetHandle: reactFlowEdge.targetHandle
      });
      
      // console.log(`[ReactFlowBridge] ✅ Created ReactFlow edge ${edge.id}:`, reactFlowEdge);
      
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
