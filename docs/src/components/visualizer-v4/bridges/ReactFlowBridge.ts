/**
 * @fileoverview ReactFlow Bridge - Converts VisualizationState to ReactFlow format
 * 
 * This bridge converts VisualizationState to ReactFlow's expected data structures.
 * ReactFlow only sees unified edges (hyperedges are included transparently).
 * Uses configurable handle system for maximum layout flexibility.
 */

import type { VisualizationState } from '../core/VisualizationState';
import type { GraphNode, GraphEdge, Container } from '../shared/types';
import { LAYOUT_CONSTANTS } from '../shared/config';
import { MarkerType, Edge as ReactFlowEdge } from '@xyflow/react';
import { getHandleConfig, CURRENT_HANDLE_STRATEGY } from '../render/handleConfig';
import { convertEdgesToReactFlow, EdgeBridgeOptions } from './EdgeBridge';

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
  extent?: 'parent' | [[number, number], [number, number]]; // Constrains node movement to parent boundaries
}

export interface ReactFlowData {
  nodes: ReactFlowNode[];
  edges: ReactFlowEdge[];
}

export class ReactFlowBridge {
  private colorPalette: string = 'Set3';
  private edgeStyleConfig?: any; // EdgeStyleConfig from EdgeStyleProcessor

  /**
   * Set the color palette for node styling
   */
  setColorPalette(palette: string): void {
    this.colorPalette = palette;
  }

  /**
   * Set the edge style configuration
   */
  setEdgeStyleConfig(config: any): void {
    this.edgeStyleConfig = config;
  }

