/**
 * Bridge Compatibility Layer
 * 
 * Provides backwards compatibility methods for existing bridge systems
 * (ReactFlow, ELK, etc.) while the core API evolves. Extracted from VisState.ts
 * for better separation of concerns.
 */

import { getELKLayoutOptions } from '../../shared/config';

export class BridgeCompatibility {
  private readonly state: any;

  constructor(state: any) {
    this.state = state;
  }

  /**
   * Get parent-child mapping for ReactFlow bridge
   */
  getParentChildMap(): Map<string, string> {
    const parentMap = new Map<string, string>();
    
    // Map visible nodes to their expanded parent containers
    for (const node of this.state.visibleNodes) {
      const parentContainer = this.state._collections.nodeContainers.get(node.id);
      if (parentContainer) {
        const parent = this.state._collections.containers.get(parentContainer);
        if (parent && !parent.collapsed && !parent.hidden) {
          parentMap.set(node.id, parentContainer);
        }
      }
    }
    
    // Also handle containers defined with children arrays (for test compatibility)
    for (const [containerId, container] of this.state._collections.containers) {
      if (!container.collapsed && !container.hidden && container.children) {
        for (const childId of container.children) {
          parentMap.set(childId, containerId);
        }
      }
    }
    
    // Also map visible containers to their parent containers
    for (const container of this.state.visibleContainers) {
      for (const [parentId, children] of this.state._collections.containerChildren) {
        if (children.has(container.id)) {
          const parent = this.state._collections.containers.get(parentId);
          if (parent && !parent.collapsed && !parent.hidden) {
            parentMap.set(container.id, parentId);
          }
          break;
        }
      }
    }
    
    return parentMap;
  }

  /**
   * Get edge handles for ReactFlow bridge
   */
  getEdgeHandles(edgeId: string): { sourceHandle?: string; targetHandle?: string } {
    const edge = this.state._collections.graphEdges.get(edgeId);
    if (!edge) return {};
    
    return {
      sourceHandle: edge.sourceHandle || 'default-out',
      targetHandle: edge.targetHandle || 'default-in'
    };
  }

  /**
   * Get collapsed containers as nodes for ELK bridge
   */
  getCollapsedContainersAsNodes(): ReadonlyArray<any> {
    const collapsedAsNodes = [];
    
    for (const container of this.state._collections.containers.values()) {
      if (container.collapsed && !container.hidden) {
        collapsedAsNodes.push({
          ...container,
          x: container.x ?? 0,
          y: container.y ?? 0,
          label: container.label || container.id,
          style: container.style || 'default',
          type: 'container-node',
          collapsed: true
        });
      }
    }
    
    return collapsedAsNodes;
  }

  /**
   * Get containers requiring layout (ELK bridge compatibility)
   */
  getContainersRequiringLayout(): ReadonlyArray<any> {
    // Return all visible containers that need layout
    return this.state.visibleContainers.map((container: any) => ({
      ...container,
      elkFixed: false
    }));
  }

  /**
   * Get top-level nodes (not inside any expanded container)
   */
  getTopLevelNodes(): ReadonlyArray<any> {
    const topLevelNodes = [];
    
    for (const node of this.state.visibleNodes) {
      const parentContainer = this.state._collections.nodeContainers.get(node.id);
      if (!parentContainer) {
        // Node has no parent container - it's top level
        topLevelNodes.push(node);
      } else {
        const parent = this.state._collections.containers.get(parentContainer);
        if (!parent || parent.collapsed || parent.hidden) {
          // Parent is collapsed/hidden - node appears at top level
          topLevelNodes.push(node);
        }
      }
    }
    
    return topLevelNodes;
  }

