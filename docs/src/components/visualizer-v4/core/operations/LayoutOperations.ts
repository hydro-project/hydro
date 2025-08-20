/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * Layout Operations - Handles layout-related operations
 * 
 * Manages layout positioning, dimension updates, and manual position tracking.
 * Extracted from VisState.ts for better separation of concerns.
 */

import { LAYOUT_CONSTANTS } from '../../shared/config';

export class LayoutOperations {
  private readonly state: any;

  constructor(state: any) {
    this.state = state;
  }

  /**
   * Set manual position for a node or container
   */
  setManualPosition(entityId: string, x: number, y: number): void {
    this.state._collections.manualPositions.set(entityId, { x, y });
  }

  /**
   * Get all manual positions for nodes and containers
   */
  getAllManualPositions(): Map<string, { x: number; y: number }> {
    return new Map(this.state._collections.manualPositions);
  }

  /**
   * Check if there are any manual positions
   */
  hasAnyManualPositions(): boolean {
    return this.state._collections.manualPositions.size > 0;
  }

  /**
   * Set container layout (applies padding and caches as expandedDimensions)
   */
  setContainerLayout(containerId: string, layout: any): void {
    const container = this.state._collections.containers.get(containerId);
    if (container) {
      container.layout = layout;
      if (layout.position) {
        container.x = layout.position.x;
        container.y = layout.position.y;
      }
      if (layout.dimensions) {
        container.width = layout.dimensions.width;
        container.height = layout.dimensions.height;
      }
    }
  }

  /**
   * Set node layout
   */
  setNodeLayout(nodeId: string, layout: any): void {
    const node = this.state._collections.graphNodes.get(nodeId);
    if (node) {
      node.layout = layout;
      if (layout.position) {
        node.x = layout.position.x;
        node.y = layout.position.y;
      }
      if (layout.dimensions) {
        node.width = layout.dimensions.width;
        node.height = layout.dimensions.height;
      }
    }
  }

  /**
   * Get container layout information
   */
  getContainerLayout(containerId: string): { position?: { x: number; y: number }; dimensions?: { width: number; height: number } } | undefined {
    const container = this.state._collections.containers.get(containerId);
    if (!container) return undefined;
    
    return {
      position: (container.x !== undefined && container.y !== undefined) ? { x: container.x, y: container.y } : undefined,
      dimensions: (container.width !== undefined && container.height !== undefined) ? { width: container.width, height: container.height } : undefined
    };
  }

  /**
   * Get node layout information
   */
  getNodeLayout(nodeId: string): { position?: { x: number; y: number }; dimensions?: { width: number; height: number } } | undefined {
    const node = this.state._collections.graphNodes.get(nodeId);
    if (!node) return undefined;
    
    return {
      position: (node.x !== undefined && node.y !== undefined) ? { x: node.x, y: node.y } : undefined,
      dimensions: (node.width !== undefined && node.height !== undefined) ? { width: node.width, height: node.height } : undefined
    };
  }

  /**
   * Get container adjusted dimensions
   * 
   * FIXED DOUBLE PADDING: Don't add manual label space since ELK layout and container 
   * components should handle label positioning internally
   */
  getContainerAdjustedDimensions(containerId: string): { width: number; height: number } {
    const container = this.state._collections.containers.get(containerId);
    if (!container) {
      throw new Error(`Container ${containerId} not found`);
    }
    
    // CRITICAL: Check if collapsed FIRST - collapsed containers should always use small dimensions
    if (container.collapsed) {
      return { 
        width: LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH,
        height: LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT
      };
    }
    
    // Get base dimensions from various possible sources (only for expanded containers)
    let baseWidth, baseHeight;
    
    if (container.expandedDimensions) {
      baseWidth = container.expandedDimensions.width;
      baseHeight = container.expandedDimensions.height;
      
      return { 
        width: baseWidth,
        height: baseHeight
      };
    } else {
      baseWidth = container.width;
      baseHeight = container.height;
    }
    
    // For expanded containers without cached dimensions, use raw dimensions
    // Let ELK layout and container components handle internal label positioning
    const width = baseWidth || LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH;
    const height = baseHeight || LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT;
    
    return { 
      width: width, 
      height: height
    };
  }

  /**
   * Clear all layout positions to force fresh ELK layout calculation
   */
  clearLayoutPositions(): void {
    // Clear positions for all visible containers
    const containers = this.state._collections.containers;
    for (const [containerId, container] of containers) {
      this.setContainerLayout(containerId, { position: undefined });
    }
    
    // CRITICAL: Clear positions for ALL nodes (both visible and hidden)
    for (const [nodeId, node] of this.state._collections.graphNodes) {
      this.setNodeLayout(nodeId, { position: undefined });
    }
  }

  /**
   * Validate and fix invalid dimensions
   */
  validateAndFixDimensions(): void {
    let fixedCount = 0;
    
    // Fix node dimensions
    for (const [nodeId, node] of this.state._collections.graphNodes) {
      let needsUpdate = false;
      const updates: any = {};
      
      if (!node.width || node.width <= 0) {
        updates.width = LAYOUT_CONSTANTS.DEFAULT_NODE_WIDTH;
        needsUpdate = true;
      }
      
      if (!node.height || node.height <= 0) {
        updates.height = LAYOUT_CONSTANTS.DEFAULT_NODE_HEIGHT;
        needsUpdate = true;
      }
      
      if (needsUpdate) {
        Object.assign(node, updates);
        fixedCount++;
      }
    }
    
    // Fix container dimensions
    for (const [containerId, container] of this.state._collections.containers) {
      let needsUpdate = false;
      const updates: any = {};
      
      if (container.width !== LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH) {
        updates.width = LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH;
        needsUpdate = true;
      }
      
      if (container.height !== LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT) {
        updates.height = LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT;
        needsUpdate = true;
      }
      
      if (needsUpdate) {
        Object.assign(container, updates);
        fixedCount++;
      }
    }
    
    if (fixedCount > 0) {
    }
  }

  /**
   * Get edge layout information (sections, routing)
   */
  getEdgeLayout(edgeId: string): { 
    sections?: Array<{
      startPoint?: { x: number; y: number };
      endPoint?: { x: number; y: number };
      bendPoints?: Array<{ x: number; y: number }>;
      [key: string]: any;
    }>;
    [key: string]: any;
  } | undefined {
    const edge = this.state._collections.graphEdges.get(edgeId);
    if (!edge) return undefined;
    
    return {
  sections: edge.sections || [],
      ...edge
    };
  }

  /**
   * Set edge layout information
   */
  setEdgeLayout(edgeId: string, layout: { 
    sections?: Array<{
      startPoint?: { x: number; y: number };
      endPoint?: { x: number; y: number };
      bendPoints?: Array<{ x: number; y: number }>;
      [key: string]: any;
    }>;
    [key: string]: any;
  }): void {
    const edge = this.state._collections.graphEdges.get(edgeId);
    if (!edge) return;
    
    Object.assign(edge, layout);
  }
}
