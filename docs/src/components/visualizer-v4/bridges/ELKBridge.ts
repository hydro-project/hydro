/**
 * @fileoverview ELK Bridge - Clean interface between VisState and ELK
 * 
 * This bridge implements the core architectural principle:
 * - VisState contains ALL data (nodes, edges, containers) 
 * - ELK gets visible elements only through visibleEdges (hyperedges included transparently)
 * - ELK returns layout positions that get applied back to VisState
 */

import { VisualizationState } from '../core/VisualizationState';
import type { Container } from '../shared/types';
import type { LayoutConfig } from '../core/types';
import { getELKLayoutOptions } from '../shared/config';

import ELK from 'elkjs';
import type { ElkGraph, ElkNode, ElkEdge } from './elk-types';

// Minimal runtime-accurate type for the ELK instance we use
type ElkInstance = {
  layout(graph: ElkGraph): Promise<ElkGraph>;
};

export class ELKBridge {
  private elk: ElkInstance; // ELK instance
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
   * Convert VisState to ELK format and run layout
   * Key insight: Include ALL visible edges (regular + hyper) with no distinction
   */
  async layoutVisState(visState: VisualizationState): Promise<void> {
    // Clear any existing edge layout data to ensure ReactFlow starts fresh
    visState.visibleEdges.forEach(edge => {
      try {
        visState.setEdgeLayout(edge.id, { sections: [] });
      } catch (error) {
        // Edge might not exist anymore, ignore
      }
    });
    
    // 1. Extract all visible data from VisState
    const elkGraph = this.visStateToELK(visState);
        
    // 2. Validate ELK input data
    this.validateELKInput(elkGraph);
    
    // 3. Yield control to browser to show loading state
    await new Promise(resolve => setTimeout(resolve, 10));
        
    // Debug: Check for data leaks in large graphs
    if ((elkGraph.children?.length || 0) > 10) {
      // CRITICAL: Check if we're accidentally including children of collapsed containers
      const leaks: string[] = [];
      for (const container of (elkGraph.children || [])) {
        // FIXED: Only check for leaks if container is marked as collapsed
        // Expanded containers (collapsed=false) are SUPPOSED to have children!
        // Check the original container state from visState
        const originalContainer = visState.getContainer(container.id);
        if (originalContainer?.collapsed && container.children && container.children.length > 0) {
          const leakMsg = `Container ${container.id} has ${container.children.length} children but should be collapsed!`;
          console.warn(`[ELKBridge] ⚠️  LEAK: ${leakMsg}`);
          console.warn(`[ELKBridge] ⚠️    Children: ${container.children.map(c => c.id).slice(0, 3).join(', ')}${container.children.length > 3 ? '...' : ''}`);
          leaks.push(leakMsg);
        }
      }
      
      // In test environments, throw an error if we have leaks
      // Note: Using globalThis to check for test environment since process may not be available in browser
      const isTestEnvironment = typeof globalThis !== 'undefined' && 
        (globalThis.process?.env?.NODE_ENV === 'test' || globalThis.process?.env?.VITEST === 'true');
      
      if (leaks.length > 0 && isTestEnvironment) {
        throw new Error(`ELK CONTAINER LEAKS DETECTED: ${leaks.length} collapsed containers have visible children. This violates the collapsed container invariant. Leaks: ${leaks.slice(0, 3).join('; ')}`);
      }
    }
    
    const elkResult = await this.elk.layout(elkGraph);
    
    const elkOutputContainers = (elkResult.children || []);
    
    // Calculate actual spacing from ELK results
    const sortedByX = elkOutputContainers
      .filter(c => c.x !== undefined)
      .sort((a, b) => (a.x || 0) - (b.x || 0));
    
    if (sortedByX.length > 1) {
      const gaps = [];
      for (let i = 1; i < sortedByX.length; i++) {
        const gap = (sortedByX[i].x || 0) - ((sortedByX[i-1].x || 0) + (sortedByX[i-1].width || 0));
        gaps.push(gap);
      }
      const avgGap = gaps.reduce((a, b) => a + b, 0) / gaps.length;
    }
    
    // 5. Yield control again before applying results
    await new Promise(resolve => setTimeout(resolve, 10));
    
    // 6. Apply results back to VisState
    this.elkToVisState(elkResult, visState);
  }

