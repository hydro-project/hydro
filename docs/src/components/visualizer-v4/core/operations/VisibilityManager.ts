/**
 * Visibility Management - Handles visibility state and cache updates
 * 
 * Centralizes all visibility-related operations including cache management,
 * cascading updates, and consistency checks. Extracted from VisState.ts
 * for better separation of concerns.
 */

export class VisibilityManager {
  private readonly state: any;

  constructor(state: any) {
    this.state = state;
  }

  /**
   * Safely set node visibility with automatic cache updates and edge cascade
   */
  setNodeVisibility(nodeId: string, visible: boolean): void {
    const node = this.state._collections.graphNodes.get(nodeId);
    if (!node) {
      console.warn(`[VisualizationState] Cannot set visibility for non-existent node: ${nodeId}`);
      return;
    }
    
    const wasVisible = !node.hidden;
    node.hidden = !visible;
    
    // Update visibility cache atomically
    if (visible) {
      this.state._collections._visibleNodes.set(nodeId, node);
    } else {
      this.state._collections._visibleNodes.delete(nodeId);
    }
    
    // Cascade visibility to connected edges
    this.cascadeNodeVisibilityToEdges(nodeId, visible);
  }

  /**
   * Safely set edge visibility with endpoint validation
   */
  setEdgeVisibility(edgeId: string, visible: boolean): void {
    const edge = this.state._collections.graphEdges.get(edgeId);
    if (!edge) {
      console.warn(`[VisualizationState] Cannot set visibility for non-existent edge: ${edgeId}`);
      return;
    }
    
    // Validate endpoints are visible before making edge visible
    if (visible) {
      const sourceValid = this.isEndpointVisible(edge.source);
      const targetValid = this.isEndpointVisible(edge.target);
      
      if (!sourceValid || !targetValid) {
        console.warn(`[VisualizationState] Cannot make edge ${edgeId} visible - endpoints not visible`);
        return;
      }
    }
    
    edge.hidden = !visible;
    
    // Update visibility cache
    if (visible) {
      this.state._collections._visibleEdges.set(edgeId, edge);
    } else {
      this.state._collections._visibleEdges.delete(edgeId);
    }
  }

  /**
   * Update container visibility caches
   */
  updateContainerVisibilityCaches(containerId: string, container: any): void {
    console.log(`[VisibilityManager] 🔄 Updating visibility caches for container ${containerId}: collapsed=${container.collapsed}, hidden=${container.hidden}`);
    
    // DIAGNOSTIC: Check for specific problem containers
    if (containerId === 'bt_81' || containerId === 'bt_98') {
      console.log(`[VisibilityManager] 🔍 DIAGNOSTIC: Problem container ${containerId} - hidden: ${container.hidden}, collapsed: ${container.collapsed}`);
      
      // Check if this container has a parent and if that parent is collapsed
      const parentContainerId = this.state._collections.nodeContainers.get(containerId);
      if (parentContainerId) {
        const parentContainer = this.state._collections.containers.get(parentContainerId);
        console.log(`[VisibilityManager] 🔍 DIAGNOSTIC: ${containerId} has parent ${parentContainerId} - parent collapsed: ${parentContainer?.collapsed}, parent hidden: ${parentContainer?.hidden}`);
        if (parentContainer?.collapsed) {
          console.log(`[VisibilityManager] 🚨 PROBLEM: ${containerId} should be hidden because parent ${parentContainerId} is collapsed!`);
        }
      } else {
        console.log(`[VisibilityManager] 🔍 DIAGNOSTIC: ${containerId} has no parent (root level container)`);
      }
    }
    
    // Update _visibleContainers (includes collapsed containers)
    if (!container.hidden) {
      this.state._collections._visibleContainers.set(containerId, container);
      console.log(`[VisibilityManager] ✅ Added ${containerId} to _visibleContainers`);
    } else {
      this.state._collections._visibleContainers.delete(containerId);
      console.log(`[VisibilityManager] ❌ Removed ${containerId} from _visibleContainers (hidden)`);
      if (containerId === 'bt_81' || containerId === 'bt_98') {
        console.log(`[VisibilityManager] 🔍 DIAGNOSTIC: Problem container ${containerId} EXCLUDED because hidden=${container.hidden}`);
      }
    }
    
    // Update _expandedContainers (only non-collapsed containers)
    if (!container.hidden && !container.collapsed) {
      this.state._collections._expandedContainers.set(containerId, container);
      console.log(`[VisibilityManager] ✅ Added ${containerId} to _expandedContainers`);
    } else {
      this.state._collections._expandedContainers.delete(containerId);
      console.log(`[VisibilityManager] ❌ Removed ${containerId} from _expandedContainers (hidden=${container.hidden}, collapsed=${container.collapsed})`);
    }
    
    // Update collapsedContainers
    if (container.collapsed && !container.hidden) {
      this.state._collections.collapsedContainers.set(containerId, container);
      console.log(`[VisibilityManager] ✅ Added ${containerId} to collapsedContainers`);
    } else {
      this.state._collections.collapsedContainers.delete(containerId);
      console.log(`[VisibilityManager] ❌ Removed ${containerId} from collapsedContainers (collapsed=${container.collapsed}, hidden=${container.hidden})`);
    }
  }

