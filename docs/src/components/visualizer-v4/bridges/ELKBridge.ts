/**
 * @fileoverview ELK Bridge - Pure transformation bridge between VisualizationState and ELK
 * 
 * This bridge is a stateless transformation layer that:
 * - Converts VisualizationState data to ELK format
 * - Applies ELK layout results back to VisualizationState
 * - Contains NO business logic or state management
 * - All validation and business rules are handled by VisualizationState
 */

import { VisualizationState } from '../core/VisualizationState';
import type { LayoutConfig } from '../core/types';
import { getELKLayoutOptions } from '../shared/config';
import { validateCoordinate, validateDimension, createLayoutUpdate, isValidDimension } from '../core/BridgeUtils';

import ELK from 'elkjs';
import type { ElkGraph, ElkNode, ElkEdge } from './elk-types';

export class ELKBridge {
  /**
   * Convert VisualizationState to ELK format and run layout
   * Pure function - no state stored in bridge
   */
  async layoutVisState(visState: VisualizationState, layoutConfig: LayoutConfig = {}): Promise<void> {
    const config = { algorithm: 'mrtree', ...layoutConfig };
    const elk = new ELK(); // Create fresh ELK instance for this operation
    
    // Clear any existing edge layout data to ensure ReactFlow starts fresh
    visState.visibleEdges.forEach(edge => {
      try {
        visState.setEdgeLayout(edge.id, { sections: [] });
      } catch (error) {
        // Edge might not exist anymore, ignore
      }
    });
    
    // 1. Convert VisState to ELK format
    const elkGraph = this.visStateToELK(visState, config);
        
    // 2. Validate ELK input data
    this.validateELKInput(elkGraph);
    
    // 3. Yield control to browser to show loading state
    await new Promise(resolve => setTimeout(resolve, 10));
    
    const elkResult = await elk.layout(elkGraph);
    
    // 4. Yield control again before applying results
    await new Promise(resolve => setTimeout(resolve, 10));
    
    // 5. Apply results back to VisState
    this.elkToVisState(elkResult, visState);
  }

  /**
   * Validate ELK input data - minimal format validation only
   */
  private validateELKInput(elkGraph: ElkGraph): void {
    if (!elkGraph.children) elkGraph.children = [];
    if (!elkGraph.edges) elkGraph.edges = [];
    
    // Validate each node has required properties
    elkGraph.children.forEach(node => {
      if (!node.id) {
        throw new Error(`ELK node missing ID: ${JSON.stringify(node)}`);
      }
      if (!isValidDimension(node.width)) {
        throw new Error(`ELK node ${node.id} has invalid width: ${node.width}`);
      }
      if (!isValidDimension(node.height)) {
        throw new Error(`ELK node ${node.id} has invalid height: ${node.height}`);
      }
      
      // Validate children if this is a container
      if (node.children) {
        node.children.forEach(child => {
          if (!child.id) {
            throw new Error(`ELK child node missing ID: ${JSON.stringify(child)}`);
          }
          if (!isValidDimension(child.width)) {
            throw new Error(`ELK child node ${child.id} has invalid width: ${child.width}`);
          }
          if (!isValidDimension(child.height)) {
            throw new Error(`ELK child node ${child.id} has invalid height: ${child.height}`);
          }
        });
      }
    });
    
    // Get all valid node IDs for edge validation
    const allValidNodeIds = new Set<string>();
    const collectNodeIds = (elkNode: ElkNode) => {
      allValidNodeIds.add(elkNode.id);
      elkNode.children?.forEach(collectNodeIds);
    };
    elkGraph.children?.forEach(collectNodeIds);
    
    // Validate edges have valid endpoints
    elkGraph.edges?.forEach(edge => {
      const hasValidSource = edge.sources?.some(sourceId => allValidNodeIds.has(sourceId));
      const hasValidTarget = edge.targets?.some(targetId => allValidNodeIds.has(targetId));
      
      if (!hasValidSource || !hasValidTarget) {
        const sourceIds = edge.sources?.join(', ') || 'none';
        const targetIds = edge.targets?.join(', ') || 'none';
        
        throw new Error(
          `ELK edge ${edge.id} has invalid endpoints!\n` +
          `Sources: [${sourceIds}] (valid: ${hasValidSource})\n` +
          `Targets: [${targetIds}] (valid: ${hasValidTarget})`
        );
      }
      
      if (!edge.id) {
        throw new Error(`ELK edge missing ID: ${JSON.stringify(edge)}`);
      }
      if (!edge.sources || edge.sources.length === 0) {
        throw new Error(`ELK edge missing sources: ${edge.id}`);
      }
      if (!edge.targets || edge.targets.length === 0) {
        throw new Error(`ELK edge missing targets: ${edge.id}`);
      }
    });
  }