  /**
   * Log ELK graph structure for debugging layout issues
   */
  private logELKGraphStructure(elkGraph: ElkGraph): void {
    
    // Log container positions if they exist (this might be the issue)
    const containersWithPositions = (elkGraph.children || []).filter(child => 
      child.x !== undefined || child.y !== undefined
    );
    
    if (containersWithPositions.length > 0) {
      for (const _container of containersWithPositions) { /* intentional no-op for now */ }
    } else {
      /* no-op */
    }
    
    // Log ALL container dimensions to see if there are inconsistencies
    const containers = (elkGraph.children || []);
  for (const _container of containers) { /* no-op */ }
    
    // CRITICAL: Log the exact layout options being sent
  }

  /**
   * Validate ELK input data to prevent null reference errors
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
    
    // Validate each node has required properties (VisualizationState should ensure this)
    elkGraph.children.forEach(node => {
      if (!node.id) {
        throw new Error(`ELK node missing ID: ${JSON.stringify(node)}`);
      }
      if (typeof node.width !== 'number' || node.width <= 0) {
        throw new Error(`ELK node ${node.id} has invalid width: ${node.width}. VisualizationState should provide valid dimensions.`);
      }
      if (typeof node.height !== 'number' || node.height <= 0) {
        throw new Error(`ELK node ${node.id} has invalid height: ${node.height}. VisualizationState should provide valid dimensions.`);
      }
      
      // Validate children if this is a container
      if (node.children) {
        node.children.forEach(child => {
          if (!child.id) {
            throw new Error(`ELK child node missing ID: ${JSON.stringify(child)}`);
          }
          if (typeof child.width !== 'number' || child.width <= 0) {
            throw new Error(`ELK child node ${child.id} has invalid width: ${child.width}. VisualizationState should provide valid dimensions.`);
          }
          if (typeof child.height !== 'number' || child.height <= 0) {
            throw new Error(`ELK child node ${child.id} has invalid height: ${child.height}. VisualizationState should provide valid dimensions.`);
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
    
    if (allValidNodeIds.size < 20) {
    }
    
    // STRICT VALIDATION: Throw error for edges with invalid endpoints instead of silently filtering
    // This forces VisualizationState to provide clean, valid data
    elkGraph.edges?.forEach(edge => {
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
          `Available nodes: ${availableNodes}\n` +
          `This indicates a bug in VisualizationState - it should not send edges that reference non-existent nodes.`
        );
      }
    });
    
    // Validate each remaining edge has required properties
    elkGraph.edges.forEach(edge => {
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
      
      // DIAGNOSTIC: Check parent relationships for problematic containers
      if (container.id === 'bt_81' || container.id === 'bt_98') {
        if (hasVisibleParent) {
          // Find the parent
          const parent = visState.visibleContainers.find(otherContainer => 
            visState.getContainerChildren(otherContainer.id).has(container.id)
          );
        }
      }
      
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
   * Apply ELK results back to VisState
   */
  private elkToVisState(elkResult: ElkGraph, visState: VisualizationState): void {
    if (!elkResult.children) {
      console.warn('[ELKBridge] ⚠️ No children in ELK result');
      return;
    }
    
    // SIMPLIFIED: Use ELK coordinates directly following ReactFlow best practices
    // No custom offset corrections - ELK provides the correct coordinate system
    
    // DIAGNOSTIC: Check if bt_81 and bt_98 are in the ELK result
  const problemContainers: string[] = ['bt_81', 'bt_98'];
    problemContainers.forEach(containerId => {
  const foundAtRoot = elkResult.children?.find(child => child.id === containerId);
      if (foundAtRoot) {
      } else {
        // Check if it's nested in any other container
        let foundNested = false;
  const searchNested = (elkNode: ElkNode, depth: number = 0) => {
          elkNode.children?.forEach(child => {
            if (child.id === containerId) {
              foundNested = true;
            } else if (child.children) {
              searchNested(child, depth + 1);
            }
          });
        };
        elkResult.children?.forEach(rootChild => searchNested(rootChild, 1));
        
        if (!foundNested) {
        }
      }
    });
    
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
    
    // SIMPLIFIED: No edge routing processing - let ReactFlow handle all edge positioning
    // ReactFlow will automatically route edges between nodes using handles
  }
  
  // REMOVED: applyOffsetToChildren - dead code in canonical flat pattern
  
  // REMOVED: updateEdgeFromELK - ReactFlow handles all edge routing automatically