  /**
   * Convert positioned VisState data to ReactFlow format
   * TRUST ELK: Use ELK's hierarchical layout results completely
   */
  visStateToReactFlow(visState: VisualizationState): ReactFlowData {
    console.log(`[Bridge] 🔍 Converting VisualizationState with ${visState.visibleNodes.length} visible nodes`);
    
    // Debug: Log first few visible nodes
    const firstFewNodes = visState.visibleNodes.slice(0, 5);
    firstFewNodes.forEach(node => {
      console.log(`[Bridge] 📋 Node ${node.id}: label="${node.label}", shortLabel="${node.shortLabel}", fullLabel="${node.fullLabel}"`);
    });

    const nodes: ReactFlowNode[] = [];
    const edges: ReactFlowEdge[] = [];
    
    // Build parent-child mapping from VisualizationState
    const parentMap = this.buildParentMap(visState);
    
    // Convert containers using ELK positions
    this.convertContainersFromELK(visState, nodes, parentMap);
    
    // Convert regular nodes using ELK positions  
    this.convertNodesFromELK(visState, nodes, parentMap);
    
    // Convert edges using smart handle selection
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
          label: node.label || node.shortLabel || node.id, // Respect toggled label field
          shortLabel: node.shortLabel || node.id,
          fullLabel: node.fullLabel || node.shortLabel || node.id,
          style: node.style || 'default',
          colorPalette: this.colorPalette,
          ...this.extractCustomProperties(node)
        }
        // NO parentId - completely flat
      };
      
      // Debug: Log label data for troubleshooting
      if (node.id === '0' || node.id === '7') {
        console.log(`[Bridge] 🏷️ Node ${node.id}: label="${flatNode.data.label}", shortLabel="${flatNode.data.shortLabel}", fullLabel="${flatNode.data.fullLabel}"`);
      }
      
      // Debug: Log node label resolution
      if (node.label && node.label !== node.shortLabel) {
        console.log(`🔗 Bridge: Node ${node.id} using toggled label: "${node.label}" (short: "${node.shortLabel}", full: "${node.fullLabel}")`);
      }
      
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
          label: container.label || container.id, // Use single label field for containers
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
    
    // Create lookup sets for performance
    const visibleContainerIds = new Set(Array.from(visState.visibleContainers).map(c => c.id));
    const visibleNodeIds = new Set(Array.from(visState.visibleNodes).map(n => n.id));
    
    // Map all containers and nodes to their parent containers for visual hierarchy
    // This creates the nested structure in ReactFlow regardless of collapsed state
    visState.visibleContainers.forEach(container => {
      // Map child containers to this parent container using VisualizationState API
      const containerChildren = visState.getContainerChildren(container.id);
      containerChildren.forEach(childId => {
        // Only set parent relationship if the child is also visible
        if (visibleContainerIds.has(childId) || visibleNodeIds.has(childId)) {
          parentMap.set(childId, container.id);
        }
      });
    });
    
    // Also map nodes to their containers using the containerId property
    visState.visibleNodes.forEach(node => {
      if (node.containerId && visibleContainerIds.has(node.containerId)) {
        parentMap.set(node.id, node.containerId);
      }
    });
    
    return parentMap;
  }

  /**
   * Sort containers by hierarchy level to ensure parents are processed before children
   */
  private sortContainersByHierarchy(containers: any[], parentMap: Map<string, string>): any[] {
    const getHierarchyLevel = (containerId: string): number => {
      let level = 0;
      let currentId = containerId;
      while (parentMap.has(currentId)) {
        level++;
        currentId = parentMap.get(currentId)!;
      }
      return level;
    };
    
    return containers.sort((a, b) => getHierarchyLevel(a.id) - getHierarchyLevel(b.id));
  }

  /**
   * Convert containers to ReactFlow container nodes using ELK layout positions
   * TRUST ELK: Use ELK's hierarchical positioning completely
   */
  private convertContainersFromELK(
    visState: VisualizationState, 
    nodes: ReactFlowNode[], 
    parentMap: Map<string, string>
  ): void {
    // Sort containers by hierarchy level (parents first, then children)
    const containers = Array.from(visState.visibleContainers);
    const sortedContainers = this.sortContainersByHierarchy(containers, parentMap);
    
    sortedContainers.forEach(container => {
      const parentId = parentMap.get(container.id);
      
      // Get position and dimensions from ELK layout (stored in VisualizationState)
      const containerLayout = visState.getContainerLayout(container.id);
      let position: { x: number; y: number };
      
      if (parentId) {
        // CHILD CONTAINER: Convert absolute ELK coordinates to relative coordinates  
        const parentLayout = visState.getContainerLayout(parentId);
        
        // Check if we have meaningful ELK layout data (not just default 0,0 values)
        const hasRealELKLayout = containerLayout?.position?.x !== undefined && 
                                containerLayout?.position?.y !== undefined &&
                                (containerLayout.position.x !== 0 || containerLayout.position.y !== 0);
        
        if (hasRealELKLayout) {
          // Use ELK coordinates converted to relative
          const absoluteX = containerLayout?.position?.x || container.x || 0;
          const absoluteY = containerLayout?.position?.y || container.y || 0;
          const parentX = parentLayout?.position?.x || 0;
          const parentY = parentLayout?.position?.y || 0;
          
                    // Validate coordinates before calculation
          const validAbsoluteX = (typeof absoluteX === 'number' && !isNaN(absoluteX) && isFinite(absoluteX)) ? absoluteX : 0;
          const validAbsoluteY = (typeof absoluteY === 'number' && !isNaN(absoluteY) && isFinite(absoluteY)) ? absoluteY : 0;
          const validParentX = (typeof parentX === 'number' && !isNaN(parentX) && isFinite(parentX)) ? parentX : 0;
          const validParentY = (typeof parentY === 'number' && !isNaN(parentY) && isFinite(parentY)) ? parentY : 0;
          
          // Log if we found invalid coordinates
          if (absoluteX !== validAbsoluteX || absoluteY !== validAbsoluteY || parentX !== validParentX || parentY !== validParentY) {
            console.warn(`[ReactFlowBridge] Fixed invalid coordinates for container ${container.id}: abs(${absoluteX},${absoluteY}) -> (${validAbsoluteX},${validAbsoluteY}), parent(${parentX},${parentY}) -> (${validParentX},${validParentY})`);
          }
          
          position = {
            x: validAbsoluteX - validParentX,
            y: validAbsoluteY - validParentY
          };
        } else {
          // FALLBACK: Grid positioning when no meaningful ELK layout data
          const siblingContainers = Array.from(visState.visibleContainers)
            .filter(c => parentMap.get(c.id) === parentId);
          const containerIndex = siblingContainers.findIndex(c => c.id === container.id);
          
          const cols = LAYOUT_CONSTANTS.CONTAINER_GRID_COLUMNS || 2;
          const col = containerIndex % cols;
          const row = Math.floor(containerIndex / cols);
          const padding = LAYOUT_CONSTANTS.CONTAINER_GRID_PADDING || 20;
          const titleHeight = LAYOUT_CONSTANTS.CONTAINER_TITLE_HEIGHT || 30;
          
          position = {
            x: padding + col * (LAYOUT_CONSTANTS.CHILD_CONTAINER_WIDTH + padding),
            y: titleHeight + row * (LAYOUT_CONSTANTS.CHILD_CONTAINER_HEIGHT + padding)
          };
        }
      } else {
        // ROOT CONTAINER: Use absolute ELK coordinates or fallback
        const rootX = containerLayout?.position?.x || container.x || 0;
        const rootY = containerLayout?.position?.y || container.y || 0;
        
        // Validate root coordinates
        const validRootX = (typeof rootX === 'number' && !isNaN(rootX) && isFinite(rootX)) ? rootX : 0;
        const validRootY = (typeof rootY === 'number' && !isNaN(rootY) && isFinite(rootY)) ? rootY : 0;
        
        position = {
          x: validRootX,
          y: validRootY
        };
      }
      
      // Get adjusted dimensions that include label space (matches test expectations)
      const adjustedDimensions = visState.getContainerAdjustedDimensions(container.id);
      
      const width = (typeof adjustedDimensions.width === 'number' && !isNaN(adjustedDimensions.width) && isFinite(adjustedDimensions.width) && adjustedDimensions.width > 0) 
        ? adjustedDimensions.width : LAYOUT_CONSTANTS.DEFAULT_PARENT_CONTAINER_WIDTH;
      const height = (typeof adjustedDimensions.height === 'number' && !isNaN(adjustedDimensions.height) && isFinite(adjustedDimensions.height) && adjustedDimensions.height > 0) 
        ? adjustedDimensions.height : LAYOUT_CONSTANTS.DEFAULT_PARENT_CONTAINER_HEIGHT;
      
      const nodeCount = container.collapsed ? 
        visState.getContainerChildren(container.id)?.size || 0 : 0;
      
      // Debug: Log container label source
      console.log(`🏷️ Container ${container.id}: label="${container.label}", id="${container.id}", using="${container.label || container.id}"`);
      
      const containerNode: ReactFlowNode = {
        id: container.id,
        type: 'container',
        position,
        data: {
          label: container.label || container.id, // Use container.label, fallback to id
          style: (container as any).style || 'default',
          collapsed: container.collapsed,
          colorPalette: this.colorPalette,
          width,
          height,
          nodeCount: nodeCount
        },
        style: {
          width,
          height
        },
        parentId: parentId,
        extent: parentId ? 'parent' : undefined // Constrain to parent if nested
      };
      
      nodes.push(containerNode);
    });
  }

  /**
   * Convert regular nodes to ReactFlow standard nodes using ELK layout positions
   * TRUST ELK: Use ELK's hierarchical positioning completely
   */
  private convertNodesFromELK(
    visState: VisualizationState, 
    nodes: ReactFlowNode[], 
    parentMap: Map<string, string>
  ): void {
    visState.visibleNodes.forEach(node => {
      const parentId = parentMap.get(node.id);
      
      // Get position from ELK layout (stored in VisualizationState)
      const nodeLayout = visState.getNodeLayout(node.id);
      let position: { x: number; y: number };
      
      if (parentId) {
        // CHILD NODE: Convert absolute ELK coordinates to relative coordinates
        const parentLayout = visState.getContainerLayout(parentId);
        const absoluteX = nodeLayout?.position?.x || node.x || 0;
        const absoluteY = nodeLayout?.position?.y || node.y || 0;
        const parentX = parentLayout?.position?.x || 0;
        const parentY = parentLayout?.position?.y || 0;
        
        // Validate all coordinates before calculation
        const validAbsoluteX = (typeof absoluteX === 'number' && !isNaN(absoluteX) && isFinite(absoluteX)) ? absoluteX : 0;
        const validAbsoluteY = (typeof absoluteY === 'number' && !isNaN(absoluteY) && isFinite(absoluteY)) ? absoluteY : 0;
        const validParentX = (typeof parentX === 'number' && !isNaN(parentX) && isFinite(parentX)) ? parentX : 0;
        const validParentY = (typeof parentY === 'number' && !isNaN(parentY) && isFinite(parentY)) ? parentY : 0;
        
        position = {
          x: validAbsoluteX - validParentX,
          y: validAbsoluteY - validParentY
        };
      } else {
        // ROOT NODE: Use absolute ELK coordinates
        const rootX = nodeLayout?.position?.x || node.x || 0;
        const rootY = nodeLayout?.position?.y || node.y || 0;
        
        // Validate root coordinates
        const validRootX = (typeof rootX === 'number' && !isNaN(rootX) && isFinite(rootX)) ? rootX : 0;
        const validRootY = (typeof rootY === 'number' && !isNaN(rootY) && isFinite(rootY)) ? rootY : 0;
        
        position = {
          x: validRootX,
          y: validRootY
        };
      }
      
      const standardNode: ReactFlowNode = {
        id: node.id,
        type: 'standard',
        position,
        data: {
          label: node.label || node.shortLabel || node.id, // Respect toggled label field
          shortLabel: node.shortLabel || node.id,
          fullLabel: node.fullLabel || node.shortLabel || node.id,
          style: node.style || 'default',
          colorPalette: this.colorPalette,
          ...this.extractCustomProperties(node)
        },
        parentId,
        connectable: CURRENT_HANDLE_STRATEGY === 'floating',
        // ReactFlow sub-flow: constrain children within parent bounds
        extent: parentId ? 'parent' : undefined
      };

      // Debug: Log label handling for ELK nodes
      if (node.label && node.label !== node.shortLabel) {
        console.log(`🔗 Bridge (ELK): Node ${node.id} using toggled label: "${node.label}" (short: "${node.shortLabel}", full: "${node.fullLabel}")`);
      }
      
      nodes.push(standardNode);
    });
  }

  /**
   * Convert regular edges to ReactFlow edges
   */
  private convertEdges(visState: VisualizationState, edges: ReactFlowEdge[]): void {
    // First, convert edges using EdgeBridge for proper styling
    const visibleEdges = Array.from(visState.visibleEdges);
    const edgeBridgeOptions: EdgeBridgeOptions = {
      edgeStyleConfig: this.edgeStyleConfig,
      showPropertyLabels: true,
      enableAnimations: true
    };
    const convertedEdges = convertEdgesToReactFlow(visibleEdges, edgeBridgeOptions);
    
    // Then, apply smart handle selection to each converted edge
    convertedEdges.forEach((reactFlowEdge, index) => {
      const originalEdge = visibleEdges[index];
      
      if (CURRENT_HANDLE_STRATEGY === 'discrete') {
        const smartHandles = this.getEdgeHandles(visState, originalEdge.id);
        
        // Override EdgeBridge handles with smart selection
        reactFlowEdge.sourceHandle = smartHandles.sourceHandle || 'out-bottom';
        reactFlowEdge.targetHandle = smartHandles.targetHandle || 'in-top';
      }
    });
    
    // Add converted edges to the result array
    edges.push(...convertedEdges);
  }

  /**
   * Get edge handles for ReactFlow bridge (moved from VisualizationState)
   * This is ReactFlow-specific logic for handle assignment
   * For discrete strategy: intelligently choose handles based on node positions
   * Prefer: inputs at top or left, outputs at bottom or right
   */
  getEdgeHandles(visState: VisualizationState, edgeId: string): { sourceHandle?: string; targetHandle?: string } {
    const edge = visState.getGraphEdge(edgeId);
    if (!edge) {
      return {};
    }
    
    if (CURRENT_HANDLE_STRATEGY === 'discrete') {
      // Get source and target nodes to determine optimal handle positions
      const sourceNode = visState.getGraphNode(edge.source);
      const targetNode = visState.getGraphNode(edge.target);
      
      if (sourceNode && targetNode && sourceNode.layout && targetNode.layout) {
        // Fix: Access position coordinates correctly from nested structure
        const sourcePos = { 
          x: sourceNode.layout.position?.x || sourceNode.layout.x || 0, 
          y: sourceNode.layout.position?.y || sourceNode.layout.y || 0,
          width: sourceNode.layout.dimensions?.width || sourceNode.layout.width || 120,
          height: sourceNode.layout.dimensions?.height || sourceNode.layout.height || 40
        };
        const targetPos = { 
          x: targetNode.layout.position?.x || targetNode.layout.x || 0, 
          y: targetNode.layout.position?.y || targetNode.layout.y || 0,
          width: targetNode.layout.dimensions?.width || targetNode.layout.width || 120,
          height: targetNode.layout.dimensions?.height || targetNode.layout.height || 40
        };
        
        // Calculate center-to-center distances
        const sourceCenterX = sourcePos.x + sourcePos.width / 2;
        const sourceCenterY = sourcePos.y + sourcePos.height / 2;
        const targetCenterX = targetPos.x + targetPos.width / 2;
        const targetCenterY = targetPos.y + targetPos.height / 2;
        
        const deltaX = targetCenterX - sourceCenterX;
        const deltaY = targetCenterY - sourceCenterY;
        
        // Use absolute distances to determine primary direction
        const absX = Math.abs(deltaX);
        const absY = Math.abs(deltaY);
        
        let sourceHandle: string;
        let targetHandle: string;
        
        // STRICT RULES: 
        // - Sources only use: out-bottom, out-right (NEVER out-top or out-left)
        // - Targets only use: in-top, in-left (NEVER in-bottom or in-right)
        
        if (absX > absY * 0.8) {
          // Horizontal relationship is stronger - prefer left/right handles
          sourceHandle = 'out-right';   // Always use right for horizontal sources
          targetHandle = 'in-left';     // Always use left for horizontal targets
        } else {
          // Vertical or diagonal relationship - use bottom/top handles
          sourceHandle = 'out-bottom';  // Always use bottom for sources (never top!)
          targetHandle = 'in-top';      // Always use top for targets
        }
        
        return { sourceHandle, targetHandle };
      }
      
      // Fallback to default if no layout information
      return {
        sourceHandle: (edge as any).sourceHandle || 'out-bottom',
        targetHandle: (edge as any).targetHandle || 'in-top'
      };
    }
    
    // Handle edges with port information for other strategies
    return {
      sourceHandle: (edge as any).sourceHandle || 'default-out',
      targetHandle: (edge as any).targetHandle || 'default-in'
    };
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
