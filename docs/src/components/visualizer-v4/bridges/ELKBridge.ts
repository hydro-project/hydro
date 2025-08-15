/**
 * @fileoverview ELK Bridge - Clean interface between VisState and ELK
 * 
 * This bridge implements the core architectural principle:
 * - VisState contains ALL data (nodes, edges, containers) 
 * - ELK gets visible elements only through visibleEdges (hyperedges included transparently)
 * - ELK returns layout positions that get applied back to VisState
 */

import { VisualizationState } from '../core/VisualizationState';
import type { 
  GraphNode, 
  GraphEdge, 
  Container,
  HyperEdge
} from '../shared/types';
import type { LayoutConfig } from '../core/types';
import { getELKLayoutOptions } from '../shared/config';

import ELK from 'elkjs';
import type { ElkGraph, ElkNode, ElkEdge } from './elk-types';

export class ELKBridge {
  private elk: any; // ELK instance
  private layoutConfig: LayoutConfig;

  constructor(layoutConfig: LayoutConfig = {}) {
    this.elk = new ELK(); // ✅ Create fresh ELK instance for each ELKBridge
    this.layoutConfig = { algorithm: 'mrtree', ...layoutConfig };
  }

  /**
   * Update layout configuration
   */
  updateLayoutConfig(config: LayoutConfig): void {
    this.layoutConfig = { ...this.layoutConfig, ...config };
  }

  /**
   * Pure format translation: Convert VisualizationState to ELK format, run layout, and apply results
   * Business logic (validation, timing, etc.) should be handled by VisualizationEngine
   */
  async layoutVisState(visState: VisualizationState): Promise<void> {
    // 1. Extract all visible data from VisState
    const elkGraph = this.visStateToELK(visState);
        
    // 2. Validate ELK input data format (format validation only)
    this.validateELKInput(elkGraph);
    
    // 3. Run ELK layout
    const elkResult = await this.elk.layout(elkGraph);
    
    // 4. Apply results back to VisState
    this.elkToVisState(elkResult, visState);
  }

  /**
   * Validate ELK input data format to prevent errors - format validation only
   * NOTE: This should only validate format, not apply business rules
   */
  private validateELKInput(elkGraph: ElkGraph): void {
    // Ensure children array exists
    if (!elkGraph.children) {
      elkGraph.children = [];
    }
    
    // Ensure edges array exists
    if (!elkGraph.edges) {
      elkGraph.edges = [];
    }
    
    // Validate each node has required properties for ELK format
    elkGraph.children.forEach(node => {
      if (!node.id) {
        throw new Error(`ELK node missing ID: ${JSON.stringify(node)}`);
      }
      if (typeof node.width !== 'number' || node.width <= 0) {
        throw new Error(`ELK node ${node.id} has invalid width: ${node.width}`);
      }
      if (typeof node.height !== 'number' || node.height <= 0) {
        throw new Error(`ELK node ${node.id} has invalid height: ${node.height}`);
      }
      
      // Validate children if this is a container
      if (node.children) {
        node.children.forEach(child => {
          if (!child.id) {
            throw new Error(`ELK child node missing ID: ${JSON.stringify(child)}`);
          }
          if (typeof child.width !== 'number' || child.width <= 0) {
            throw new Error(`ELK child node ${child.id} has invalid width: ${child.width}`);
          }
          if (typeof child.height !== 'number' || child.height <= 0) {
            throw new Error(`ELK child node ${child.id} has invalid height: ${child.height}`);
          }
        });
      }
    });
    
    // Get all valid node IDs from the ELK graph for edge validation
    const allValidNodeIds = new Set<string>();
    const collectNodeIds = (elkNode: ElkNode) => {
      allValidNodeIds.add(elkNode.id);
      elkNode.children?.forEach(collectNodeIds);
    };
    elkGraph.children?.forEach(collectNodeIds);
    
    // Validate edge format - ensure all edges reference valid nodes
    elkGraph.edges?.forEach(edge => {
      if (!edge.id) {
        throw new Error(`ELK edge missing ID: ${JSON.stringify(edge)}`);
      }
      if (!edge.sources || edge.sources.length === 0) {
        throw new Error(`ELK edge missing sources: ${edge.id}`);
      }
      if (!edge.targets || edge.targets.length === 0) {
        throw new Error(`ELK edge missing targets: ${edge.id}`);
      }
      
      const hasValidSource = edge.sources?.some(sourceId => allValidNodeIds.has(sourceId));
      const hasValidTarget = edge.targets?.some(targetId => allValidNodeIds.has(targetId));
      
      if (!hasValidSource || !hasValidTarget) {
        const sourceIds = edge.sources?.join(', ') || 'none';
        const targetIds = edge.targets?.join(', ') || 'none';
        const availableNodes = Array.from(allValidNodeIds).slice(0, 10).join(', ') + (allValidNodeIds.size > 10 ? '...' : '');
        
        throw new Error(
          `ELKBridge received edge ${edge.id} with invalid endpoints!\n` +
          `Sources: [${sourceIds}] (valid: ${hasValidSource})\n` +
          `Targets: [${targetIds}] (valid: ${hasValidTarget})\n` +
          `Available nodes: ${availableNodes}`
        );
      }
    });
  }