  /**
   * Update container dimensions and child positions from ELK result
   */
  private updateContainerFromELK(elkNode: ElkNode, visState: VisualizationState): void {
    // Use VisState's proper layout methods instead of direct property access
    const layoutUpdates: {
      position?: Partial<{ x: number; y: number }>;
      dimensions?: Partial<{ width: number; height: number }>;
    } = {};
    
    // Validate and set position
    if (elkNode.x !== undefined || elkNode.y !== undefined) {
      layoutUpdates.position = {};
      
      // Validate x coordinate
      if (elkNode.x !== undefined) {
        if (typeof elkNode.x === 'number' && !isNaN(elkNode.x) && isFinite(elkNode.x)) {
          layoutUpdates.position.x = elkNode.x;
        } else {
          console.error(`[ELKBridge] ❌ LAYOUT BUG: Invalid x coordinate for container ${elkNode.id}: ${elkNode.x} (type: ${typeof elkNode.x})`);
          layoutUpdates.position.x = 0; // Temporarily fallback to see what's happening
        }
      }
      
      // Validate y coordinate  
      if (elkNode.y !== undefined) {
        if (typeof elkNode.y === 'number' && !isNaN(elkNode.y) && isFinite(elkNode.y)) {
          layoutUpdates.position.y = elkNode.y;
        } else {
          console.error(`[ELKBridge] ❌ LAYOUT BUG: Invalid y coordinate for container ${elkNode.id}: ${elkNode.y} (type: ${typeof elkNode.y})`);
          layoutUpdates.position.y = 0; // Temporarily fallback to see what's happening
        }
      }
    } else {
      // ELK didn't provide ANY position coordinates - this is also a bug!
      console.error(`[ELKBridge] ❌ LAYOUT BUG: ELK provided no position coordinates for container ${elkNode.id}`);
      // Temporarily use origin fallback to see what's happening
      layoutUpdates.position = { x: 0, y: 0 };
    }
    
    // Validate and set dimensions
    if (elkNode.width !== undefined || elkNode.height !== undefined) {
      layoutUpdates.dimensions = {};
      
      // Validate width
      if (elkNode.width !== undefined) {
        if (typeof elkNode.width === 'number' && !isNaN(elkNode.width) && isFinite(elkNode.width) && elkNode.width > 0) {
          layoutUpdates.dimensions.width = elkNode.width;
        } else {
          console.error(`[ELKBridge] Invalid width for container ${elkNode.id}: ${elkNode.width}`);
          layoutUpdates.dimensions.width = 200; // Fallback
        }
      }
      
      // Validate height
      if (elkNode.height !== undefined) {
        if (typeof elkNode.height === 'number' && !isNaN(elkNode.height) && isFinite(elkNode.height) && elkNode.height > 0) {
          layoutUpdates.dimensions.height = elkNode.height;
        } else {
          console.error(`[ELKBridge] Invalid height for container ${elkNode.id}: ${elkNode.height}`);
          layoutUpdates.dimensions.height = 150; // Fallback
        }
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
          // This will be used by the container component to render the label
          const containerLayout = visState.getContainerLayout(containerId) || { 
            position: { x: container.x || 0, y: container.y || 0 },
            dimensions: { width: container.width || 200, height: container.height || 150 }
          };
          
          // Update container layout with label position information
          visState.setContainerLayout(containerId, {
            ...containerLayout,
            // Coerce numeric shapes for TS
            position: containerLayout.position ? { x: Number(containerLayout.position.x) || 0, y: Number(containerLayout.position.y) || 0 } : undefined,
            dimensions: containerLayout.dimensions ? { width: Number(containerLayout.dimensions.width) || 0, height: Number(containerLayout.dimensions.height) || 0 } : undefined,
            labelPosition: {
              x: Number(elkChildNode.x) || 0,
              y: Number(elkChildNode.y) || 0,
              width: Number(elkChildNode.width) || 150,
              height: Number(elkChildNode.height) || 20
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
    // Try to update as regular node first using VisState's layout methods
    try {
      const layoutUpdates: {
        position?: Partial<{ x: number; y: number }>;
        dimensions?: Partial<{ width: number; height: number }>;
      } = {};
      
      // Validate and set position
      if (elkNode.x !== undefined || elkNode.y !== undefined) {
        layoutUpdates.position = {};
        
        if (elkNode.x !== undefined) {
          if (typeof elkNode.x === 'number' && !isNaN(elkNode.x) && isFinite(elkNode.x)) {
            layoutUpdates.position.x = elkNode.x;
          } else {
            console.error(`[ELKBridge] Invalid x coordinate for node ${elkNode.id}: ${elkNode.x}`);
            layoutUpdates.position.x = 0;
          }
        }
        
        if (elkNode.y !== undefined) {
          if (typeof elkNode.y === 'number' && !isNaN(elkNode.y) && isFinite(elkNode.y)) {
            layoutUpdates.position.y = elkNode.y;
          } else {
            console.error(`[ELKBridge] Invalid y coordinate for node ${elkNode.id}: ${elkNode.y}`);
            layoutUpdates.position.y = 0;
          }
        }
      }
      
      // Validate and set dimensions
      if (elkNode.width !== undefined || elkNode.height !== undefined) {
        layoutUpdates.dimensions = {};
        
        if (elkNode.width !== undefined) {
          if (typeof elkNode.width === 'number' && !isNaN(elkNode.width) && isFinite(elkNode.width) && elkNode.width > 0) {
            layoutUpdates.dimensions.width = elkNode.width;
          } else {
            console.error(`[ELKBridge] Invalid width for node ${elkNode.id}: ${elkNode.width}`);
            layoutUpdates.dimensions.width = 180;
          }
        }
        
        if (elkNode.height !== undefined) {
          if (typeof elkNode.height === 'number' && !isNaN(elkNode.height) && isFinite(elkNode.height) && elkNode.height > 0) {
            layoutUpdates.dimensions.height = elkNode.height;
          } else {
            console.error(`[ELKBridge] Invalid height for node ${elkNode.id}: ${elkNode.height}`);
            layoutUpdates.dimensions.height = 60;
          }
        }
      }
      
      if (Object.keys(layoutUpdates).length > 0) {
        visState.setNodeLayout(elkNode.id, layoutUpdates);
      }
      return;
    } catch (nodeError) {
      // If not found as node, might be a collapsed container - apply same validation
      try {
        const layoutUpdates: {
          position?: Partial<{ x: number; y: number }>;
          dimensions?: Partial<{ width: number; height: number }>;
        } = {};
        
        if (elkNode.x !== undefined || elkNode.y !== undefined) {
          layoutUpdates.position = {};
          
          if (elkNode.x !== undefined) {
            if (typeof elkNode.x === 'number' && !isNaN(elkNode.x) && isFinite(elkNode.x)) {
              layoutUpdates.position.x = elkNode.x;
            } else {
              console.error(`[ELKBridge] Invalid x coordinate for container ${elkNode.id}: ${elkNode.x}`);
              layoutUpdates.position.x = 0;
            }
          }
          
          if (elkNode.y !== undefined) {
            if (typeof elkNode.y === 'number' && !isNaN(elkNode.y) && isFinite(elkNode.y)) {
              layoutUpdates.position.y = elkNode.y;
            } else {
              console.error(`[ELKBridge] Invalid y coordinate for container ${elkNode.id}: ${elkNode.y}`);
              layoutUpdates.position.y = 0;
            }
          }
        }
        
        if (elkNode.width !== undefined || elkNode.height !== undefined) {
          layoutUpdates.dimensions = {};
          
          if (elkNode.width !== undefined) {
            if (typeof elkNode.width === 'number' && !isNaN(elkNode.width) && isFinite(elkNode.width) && elkNode.width > 0) {
              layoutUpdates.dimensions.width = elkNode.width;
            } else {
              console.error(`[ELKBridge] Invalid width for container ${elkNode.id}: ${elkNode.width}`);
              layoutUpdates.dimensions.width = 200;
            }
          }
          
          if (elkNode.height !== undefined) {
            if (typeof elkNode.height === 'number' && !isNaN(elkNode.height) && isFinite(elkNode.height) && elkNode.height > 0) {
              layoutUpdates.dimensions.height = elkNode.height;
            } else {
              console.error(`[ELKBridge] Invalid height for container ${elkNode.id}: ${elkNode.height}`);
              layoutUpdates.dimensions.height = 150;
            }
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

  /**
   * Get containers requiring layout (moved from VisualizationState)
   * This is ELK-specific logic for determining which containers need layout
   */
  getContainersRequiringLayout(visState: VisualizationState, changedContainerId?: string): ReadonlyArray<Container> {
    // Return all visible containers that need layout
  return visState.visibleContainers.filter(container => !container.hidden) as ReadonlyArray<Container>;
  }

  /**
   * Get container ELK fixed status (moved from VisualizationState)
   * This is ELK-specific logic for tracking fixed positions
   */
  getContainerELKFixed(visState: VisualizationState, containerId: string): boolean {
    const container = visState.getContainer(containerId);
    if (!container) return false;

  const fixed = (container as { elkFixed?: unknown }).elkFixed;
  return typeof fixed === 'boolean' ? fixed : false;
  }

}
