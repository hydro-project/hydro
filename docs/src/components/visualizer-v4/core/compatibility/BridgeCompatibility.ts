/**
 * Bridge Compatibility Layer
 * 
 * Provides backwards compatibility methods for existing bridge systems
 * (ReactFlow, ELK, etc.) while the core API evolves. Extracted from VisState.ts
 * for better separation of concerns.
 */

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
}