  /**
   * Get top-level containers (containers with no visible parent container)
   */
  getTopLevelContainers(): ReadonlyArray<any> {
    const topLevelContainers = [];
    
    for (const container of this.state.visibleContainers) {
      // Check if this container has a parent container
      let hasVisibleParent = false;
      
      for (const [parentId, children] of this.state._collections.containerChildren) {
        if (children.has(container.id)) {
          const parent = this.state._collections.containers.get(parentId);
          if (parent && !parent.collapsed && !parent.hidden) {
            hasVisibleParent = true;
            break;
          }
        }
      }
      
      if (!hasVisibleParent) {
        // Container has no visible parent - it's top level
        topLevelContainers.push(container);
      }
    }
    
    return topLevelContainers;
  }

  /**
   * Get container ELK fixed status (minimal bridge compatibility)
   */
  getContainerELKFixed(containerId: string): boolean {
    const container = this.state._collections.containers.get(containerId);
    return container?.elkFixed || false;
  }

  /**
   * Convert VisualizationState to ELK graph format
   * This method handles all the business logic that was previously in ELKBridge
   */
  toELKGraph(): any {
    // Build ELK hierarchy
    const rootNodes: any[] = [];
    const processedNodes = new Set<string>();
    const processedContainers = new Set<string>();
    
    // Helper function to build container hierarchy recursively
    const buildContainerHierarchy = (containerId: string): any => {
      const container = this.state._collections.containers.get(containerId);
      if (!container) {
        throw new Error(`Container ${containerId} not found`);
      }
      
      // Ensure valid dimensions - fallback to defaults if invalid
      const containerWidth = (typeof container.width === 'number' && !isNaN(container.width) && isFinite(container.width)) 
        ? container.width : 200;
      const containerHeight = (typeof container.height === 'number' && !isNaN(container.height) && isFinite(container.height)) 
        ? container.height : 150;
      
      const containerNode: any = {
        id: container.id,
        width: containerWidth,
        height: containerHeight,
        children: []
      };
      
      if (!container.collapsed) {
        // Use VisualizationState API to get children
        const containerChildren = this.state._collections.containerChildren.get(container.id) || new Set();
        containerChildren.forEach((childId: string) => {
          // Check if child is a container
          const childContainer = this.state._collections.containers.get(childId);
          if (childContainer && this.state.visibleContainers.some((vc: any) => vc.id === childId)) {
            // Add child container recursively
            const childContainerNode = buildContainerHierarchy(childId);
            containerNode.children!.push(childContainerNode);
            processedContainers.add(childId);
          } else {
            // Add child node
            const childNode = this.state.visibleNodes.find((n: any) => n.id === childId);
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
        if (containerNode.children!.length > 0) {
          const labelNode: any = {
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
    this.state.visibleContainers.forEach((container: any) => {
      // Check if this container has a parent that's also visible
      const hasVisibleParent = this.state.visibleContainers.some((otherContainer: any) => {
        const children = this.state._collections.containerChildren.get(otherContainer.id);
        return children && children.has(container.id);
      });
      
      if (!hasVisibleParent && !processedContainers.has(container.id)) {
        const containerNode = buildContainerHierarchy(container.id);
        rootNodes.push(containerNode);
        processedContainers.add(container.id);
      }
    });
    
    // Add any uncontained nodes at root level
    this.state.visibleNodes.forEach((node: any) => {
      if (!processedNodes.has(node.id)) {
        rootNodes.push({
          id: node.id,
          width: node.width || 180,
          height: node.height || 60
        });
      }
    });
    
    // Convert edges - ELK will handle hierarchy automatically
    const allEdges: any[] = Array.from(this.state.visibleEdges).map((edge: any) => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target]
    }));
    
    return {
      id: 'root',
      children: rootNodes,
      edges: allEdges,
      layoutOptions: getELKLayoutOptions('layered')
    };
  }

  /**
   * Apply ELK layout results back to VisualizationState
   * This method handles all the business logic that was previously in ELKBridge
   */
  applyELKLayout(elkResult: any): void {
    if (!elkResult.children) {
      console.warn('[BridgeCompatibility] No children in ELK result');
      return;
    }
    
    // Apply positions to containers and nodes using ELK coordinates directly
    elkResult.children.forEach((elkNode: any) => {
      // Check if this ID exists as a container in VisualizationState first
      try {
        const container = this.state._collections.containers.get(elkNode.id);
        if (container) {
          this.updateContainerFromELK(elkNode);
          return;
        }
      } catch (e) {
        // Not a container, continue to node logic
      }
      
      // Handle as node or container based on ELK structure
      if (elkNode.children && elkNode.children.length > 0) {
        this.updateContainerFromELK(elkNode);
      } else {
        this.updateNodeFromELK(elkNode);
      }
    });
  }

  /**
   * Update container from ELK result
   */
  private updateContainerFromELK(elkNode: any): void {
    const layoutUpdates: any = {};
    
    // Validate and set position
    if (elkNode.x !== undefined || elkNode.y !== undefined) {
      layoutUpdates.position = {};
      
      if (elkNode.x !== undefined) {
        if (typeof elkNode.x === 'number' && !isNaN(elkNode.x) && isFinite(elkNode.x)) {
          layoutUpdates.position.x = elkNode.x;
        } else {
          console.error(`[BridgeCompatibility] Invalid x coordinate for container ${elkNode.id}: ${elkNode.x}`);
          layoutUpdates.position.x = 0;
        }
      }
      
      if (elkNode.y !== undefined) {
        if (typeof elkNode.y === 'number' && !isNaN(elkNode.y) && isFinite(elkNode.y)) {
          layoutUpdates.position.y = elkNode.y;
        } else {
          console.error(`[BridgeCompatibility] Invalid y coordinate for container ${elkNode.id}: ${elkNode.y}`);
          layoutUpdates.position.y = 0;
        }
      }
    } else {
      console.error(`[BridgeCompatibility] ELK provided no position coordinates for container ${elkNode.id}`);
      layoutUpdates.position = { x: 0, y: 0 };
    }
    
    // Validate and set dimensions
    if (elkNode.width !== undefined || elkNode.height !== undefined) {
      layoutUpdates.dimensions = {};
      
      if (elkNode.width !== undefined) {
        if (typeof elkNode.width === 'number' && !isNaN(elkNode.width) && isFinite(elkNode.width) && elkNode.width > 0) {
          layoutUpdates.dimensions.width = elkNode.width;
        } else {
          console.error(`[BridgeCompatibility] Invalid width for container ${elkNode.id}: ${elkNode.width}`);
          layoutUpdates.dimensions.width = 200;
        }
      }
      
      if (elkNode.height !== undefined) {
        if (typeof elkNode.height === 'number' && !isNaN(elkNode.height) && isFinite(elkNode.height) && elkNode.height > 0) {
          layoutUpdates.dimensions.height = elkNode.height;
        } else {
          console.error(`[BridgeCompatibility] Invalid height for container ${elkNode.id}: ${elkNode.height}`);
          layoutUpdates.dimensions.height = 150;
        }
      }
    }
    
    if (Object.keys(layoutUpdates).length > 0) {
      this.state.setContainerLayout(elkNode.id, layoutUpdates);
    }
    
    // Update child positions recursively
    elkNode.children?.forEach((elkChildNode: any) => {
      // Handle label nodes
      if (elkChildNode.id.endsWith('_label')) {
        const containerId = elkChildNode.id.replace('_label', '');
        const container = this.state._collections.containers.get(containerId);
        
        if (container) {
          const containerLayout = this.state.getContainerLayout(containerId) || { 
            position: { x: container.x || 0, y: container.y || 0 },
            dimensions: { width: container.width || 200, height: container.height || 150 }
          };
          
          this.state.setContainerLayout(containerId, {
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
        this.updateContainerFromELK(elkChildNode);
      } else {
        this.updateNodeFromELK(elkChildNode);
      }
    });
  }

  /**
   * Update node from ELK result
   */
  private updateNodeFromELK(elkNode: any): void {
    try {
      const layoutUpdates: any = {};
      
      // Validate and set position
      if (elkNode.x !== undefined || elkNode.y !== undefined) {
        layoutUpdates.position = {};
        
        if (elkNode.x !== undefined) {
          if (typeof elkNode.x === 'number' && !isNaN(elkNode.x) && isFinite(elkNode.x)) {
            layoutUpdates.position.x = elkNode.x;
          } else {
            console.error(`[BridgeCompatibility] Invalid x coordinate for node ${elkNode.id}: ${elkNode.x}`);
            layoutUpdates.position.x = 0;
          }
        }
        
        if (elkNode.y !== undefined) {
          if (typeof elkNode.y === 'number' && !isNaN(elkNode.y) && isFinite(elkNode.y)) {
            layoutUpdates.position.y = elkNode.y;
          } else {
            console.error(`[BridgeCompatibility] Invalid y coordinate for node ${elkNode.id}: ${elkNode.y}`);
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
            console.error(`[BridgeCompatibility] Invalid width for node ${elkNode.id}: ${elkNode.width}`);
            layoutUpdates.dimensions.width = 180;
          }
        }
        
        if (elkNode.height !== undefined) {
          if (typeof elkNode.height === 'number' && !isNaN(elkNode.height) && isFinite(elkNode.height) && elkNode.height > 0) {
            layoutUpdates.dimensions.height = elkNode.height;
          } else {
            console.error(`[BridgeCompatibility] Invalid height for node ${elkNode.id}: ${elkNode.height}`);
            layoutUpdates.dimensions.height = 60;
          }
        }
      }
      
      if (Object.keys(layoutUpdates).length > 0) {
        this.state.setNodeLayout(elkNode.id, layoutUpdates);
      }
      return;
    } catch (nodeError) {
      // If not found as node, might be a collapsed container
      try {
        const layoutUpdates: any = {};
        
        if (elkNode.x !== undefined || elkNode.y !== undefined) {
          layoutUpdates.position = {};
          
          if (elkNode.x !== undefined) {
            if (typeof elkNode.x === 'number' && !isNaN(elkNode.x) && isFinite(elkNode.x)) {
              layoutUpdates.position.x = elkNode.x;
            } else {
              console.error(`[BridgeCompatibility] Invalid x coordinate for container ${elkNode.id}: ${elkNode.x}`);
              layoutUpdates.position.x = 0;
            }
          }
          
          if (elkNode.y !== undefined) {
            if (typeof elkNode.y === 'number' && !isNaN(elkNode.y) && isFinite(elkNode.y)) {
              layoutUpdates.position.y = elkNode.y;
            } else {
              console.error(`[BridgeCompatibility] Invalid y coordinate for container ${elkNode.id}: ${elkNode.y}`);
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
              console.error(`[BridgeCompatibility] Invalid width for container ${elkNode.id}: ${elkNode.width}`);
              layoutUpdates.dimensions.width = 200;
            }
          }
          
          if (elkNode.height !== undefined) {
            if (typeof elkNode.height === 'number' && !isNaN(elkNode.height) && isFinite(elkNode.height) && elkNode.height > 0) {
              layoutUpdates.dimensions.height = elkNode.height;
            } else {
              console.error(`[BridgeCompatibility] Invalid height for container ${elkNode.id}: ${elkNode.height}`);
              layoutUpdates.dimensions.height = 150;
            }
          }
        }
        
        if (Object.keys(layoutUpdates).length > 0) {
          this.state.setContainerLayout(elkNode.id, layoutUpdates);
        }
        return;
      } catch (containerError) {
        console.warn(`[BridgeCompatibility] Node/Container ${elkNode.id} not found in VisualizationState`);
      }
    }
  }
}