  /**
   * Cascade node visibility to connected edges
   */
  private cascadeNodeVisibilityToEdges(nodeId: string, nodeVisible: boolean): void {
    const connectedEdges = this.state._collections.nodeToEdges.get(nodeId) || new Set();
    
    for (const edgeId of Array.from(connectedEdges)) {
      const edge = this.state._collections.graphEdges.get(edgeId);
      if (!edge) continue;
      
      // Edge can only be visible if both endpoints are visible
      const sourceVisible = this.isEndpointVisible(edge.source);
      const targetVisible = this.isEndpointVisible(edge.target);
      const shouldBeVisible = sourceVisible && targetVisible;
      
      this.setEdgeVisibility(edgeId as string, shouldBeVisible);
    }
  }

  /**
   * Cascade container visibility to descendants
   */
  cascadeContainerVisibility(containerId: string, visible: boolean): void {
    if (!visible) {
      // When hiding container, hide all descendants
      this.hideAllDescendants(containerId);
    }
    // Note: When showing container, we don't automatically show descendants
    // They may have been individually hidden
  }

  /**
   * Hide all descendants of a container
   */
  private hideAllDescendants(containerId: string): void {
    const children = this.state._collections.containerChildren.get(containerId) || new Set();
    
    for (const childId of Array.from(children)) {
      // First, recursively hide descendants
      this.hideAllDescendants(childId as string);
      this.hideChild(childId as string);
    }
  }

  /**
   * Hide a specific child (container or node)
   */
  private hideChild(childId: string): void {
    const childContainer = this.state._collections.containers.get(childId);
    if (childContainer) {
      childContainer.collapsed = true;
      childContainer.hidden = true;
      childContainer.x = undefined;
      childContainer.y = undefined;
      if (childContainer.layout) {
        childContainer.layout.position = undefined;
      }
      this.updateContainerVisibilityCaches(childId, childContainer);
    } else {
      const node = this.state._collections.graphNodes.get(childId);
      if (node) {
        node.hidden = true;
        this.state._collections._visibleNodes.delete(childId);
        this.cascadeNodeVisibilityToEdges(childId, false);
      }
    }
  }

  /**
   * Check if an endpoint (node or container) is visible
   */
  private isEndpointVisible(endpointId: string): boolean {
    // Check if it's a visible node
    const node = this.state._collections.graphNodes.get(endpointId);
    if (node) return !node.hidden;
    
    // Check if it's a visible container (collapsed containers are visible)
    const container = this.state._collections.containers.get(endpointId);
    if (container) return !container.hidden;
    
    return false;
  }
}