  /**
   * Convert VisState to ELK format
   * Pure transformation - no business logic
   */
  private visStateToELK(visState: VisualizationState, layoutConfig: LayoutConfig): ElkGraph {    
    // Build proper container hierarchy
    const rootNodes: ElkNode[] = [];
    const processedNodes = new Set<string>();
    const processedContainers = new Set<string>();
    
    // Helper function to build container hierarchy recursively
    const buildContainerHierarchy = (containerId: string): ElkNode => {
      const container = visState.getContainer(containerId);
      if (!container) {
        throw new Error(`Container ${containerId} not found`);
      }
      
      // Get validated dimensions
      const containerWidth = validateDimension(container.width, 200);
      const containerHeight = validateDimension(container.height, 150);
      
      const containerNode: ElkNode = {
        id: container.id,
        width: containerWidth,
        height: containerHeight,
        children: []
      };
      
      if (!container.collapsed) {
        // Use VisualizationState API to get children
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
              const nodeWidth = validateDimension(childNode.width, 180);
              const nodeHeight = validateDimension(childNode.height, 60);
                
              containerNode.children!.push({
                id: childNode.id,
                width: nodeWidth,
                height: nodeHeight
              });
              processedNodes.add(childId);
            }
          }
        });
        
        // Add a label node for expanded containers
        if (containerNode.children!.length > 0) {
          const labelNode: ElkNode = {
            id: `${container.id}_label`,
            width: Math.min(containerWidth * 0.6, 150),
            height: 20
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
          width: validateDimension(node.width, 180),
          height: validateDimension(node.height, 60)
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
      layoutOptions: getELKLayoutOptions(layoutConfig.algorithm)
    };
  }

  /**
   * Apply ELK results back to VisState
   * Pure transformation - no complex business logic
   */
  private elkToVisState(elkResult: ElkGraph, visState: VisualizationState): void {
    if (!elkResult.children) {
      console.warn('[ELKBridge] No children in ELK result');
      return;
    }
    
    // Apply positions to containers and nodes using ELK coordinates
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
  /**
   * Update container dimensions and child positions from ELK result
   */
  private updateContainerFromELK(elkNode: ElkNode, visState: VisualizationState): void {
    const layoutUpdates = createLayoutUpdate({
      x: elkNode.x,
      y: elkNode.y,
      width: elkNode.width,
      height: elkNode.height
    });
    
    if (Object.keys(layoutUpdates).length > 0) {
      visState.setContainerLayout(elkNode.id, layoutUpdates);
    }
    
    // Update child positions recursively
    elkNode.children?.forEach(elkChildNode => {
      // Handle label nodes - store label position with the container
      if (elkChildNode.id.endsWith('_label')) {
        const containerId = elkChildNode.id.replace('_label', '');
        const container = visState.getContainer(containerId);
        
        if (container) {
          const containerLayout = visState.getContainerLayout(containerId) || { 
            position: { x: container.x || 0, y: container.y || 0 },
            dimensions: { width: container.width || 200, height: container.height || 150 }
          };
          
          // Update container layout with label position information
          visState.setContainerLayout(containerId, {
            ...containerLayout,
            labelPosition: {
              x: validateCoordinate(elkChildNode.x, 0),
              y: validateCoordinate(elkChildNode.y, 0),
              width: validateDimension(elkChildNode.width, 150),
              height: validateDimension(elkChildNode.height, 20)
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
    const layoutUpdates = createLayoutUpdate({
      x: elkNode.x,
      y: elkNode.y,
      width: elkNode.width,
      height: elkNode.height
    });
    
    // Try to update as regular node first
    try {
      if (Object.keys(layoutUpdates).length > 0) {
        visState.setNodeLayout(elkNode.id, layoutUpdates);
      }
      return;
    } catch (nodeError) {
      // If not found as node, might be a collapsed container
      try {
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