  /**
   * Convert VisState to ELK format
   * HIERARCHICAL: Use proper ELK hierarchy to match ReactFlow parent-child relationships
   */
  private visStateToELK(visState: VisualizationState): ElkGraph {    
    // HIERARCHICAL: Build proper container hierarchy
    const rootNodes: ElkNode[] = [];
    const processedNodes = new Set<string>();
    const processedContainers = new Set<string>();
    
    // Helper function to build container hierarchy recursively
    const buildContainerHierarchy = (containerId: string): ElkNode => {
      const container = visState.getContainer(containerId);
      if (!container) {
        throw new Error(`Container ${containerId} not found`);
      }
      
      // Ensure valid dimensions - fallback to defaults if invalid
      const containerWidth = (typeof container.width === 'number' && !isNaN(container.width) && isFinite(container.width)) 
        ? container.width : 200;
      const containerHeight = (typeof container.height === 'number' && !isNaN(container.height) && isFinite(container.height)) 
        ? container.height : 150;
      
      const containerNode: ElkNode = {
        id: container.id,
        width: containerWidth,
        height: containerHeight,
        children: []
      };
      
      if (!container.collapsed) {
        // Use VisualizationState API to get children (returns Set)
        const containerChildren = visState.getContainerChildren(container.id);
        containerChildren.forEach(childId => {
          // Check if child is a container
          const childContainer = visState.getContainer(childId);
          if (childContainer && visState.visibleContainers.some(vc => vc.id === childId)) {
            // Add child container recursively
            const childContainerNode = buildContainerHierarchy(childId);
            containerNode.children!.push(childContainerNode);
            processedContainers.add(childId);
          } else {
            // Add child node
            const childNode = visState.visibleNodes.find(n => n.id === childId);
            if (childNode) {
              // Ensure valid node dimensions
              const nodeWidth = (typeof childNode.width === 'number' && !isNaN(childNode.width) && isFinite(childNode.width)) 
                ? childNode.width : 180;
              const nodeHeight = (typeof childNode.height === 'number' && !isNaN(childNode.height) && isFinite(childNode.height)) 
                ? childNode.height : 60;
                
              containerNode.children!.push({
                id: childNode.id,
                width: nodeWidth,
                height: nodeHeight
              });
              processedNodes.add(childId);
            }
          }
        });
        
        // Add a label node for expanded containers to ensure ELK accounts for label space
        // This acts as a virtual node that reserves space for the container's label
        if (containerNode.children!.length > 0) { // Only add label if container has content
          const labelNode: ElkNode = {
            id: `${container.id}_label`,
            width: Math.min(containerWidth * 0.6, 150), // Label width (smaller to not dominate layout)
            height: 20, // Compact label height
            layoutOptions: {
              // Let ELK position the label node naturally among other children
              // No fixed positioning - ELK will place it where it fits best
            }
          };
          containerNode.children!.push(labelNode);
        }
      }
      
      return containerNode;
    };
    
    // Add only root-level containers to rootNodes
    visState.visibleContainers.forEach(container => {
      // Check if this container has a parent that's also visible
      const hasVisibleParent = visState.visibleContainers.some(otherContainer => 
        visState.getContainerChildren(otherContainer.id).has(container.id)
      );
      
      if (!hasVisibleParent && !processedContainers.has(container.id)) {
        const containerNode = buildContainerHierarchy(container.id);
        rootNodes.push(containerNode);
        processedContainers.add(container.id);
      }
    });
    
    // Add any uncontained nodes at root level
    visState.visibleNodes.forEach(node => {
      if (!processedNodes.has(node.id)) {
        rootNodes.push({
          id: node.id,
          width: node.width || 180,
          height: node.height || 60
        });
      }
    });
    
    // Convert edges - ELK will handle hierarchy automatically
    const allEdges: ElkEdge[] = Array.from(visState.visibleEdges).map(edge => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target]
    }));
    
    return {
      id: 'root',
      children: rootNodes,
      edges: allEdges,
      layoutOptions: getELKLayoutOptions(this.layoutConfig.algorithm)
    };
  }

  /**
   * Apply ELK results back to VisState - pure format translation
   */
  private elkToVisState(elkResult: ElkGraph, visState: VisualizationState): void {
    if (!elkResult.children) {
      console.warn('[ELKBridge] ⚠️ No children in ELK result');
      return;
    }
    
    // Apply positions to containers and nodes using ELK coordinates directly
    elkResult.children.forEach(elkNode => {
      // Check if this ID exists as a container in VisState first
      try {
        const container = visState.getContainer(elkNode.id);
        if (container) {
          this.updateContainerFromELK(elkNode, visState);
          return;
        }
      } catch (e) {
        // Not a container, continue to node logic
      }
      
      // Handle as node or container based on ELK structure
      if (elkNode.children && elkNode.children.length > 0) {
        this.updateContainerFromELK(elkNode, visState);
      } else {
        this.updateNodeFromELK(elkNode, visState);
      }
    });
  }
  
  // REMOVED: applyOffsetToChildren - dead code in canonical flat pattern
  
  // REMOVED: updateEdgeFromELK - ReactFlow handles all edge routing automatically

  /**
   * Update container dimensions and child positions from ELK result
   */
  private updateContainerFromELK(elkNode: ElkNode, visState: VisualizationState): void {
    const layoutUpdates: any = {};
    
    // Validate and set position
    if (elkNode.x !== undefined || elkNode.y !== undefined) {
      layoutUpdates.position = {};
      
      if (elkNode.x !== undefined && typeof elkNode.x === 'number' && !isNaN(elkNode.x) && isFinite(elkNode.x)) {
        layoutUpdates.position.x = elkNode.x;
      }
      
      if (elkNode.y !== undefined && typeof elkNode.y === 'number' && !isNaN(elkNode.y) && isFinite(elkNode.y)) {
        layoutUpdates.position.y = elkNode.y;
      }
    }
    
    // Validate and set dimensions
    if (elkNode.width !== undefined || elkNode.height !== undefined) {
      layoutUpdates.dimensions = {};
      
      if (elkNode.width !== undefined && typeof elkNode.width === 'number' && !isNaN(elkNode.width) && isFinite(elkNode.width) && elkNode.width > 0) {
        layoutUpdates.dimensions.width = elkNode.width;
      }
      
      if (elkNode.height !== undefined && typeof elkNode.height === 'number' && !isNaN(elkNode.height) && isFinite(elkNode.height) && elkNode.height > 0) {
        layoutUpdates.dimensions.height = elkNode.height;
      }
    }
    
    if (Object.keys(layoutUpdates).length > 0) {
      visState.setContainerLayout(elkNode.id, layoutUpdates);
    }
    
    // Update child positions (recursively handle containers vs nodes)
    elkNode.children?.forEach(elkChildNode => {
      // Handle label nodes - store label position with the container
      if (elkChildNode.id.endsWith('_label')) {
        const containerId = elkChildNode.id.replace('_label', '');
        const container = visState.getContainer(containerId);
        
        if (container) {
          // Store label positioning information with the container
          const containerLayout = visState.getContainerLayout(containerId) || { 
            position: { x: container.x || 0, y: container.y || 0 },
            dimensions: { width: container.width || 200, height: container.height || 150 }
          };
          
          // Update container layout with label position information
          visState.setContainerLayout(containerId, {
            ...containerLayout,
            labelPosition: {
              x: elkChildNode.x || 0,
              y: elkChildNode.y || 0,
              width: elkChildNode.width || 150,
              height: elkChildNode.height || 20
            }
          });
        }
        
        return;
      }
      
      if (elkChildNode.children && elkChildNode.children.length > 0) {
        // This child is also a container - recurse into it
        this.updateContainerFromELK(elkChildNode, visState);
      } else {
        // This child is a leaf node - update its position
        this.updateNodeFromELK(elkChildNode, visState);
      }
    });
  }

  /**
   * Update node position from ELK result
   */
  private updateNodeFromELK(elkNode: ElkNode, visState: VisualizationState): void {
    // Try to update as regular node first
    try {
      const layoutUpdates: any = {};
      
      // Set position
      if (elkNode.x !== undefined || elkNode.y !== undefined) {
        layoutUpdates.position = {};
        
        if (elkNode.x !== undefined && typeof elkNode.x === 'number' && !isNaN(elkNode.x) && isFinite(elkNode.x)) {
          layoutUpdates.position.x = elkNode.x;
        }
        
        if (elkNode.y !== undefined && typeof elkNode.y === 'number' && !isNaN(elkNode.y) && isFinite(elkNode.y)) {
          layoutUpdates.position.y = elkNode.y;
        }
      }
      
      // Set dimensions
      if (elkNode.width !== undefined || elkNode.height !== undefined) {
        layoutUpdates.dimensions = {};
        
        if (elkNode.width !== undefined && typeof elkNode.width === 'number' && !isNaN(elkNode.width) && isFinite(elkNode.width) && elkNode.width > 0) {
          layoutUpdates.dimensions.width = elkNode.width;
        }
        
        if (elkNode.height !== undefined && typeof elkNode.height === 'number' && !isNaN(elkNode.height) && isFinite(elkNode.height) && elkNode.height > 0) {
          layoutUpdates.dimensions.height = elkNode.height;
        }
      }
      
      if (Object.keys(layoutUpdates).length > 0) {
        visState.setNodeLayout(elkNode.id, layoutUpdates);
      }
      return;
    } catch (nodeError) {
      // If not found as node, might be a collapsed container
      try {
        const layoutUpdates: any = {};
        
        if (elkNode.x !== undefined || elkNode.y !== undefined) {
          layoutUpdates.position = {};
          
          if (elkNode.x !== undefined && typeof elkNode.x === 'number' && !isNaN(elkNode.x) && isFinite(elkNode.x)) {
            layoutUpdates.position.x = elkNode.x;
          }
          
          if (elkNode.y !== undefined && typeof elkNode.y === 'number' && !isNaN(elkNode.y) && isFinite(elkNode.y)) {
            layoutUpdates.position.y = elkNode.y;
          }
        }
        
        if (elkNode.width !== undefined || elkNode.height !== undefined) {
          layoutUpdates.dimensions = {};
          
          if (elkNode.width !== undefined && typeof elkNode.width === 'number' && !isNaN(elkNode.width) && isFinite(elkNode.width) && elkNode.width > 0) {
            layoutUpdates.dimensions.width = elkNode.width;
          }
          
          if (elkNode.height !== undefined && typeof elkNode.height === 'number' && !isNaN(elkNode.height) && isFinite(elkNode.height) && elkNode.height > 0) {
            layoutUpdates.dimensions.height = elkNode.height;
          }
        }
        
        if (Object.keys(layoutUpdates).length > 0) {
          visState.setContainerLayout(elkNode.id, layoutUpdates);
        }
        return;
      } catch (containerError) {
        console.warn(`[ELKBridge] Node/Container ${elkNode.id} not found in VisState`);
      }
    }
  }

}
