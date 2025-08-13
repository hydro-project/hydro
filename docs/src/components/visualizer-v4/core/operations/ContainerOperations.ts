/**
 * Container Operations - Collapse/Expand Logic
 * 
 * Handles all container state transitions including collapse/expand operations,
 * hyperEdge management, and visibility cascading. Extracted from VisState.ts
 * for better separation of concerns.
 */

import { LAYOUT_CONSTANTS, HYPEREDGE_CONSTANTS, SIZES } from '../../shared/config';

export class ContainerOperations {
  private readonly state: any;
  private isExpanding: boolean = false;

  constructor(state: any) {
    this.state = state;
  }

  /**
   * Handle container collapse with full hyperEdge management
   */
  handleContainerCollapse(containerId: string): void {
    console.log(`[DEBUG] Starting collapse for container ${containerId}`);
    
    // CRITICAL: Verify no hyperEdges exist for this container before collapse
    // This ensures expansion cleanup worked correctly
    this.validateNoExistingHyperEdges(containerId);
    
    // Track hyperEdges created during this collapse to detect duplicates
    const createdHyperEdgeIds = new Set<string>();
    
    // 1. Recurse bottom-up
    const children = this.state.getContainerChildren(containerId);
    for (const childId of children) {
      const childContainer = this.state.getContainer(childId);
      if (childContainer && !childContainer.collapsed) {
        this.handleContainerCollapse(childId);
      }
    }

    // Now everything below us is collapsed. Don't mark children hidden until we find crossing edges

    // 2. Find crossing edges
    const crossingEdges = this.getCrossingEdges(containerId);
    console.log(`[COLLAPSE] Found ${crossingEdges.length} crossing edges for container ${containerId}`);

    // 3. Hide children
    for (const childId of children) {
      this.hideChild(childId); 
    }

    // 4. Create hyperEdges to replace crossing edges
    const hyperEdges = this.prepareHyperedges(containerId, crossingEdges);
    console.log(`[COLLAPSE] Creating ${hyperEdges.length} hyperEdges for container ${containerId}`);
    
    for (const hyperEdge of hyperEdges) {
      console.log(`[DEBUG] Creating hyperEdge: ${hyperEdge.id} (${hyperEdge.source} -> ${hyperEdge.target})`);
      
      // Check for duplicates within this collapse operation
      if (createdHyperEdgeIds.has(hyperEdge.id)) {
        throw new Error(`DUPLICATE HYPEREDGE: Attempting to create hyperEdge ${hyperEdge.id} twice during collapse of ${containerId}. This indicates a bug in the hyperEdge generation logic.`);
      }
      
      // Check if hyperEdge already exists globally (should not happen if expansion cleanup worked correctly)
      if (this.state._collections.hyperEdges.has(hyperEdge.id)) {
        throw new Error(`HYPEREDGE ALREADY EXISTS: Attempting to create hyperEdge ${hyperEdge.id} during collapse of ${containerId}, but it already exists. This indicates incomplete cleanup during previous expansion.`);
      }
      
      // Track this hyperEdge locally
      createdHyperEdgeIds.add(hyperEdge.id);
      
      // Create hyperEdge using internal API
      this.state._collections.hyperEdges.set(hyperEdge.id, hyperEdge);
      
      // Update node-to-edges mapping for hyperEdges
      const sourceEdges = this.state._collections.nodeToEdges.get(hyperEdge.source) || new Set();
      sourceEdges.add(hyperEdge.id);
      this.state._collections.nodeToEdges.set(hyperEdge.source, sourceEdges);
      
      const targetEdges = this.state._collections.nodeToEdges.get(hyperEdge.target) || new Set();
      targetEdges.add(hyperEdge.id);
      this.state._collections.nodeToEdges.set(hyperEdge.target, targetEdges);
    }
    
    // 5. Hide the original crossing edges
    console.log(`[DEBUG] About to hide ${crossingEdges.length} crossing edges`);
    for (const edge of crossingEdges) {
      if (edge.id.startsWith(HYPEREDGE_CONSTANTS.PREFIX)) {
        console.log(`[DEBUG] [ENCAPSULATION BREACH] Directly deleting hyperEdge: ${edge.id}`);
        this.state._collections.hyperEdges.delete(edge.id);
      } else {
        console.log(`[DEBUG] Hiding regular edge: ${edge.id}`);
        edge.hidden = true;
        this.state._collections._visibleEdges.delete(edge.id);
      }
    }
    
    // 6. Update existing hyperEdges that now have invalid endpoints
    console.log(`[DEBUG] Updating existing hyperEdges with invalid endpoints due to collapse of ${containerId}`);
    this.updateInvalidatedHyperEdges(containerId);
    
    // Validate hyperEdge endpoints and routing after all updates
    this.validateHyperEdgeEndpoints();
    this.validateHyperEdgeLifting();
  }

  /**
   * Handle container expansion with cleanup (non-recursive)
   * Expands only the specified container, leaving child containers in their current state
   */
  handleContainerExpansion(containerId: string): void {
    console.log(`[EXPANSION] ⭐ Starting expansion of ${containerId}`);
    
    // CRITICAL: Temporarily suppress validation during the expansion process
    // to avoid invariant violations during the transition period
    const wasValidationEnabled = this.state._validationEnabled;
    this.state._validationEnabled = false;
    
    try {
      // STEP 1: Mark the container as expanded
      // INVARIANT VIOLATION: This creates INVALID_HYPEREDGE_ROUTING violations
      // REASON: Existing hyperEdges like "hyper_containerA_to_containerB" now have
      //         containerA as non-collapsed, violating "at least one collapsed endpoint" rule
      // WHY OK: We will fix this in Step 3 by removing these hyperEdges
      const container = this.state.getContainer(containerId);
      if (!container) {
        throw new Error(`Container ${containerId} not found`);
      }
      container.collapsed = false;
      this.state._updateContainerVisibilityCaches(containerId, container);
      
      // STEP 2: Show immediate children (make them visible)
      // INVARIANT VIOLATION: EDGE_TO_HIDDEN_SOURCE/TARGET temporarily violated
      // REASON: When we show nodeA1, any edges to still-hidden nodes (in containerB) 
      //         become "visible edge to hidden node"
      // WHY OK: These edges are currently hidden, but validation is suppressed
      this.showImmediateChildren(containerId);
      
      // STEP 3: Clean up old hyperEdges and restore/create new connections
      // INVARIANT VIOLATION: MISSING_HYPEREDGE temporarily violated
      // REASON: We remove hyperEdges but haven't created replacements yet
      // WHY OK: We immediately restore connections in the cleanup process
      console.log(`[EXPANSION] ⭐ About to call cleanupHyperEdgesForExpansion for ${containerId}`);
      this.cleanupHyperEdgesForExpansion(containerId);
      
    } finally {
      // STEP 4: Re-enable validation - all invariants should now be satisfied
      this.state._validationEnabled = wasValidationEnabled;
    }
    
    console.log(`[EXPANSION] Completed expansion of ${containerId}`);
  }

  /**
   * Recursively expand a container and all its child containers
   * Convenience function for cases where you want full expansion
   */
  handleContainerExpansionRecursive(containerId: string): void {
    console.log(`[EXPANSION_RECURSIVE] Starting recursive expansion of ${containerId}`);
    
    // First expand this container
    this.handleContainerExpansion(containerId);
    
    // Then recursively expand all child containers
    const children = this.state.getContainerChildren(containerId) || new Set();
    for (const childId of Array.from(children)) {
      if (typeof childId === 'string') {
        const childContainer = this.state.getContainer(childId);
        if (childContainer && childContainer.collapsed) {
          this.handleContainerExpansionRecursive(childId);
        }
      }
    }
    
    console.log(`[EXPANSION_RECURSIVE] Completed recursive expansion of ${containerId}`);
  }

  /**
   * Hide a child container or node during collapse
   */
  private hideChild(childId: string): void {
    const childContainer = this.state.getContainer(childId);
    if (childContainer) {
      // mark the child container as collapsed and hidden
      childContainer.collapsed = true; // Must be collapsed
      childContainer.hidden = true;    // Must be hidden

      // CRITICAL: Clear layout positions to prevent stale layout data invariant violations
      childContainer.x = undefined;
      childContainer.y = undefined;
      if (childContainer.layout) {
        childContainer.layout.position = undefined;
      }
      
      // Update visibility caches
      this.state._updateContainerVisibilityCaches(childId, childContainer);
    } else {
      // If it's a node, hide it directly
      const node = this.state.getGraphNode(childId);
      if (node) {
        node.hidden = true;
        this.state._collections._visibleNodes.delete(childId);
        
        // Cascade to connected edges
        this.state._cascadeNodeVisibilityToEdges(childId, false);
      }
    }
  }

  /**
   * Show immediate children during expansion
   */
  private showImmediateChildren(containerId: string): void {
    const children = this.state.getContainerChildren(containerId) || new Set();
    
    for (const childId of Array.from(children)) {
      const childContainer = this.state.getContainer(childId);
      if (childContainer) {
        // Show the container but keep it collapsed initially
        childContainer.hidden = false;
        this.state._updateContainerVisibilityCaches(childId, childContainer);
      } else {
        // Show the node
        const node = this.state.getGraphNode(childId);
        if (node) {
          this.state.setNodeVisibility(childId, true);
        }
      }
    }
  }

  /**
   * Prepare hyperEdges for a collapsed container
   */
  private prepareHyperedges(containerId: string, crossingEdges: any[]): any[] {
    // CRITICAL: Use ALL descendant nodes, not just direct children, to match getCrossingEdges logic
    const allDescendantNodes = new Set(this.getAllDescendantNodes(containerId));
    const edgeGroups = new Map<string, { incoming: any[], outgoing: any[] }>();

    // Group edges by external endpoint (routed to lowest visible ancestor)
    for (const edge of crossingEdges) {
      const sourceInContainer = allDescendantNodes.has(edge.source);
      const rawExternalEndpoint = sourceInContainer ? edge.target : edge.source;
      
      // CRITICAL: Route the external endpoint to its lowest visible ancestor
      const externalEndpoint = this.findLowestVisibleAncestor(rawExternalEndpoint);
      
      const isOutgoing = sourceInContainer; // container -> external

      if (!edgeGroups.has(externalEndpoint)) {
        edgeGroups.set(externalEndpoint, { incoming: [], outgoing: [] });
      }

      const group = edgeGroups.get(externalEndpoint)!;
      if (isOutgoing) {
        group.outgoing.push(edge);
      } else {
        group.incoming.push(edge);
      }
    }

    // Create hyperedge objects
    const hyperEdges: any[] = [];
    
    for (const [externalEndpoint, group] of Array.from(edgeGroups.entries())) {
      // Validate that the external endpoint exists and is visible
      const endpointExists = this.state._collections.graphNodes.has(externalEndpoint) || 
                           this.state._collections.containers.has(externalEndpoint);
      const endpointVisible = this.isNodeOrContainerVisible(externalEndpoint);
      
      if (!endpointExists) {
        console.warn(`[HYPEREDGE] Skipping hyperEdge creation - external endpoint ${externalEndpoint} does not exist`);
        continue;
      }
      
      if (!endpointVisible) {
        console.warn(`[HYPEREDGE] Skipping hyperEdge creation - external endpoint ${externalEndpoint} is not visible`);
        continue;
      }
      
      // Create hyperedge for incoming connections (external -> container)
      if (group.incoming.length > 0) {
        const hyperEdgeId = `${HYPEREDGE_CONSTANTS.PREFIX}${externalEndpoint}${HYPEREDGE_CONSTANTS.SEPARATOR}${containerId}`;
        hyperEdges.push(this.createHyperedgeObject(hyperEdgeId, externalEndpoint, containerId, group.incoming, containerId));
      }

      // Create hyperedge for outgoing connections (container -> external)
      if (group.outgoing.length > 0) {
        const hyperEdgeId = `${HYPEREDGE_CONSTANTS.PREFIX}${containerId}${HYPEREDGE_CONSTANTS.SEPARATOR}${externalEndpoint}`;
        hyperEdges.push(this.createHyperedgeObject(hyperEdgeId, containerId, externalEndpoint, group.outgoing, containerId));
      }
    }

    return hyperEdges;
  }

  /**
   * Create a hyperEdge object from original edges
   */
  private createHyperedgeObject(hyperEdgeId: string, source: string, target: string, originalEdges: any[], collapsedContainerId: string): any {
    // Store original edge information for restoration as aggregatedEdges
    const aggregatedEdges = new Map();
    for (const edge of originalEdges) {
      aggregatedEdges.set(edge.id, {
        source: edge.source,
        target: edge.target
      });
    }

    // Use the highest priority style from the aggregated edges
    const style = this.hyperEdgeStyles(originalEdges);

    return {
      id: hyperEdgeId,
      source,
      target,
      style,
      aggregatedEdges,  // Changed from originalEndpoints to aggregatedEdges
      hidden: false
    };
  }

  /**
   * Aggregate styles from multiple edges (highest priority wins)
   */
  private hyperEdgeStyles(edges: any[]): string {
    // Priority order: ERROR > WARNING > THICK > HIGHLIGHTED > DEFAULT
    const stylePriority: Record<string, number> = {
      'error': 5,
      'warning': 4,
      'thick': 3,
      'highlighted': 2,
      'default': 1
    };
    
    let highestPriority = 0;
    let resultStyle = 'default';
    
    for (const edge of edges) {
      const priority = stylePriority[edge.style] || 1;
      if (priority > highestPriority) {
        highestPriority = priority;
        resultStyle = edge.style;
      }
    }
    
    return resultStyle;
  }

  /**
   * Restore connections when a container is expanded using visible hyperEdges as the source of truth
   */
  private cleanupHyperEdgesForExpansion(containerId: string): void {
    console.log(`[EXPANSION] Restoring connections for expanded container ${containerId}`);
    
    // Find all hyperEdges that correspond to the expanding container
    // This includes hyperEdges that might connect to ancestors/descendants due to remote side changes
    const correspondingHyperEdges = this.findCorrespondingHyperEdges(containerId);
    console.log(`[EXPANSION] Found ${correspondingHyperEdges.length} corresponding hyperEdges to process`);
    
    // Process each corresponding hyperEdge to restore the cached connections
    for (const hyperEdge of correspondingHyperEdges) {
      console.log(`[EXPANSION] Processing hyperEdge ${hyperEdge.id}: ${hyperEdge.source} -> ${hyperEdge.target}`);
      
      // Determine which side is the remote endpoint
      const remote = hyperEdge.source === containerId ? hyperEdge.target : hyperEdge.source;
      const isOutgoing = hyperEdge.source === containerId;
      
      console.log(`[EXPANSION] Remote endpoint: ${remote}, isOutgoing: ${isOutgoing}`);
      
      // Remove the hyperEdge first
      this.removeHyperEdge(hyperEdge.id);
      
      // Restore connections using cached aggregatedEdges
      if (hyperEdge.aggregatedEdges && hyperEdge.aggregatedEdges.size > 0) {
        console.log(`[EXPANSION] Restoring ${hyperEdge.aggregatedEdges.size} original connections`);
        
        for (const [originalEdgeId, endpoints] of hyperEdge.aggregatedEdges) {
          if (isOutgoing) {
            // For outgoing hyperEdges: connect internal source to remote endpoint
            this.createOrRestoreConnection(endpoints.source, remote, hyperEdge.aggregatedEdges, originalEdgeId);
          } else {
            // For incoming hyperEdges: connect remote endpoint to internal target
            this.createOrRestoreConnection(remote, endpoints.target, hyperEdge.aggregatedEdges, originalEdgeId);
          }
        }
      } else {
        console.log(`[EXPANSION] No aggregatedEdges found for hyperEdge ${hyperEdge.id}, creating direct container connection`);
        // Fallback: create a direct connection between the container and remote
        if (isOutgoing) {
          this.createOrRestoreConnection(containerId, remote, new Map(), `direct_${containerId}_${remote}`);
        } else {
          this.createOrRestoreConnection(remote, containerId, new Map(), `direct_${remote}_${containerId}`);
        }
      }
    }
    
    // Also restore any hidden edges that should now be visible
    this.restoreHiddenEdgesForExpansion(containerId);
  }

  /**
   * Find all hyperEdges that correspond to a container being expanded
   * This includes ANY hyperEdge that has the expanding container as an endpoint,
   * because when a container expands, it should no longer be represented in hyperEdges
   */
  private findCorrespondingHyperEdges(containerId: string): any[] {
    const correspondingHyperEdges = [];
    
    console.log(`[DEBUG] Finding corresponding hyperEdges for expanding container: ${containerId}`);
    
    for (const [hyperEdgeId, hyperEdge] of this.state._collections.hyperEdges) {
      if (hyperEdge.hidden) continue;
      
      console.log(`[DEBUG] Checking hyperEdge: ${hyperEdgeId} (${hyperEdge.source} -> ${hyperEdge.target})`);
      
      // ANY hyperEdge that has the expanding container as either source or target
      // should be cleaned up, because the container is no longer collapsed
      if (hyperEdge.source === containerId || hyperEdge.target === containerId) {
        console.log(`[DEBUG] ✅ FOUND corresponding hyperEdge: ${hyperEdgeId}`);
        correspondingHyperEdges.push(hyperEdge);
      } else {
        console.log(`[DEBUG] ❌ Not corresponding: ${hyperEdgeId}`);
      }
    }
    
    console.log(`[DEBUG] Total corresponding hyperEdges found: ${correspondingHyperEdges.length}`);
    return correspondingHyperEdges;
  }

  /**
   * Check if an endpoint corresponds to a container (direct match, ancestor, or descendant)
   */
  private isCorrespondingEndpoint(endpoint: string, containerId: string): boolean {
    // Direct match
    if (endpoint === containerId) {
      return true;
    }
    
    // Check if endpoint is an ancestor of containerId
    if (this.isAncestorOf(endpoint, containerId)) {
      return true;
    }
    
    // Check if endpoint is a descendant of containerId
    if (this.isAncestorOf(containerId, endpoint)) {
      return true;
    }
    
    return false;
  }

  /**
   * Check if ancestorId is an ancestor of descendantId
   */
  private isAncestorOf(ancestorId: string, descendantId: string): boolean {
    let current = descendantId;
    
    while (current) {
      const container = this.state.getContainer(current);
      if (!container || !container.parentId) {
        break;
      }
      
      if (container.parentId === ancestorId) {
        return true;
      }
      
      current = container.parentId;
    }
    
    return false;
  }

  /**
   * Remove a hyperEdge and clean up all references
   */
  private removeHyperEdge(hyperEdgeId: string): void {
    const hyperEdge = this.state._collections.hyperEdges.get(hyperEdgeId);
    if (!hyperEdge) return;
    
    // Remove from collections
    this.state._collections.hyperEdges.delete(hyperEdgeId);
    this.state._collections._visibleEdges.delete(hyperEdgeId);
    
    // Clean up node-to-edges mapping
    this.removeFromNodeToEdges(hyperEdge.source, hyperEdgeId);
    this.removeFromNodeToEdges(hyperEdge.target, hyperEdgeId);
    
    console.log(`[EXPANSION] Removed hyperEdge ${hyperEdgeId}`);
  }

  /**
   * Check if an edge between source and target would cross any collapsed containers
   */
  private edgeCrossesCollapsedContainer(source: string, target: string): boolean {
    // Get all collapsed containers
    for (const [containerId, container] of this.state._collections.containers) {
      if (!container.collapsed) continue;
      
      const allDescendantNodes = new Set(this.getAllDescendantNodes(containerId));
      const sourceInContainer = allDescendantNodes.has(source);
      const targetInContainer = allDescendantNodes.has(target);
      
      // Edge crosses boundary if exactly one endpoint is in container
      if (sourceInContainer !== targetInContainer) {
        return true;
      }
    }
    
    return false;
  }

  /**
   * Create or restore a connection between two entities
   * This handles both regular edges and hyperEdges based on the target type
   */
  private createOrRestoreConnection(source: string, target: string, aggregatedEdges: Map<string, any>, originalEdgeId?: string): void {
    // Check if this is restoring an original edge
    if (originalEdgeId && this.state._collections.graphEdges.has(originalEdgeId)) {
      const originalEdge = this.state._collections.graphEdges.get(originalEdgeId);
      if (originalEdge && originalEdge.hidden) {
        // Before restoring, check if the endpoints are still hidden
        const sourceNode = this.state.getGraphNode(source);
        const targetNode = this.state.getGraphNode(target);
        const sourceIsHidden = sourceNode && sourceNode.hidden;
        const targetIsHidden = targetNode && targetNode.hidden;
        
        if (sourceIsHidden || targetIsHidden) {
          console.log(`[EXPANSION] Cannot restore edge ${originalEdgeId} yet - source hidden: ${sourceIsHidden}, target hidden: ${targetIsHidden}`);
          // Don't restore the edge yet, and don't create a hyperEdge either
          // This edge will be handled when its endpoints become visible
          return;
        } else {
          // Check if the edge crosses any collapsed containers
          const targetContainer = this.state.getContainer(target);
          const sourceContainer = this.state.getContainer(source);
          const targetIsCollapsedContainer = targetContainer && targetContainer.collapsed;
          const sourceIsCollapsedContainer = sourceContainer && sourceContainer.collapsed;
          
          // Check if edge crosses any other collapsed containers
          const crossesCollapsedContainer = this.edgeCrossesCollapsedContainer(source, target);
          
          if (!targetIsCollapsedContainer && !sourceIsCollapsedContainer && !crossesCollapsedContainer) {
            // Safe to restore the original edge
            originalEdge.hidden = false;
            this.state._collections._visibleEdges.set(originalEdgeId, originalEdge);
            console.log(`[EXPANSION] Restored original edge ${originalEdgeId}: ${source} -> ${target}`);
            return;
          }
        }
        
        // If we reach here, we need to create a hyperEdge (either due to hidden endpoints or collapsed containers)
        const targetContainer = this.state.getContainer(target);
        const sourceContainer = this.state.getContainer(source);
        const targetIsCollapsedContainer = targetContainer && targetContainer.collapsed;
        const sourceIsCollapsedContainer = sourceContainer && sourceContainer.collapsed;
        const crossesCollapsedContainer = this.edgeCrossesCollapsedContainer(source, target);
        
        if (targetIsCollapsedContainer || sourceIsCollapsedContainer || crossesCollapsedContainer) {
          // Create a hyperEdge instead of restoring the regular edge
          // Route hidden endpoints to their visible containers
          let hyperSource = source;
          let hyperTarget = target;
          
          // If source is hidden, route to its visible container
          if (sourceIsHidden) {
            hyperSource = this.findLowestVisibleAncestor(source);
          }
          
          // If target is hidden, route to its visible container  
          if (targetIsHidden) {
            hyperTarget = this.findLowestVisibleAncestor(target);
          }
          
          const hyperEdgeId = `${HYPEREDGE_CONSTANTS.PREFIX}${hyperSource}${HYPEREDGE_CONSTANTS.SEPARATOR}${hyperTarget}`;
          
          console.log(`[DEBUG] createOrRestoreConnection attempting to create hyperEdge: ${hyperEdgeId}`);
          console.log(`[DEBUG] HyperEdge already exists: ${this.state._collections.hyperEdges.has(hyperEdgeId)}`);
          
          if (!this.state._collections.hyperEdges.has(hyperEdgeId)) {
            const hyperEdge = {
              id: hyperEdgeId,
              source: hyperSource,
              target: hyperTarget,
              style: 'default',
              aggregatedEdges: new Map([[originalEdgeId, originalEdge]]),
              hidden: false
            };
            
            // Add to collections
            this.state._collections.hyperEdges.set(hyperEdgeId, hyperEdge);
            // Note: hyperEdges should NOT be added to _visibleEdges - they're handled separately in visibleEdges getter
            
            // Update node-to-edges mapping
            const sourceEdges = this.state._collections.nodeToEdges.get(hyperSource) || new Set();
            sourceEdges.add(hyperEdgeId);
            this.state._collections.nodeToEdges.set(hyperSource, sourceEdges);
            
            const targetEdges = this.state._collections.nodeToEdges.get(hyperTarget) || new Set();
            targetEdges.add(hyperEdgeId);
            this.state._collections.nodeToEdges.set(hyperTarget, targetEdges);
            
            console.log(`[EXPANSION] Created hyperEdge ${hyperEdgeId} instead of restoring edge: ${hyperSource} -> ${hyperTarget} (original: ${source} -> ${target})`);
          } else {
            // HyperEdge already exists, aggregate this edge into it
            const existingHyperEdge = this.state._collections.hyperEdges.get(hyperEdgeId);
            if (existingHyperEdge && originalEdgeId && originalEdge) {
              existingHyperEdge.aggregatedEdges.set(originalEdgeId, originalEdge);
              console.log(`[EXPANSION] Aggregated edge ${originalEdgeId} into existing hyperEdge ${hyperEdgeId}`);
            }
          }
          return;
        }
        
        // Safe to restore the original edge
        originalEdge.hidden = false;
        this.state._collections._visibleEdges.set(originalEdgeId, originalEdge);
        console.log(`[EXPANSION] Restored original edge ${originalEdgeId}: ${source} -> ${target}`);
        return;
      }
    }
    
    // Determine if we need to create a hyperEdge or regular connection
    const targetContainer = this.state.getContainer(target);
    const sourceContainer = this.state.getContainer(source);
    
    // Only create hyperEdges if BOTH endpoints are containers and at least one is collapsed
    const targetIsCollapsedContainer = targetContainer && targetContainer.collapsed;
    const sourceIsCollapsedContainer = sourceContainer && sourceContainer.collapsed;
    
    if (targetIsCollapsedContainer || sourceIsCollapsedContainer) {
      // Create a hyperEdge only if connecting to/from collapsed containers
      const hyperEdgeId = `${HYPEREDGE_CONSTANTS.PREFIX}${source}${HYPEREDGE_CONSTANTS.SEPARATOR}${target}`;
      
      if (!this.state._collections.hyperEdges.has(hyperEdgeId)) {
        const hyperEdge = {
          id: hyperEdgeId,
          source,
          target,
          style: 'default',
          aggregatedEdges,
          hidden: false
        };
        
        // Add to collections
        this.state._collections.hyperEdges.set(hyperEdgeId, hyperEdge);
        this.state._collections._visibleEdges.set(hyperEdgeId, hyperEdge);
        
        // Update node-to-edges mapping
        const sourceEdges = this.state._collections.nodeToEdges.get(source) || new Set();
        sourceEdges.add(hyperEdgeId);
        this.state._collections.nodeToEdges.set(source, sourceEdges);
        
        const targetEdges = this.state._collections.nodeToEdges.get(target) || new Set();
        targetEdges.add(hyperEdgeId);
        this.state._collections.nodeToEdges.set(target, targetEdges);
        
        console.log(`[EXPANSION] Created hyperEdge ${hyperEdgeId}: ${source} -> ${target}`);
      }
    } else {
      // Both endpoints are visible nodes/expanded containers - this should be handled by restoreHiddenEdgesForExpansion
      console.log(`[EXPANSION] Skipping direct edge creation for ${source} -> ${target} (should be restored from hidden edges)`);
    }
  }

  /**
   * Restore hidden edges that should now be visible after container expansion
   */
  private restoreHiddenEdgesForExpansion(containerId: string): void {
    const children = this.state._collections.containerChildren.get(containerId) || new Set();
    
    for (const childId of children) {
      const childEdges = this.state._collections.nodeToEdges.get(childId) || new Set();
      
      for (const edgeId of childEdges) {
        const edge = this.state._collections.graphEdges.get(edgeId);
        if (edge && edge.hidden) {
          // Check if this edge should be restored (both endpoints are now visible)
          const sourceVisible = this.isNodeOrContainerVisible(edge.source);
          const targetVisible = this.isNodeOrContainerVisible(edge.target);
          
          if (sourceVisible && targetVisible) {
            // Restore the edge
            edge.hidden = false;
            this.state._collections._visibleEdges.set(edgeId, edge);
            console.log(`[EXPANSION] Restored hidden edge ${edgeId}: ${edge.source} -> ${edge.target}`);
          }
        }
      }
    }
  }

  /**
   * Update existing hyperEdges that now have invalid endpoints due to container collapse
   */
  private updateInvalidatedHyperEdges(newlyCollapsedContainerId: string): void {
    const updatedHyperEdges: Array<{oldId: string, newHyperEdge: any}> = [];
    const toDelete: string[] = [];
    
    console.log(`[HYPEREDGE_LIFTING] Checking existing hyperEdges for invalidation due to collapse of ${newlyCollapsedContainerId}`);
    
    for (const [hyperEdgeId, hyperEdge] of this.state._collections.hyperEdges) {
      if (hyperEdge.hidden) continue;
      
      // Check if either endpoint is now invalid (hidden or doesn't exist)
      const sourceContainer = this.state._collections.containers.get(hyperEdge.source);
      const sourceNode = this.state._collections.graphNodes.get(hyperEdge.source);
      const targetContainer = this.state._collections.containers.get(hyperEdge.target);
      const targetNode = this.state._collections.graphNodes.get(hyperEdge.target);
      
      const sourceExists = sourceContainer || sourceNode;
      const targetExists = targetContainer || targetNode;
      
      if (!sourceExists || !targetExists) {
        console.warn(`[HYPEREDGE_LIFTING] Removing hyperEdge ${hyperEdgeId} - endpoint doesn't exist`);
        toDelete.push(hyperEdgeId);
        continue;
      }
      
      // Check if endpoints are effectively hidden
      const sourceHidden = (sourceContainer?.hidden) || (sourceNode?.hidden) || 
                          (sourceNode && this.isNodeInCollapsedContainer(hyperEdge.source));
      const targetHidden = (targetContainer?.hidden) || (targetNode?.hidden) || 
                          (targetNode && this.isNodeInCollapsedContainer(hyperEdge.target));
      
      let needsUpdate = false;
      let newSource = hyperEdge.source;
      let newTarget = hyperEdge.target;
      
      // If source is hidden/invalid, find its visible ancestor
      if (sourceHidden) {
        const visibleAncestor = this.findLowestVisibleAncestor(hyperEdge.source);
        if (visibleAncestor !== hyperEdge.source) {
          console.log(`[HYPEREDGE_LIFTING] Source ${hyperEdge.source} is hidden, lifting to ancestor ${visibleAncestor}`);
          newSource = visibleAncestor;
          needsUpdate = true;
        }
      }
      
      // If target is hidden/invalid, find its visible ancestor
      if (targetHidden) {
        const visibleAncestor = this.findLowestVisibleAncestor(hyperEdge.target);
        if (visibleAncestor !== hyperEdge.target) {
          console.log(`[HYPEREDGE_LIFTING] Target ${hyperEdge.target} is hidden, lifting to ancestor ${visibleAncestor}`);
          newTarget = visibleAncestor;
          needsUpdate = true;
        }
      }
      
      if (needsUpdate) {
        // Check if the new routing would create a self-loop
        if (newSource === newTarget) {
          console.log(`[HYPEREDGE_LIFTING] Removing hyperEdge ${hyperEdgeId} - would create self-loop`);
          toDelete.push(hyperEdgeId);
          continue;
        }
        
        // Create new hyperEdge with updated endpoints
        const newHyperEdgeId = `${HYPEREDGE_CONSTANTS.PREFIX}${newSource}${HYPEREDGE_CONSTANTS.SEPARATOR}${newTarget}`;
        
        // Check if this hyperEdge already exists
        if (this.state._collections.hyperEdges.has(newHyperEdgeId)) {
          console.log(`[HYPEREDGE_LIFTING] HyperEdge ${newHyperEdgeId} already exists, removing duplicate ${hyperEdgeId}`);
          
          // IMPORTANT: Merge aggregatedEdges from the duplicate into the existing hyperEdge
          // to ensure all original edges are properly tracked
          const existingHyperEdge = this.state._collections.hyperEdges.get(newHyperEdgeId);
          if (existingHyperEdge && hyperEdge.aggregatedEdges) {
            if (!existingHyperEdge.aggregatedEdges) {
              existingHyperEdge.aggregatedEdges = new Map();
            }
            
            // Merge all aggregated edges from the duplicate
            for (const [edgeId, edgeData] of hyperEdge.aggregatedEdges) {
              existingHyperEdge.aggregatedEdges.set(edgeId, edgeData);
            }
            
            console.log(`[HYPEREDGE_LIFTING] Merged ${hyperEdge.aggregatedEdges.size} aggregated edges from ${hyperEdgeId} into ${newHyperEdgeId}`);
          }
          
          toDelete.push(hyperEdgeId);
          continue;
        }
        
        const newHyperEdge = {
          ...hyperEdge,
          id: newHyperEdgeId,
          source: newSource,
          target: newTarget,
          liftedFrom: hyperEdgeId  // Track the original for debugging
        };
        
        updatedHyperEdges.push({ oldId: hyperEdgeId, newHyperEdge });
      }
    }
    
    // Apply all updates atomically
    for (const { oldId, newHyperEdge } of updatedHyperEdges) {
      this.state._collections.hyperEdges.delete(oldId);
      this.state._collections.hyperEdges.set(newHyperEdge.id, newHyperEdge);
      
      // Update node-to-edges mapping
      const sourceEdges = this.state._collections.nodeToEdges.get(newHyperEdge.source) || new Set();
      sourceEdges.delete(oldId);
      sourceEdges.add(newHyperEdge.id);
      this.state._collections.nodeToEdges.set(newHyperEdge.source, sourceEdges);
      
      const targetEdges = this.state._collections.nodeToEdges.get(newHyperEdge.target) || new Set();
      targetEdges.delete(oldId);
      targetEdges.add(newHyperEdge.id);
      this.state._collections.nodeToEdges.set(newHyperEdge.target, targetEdges);
    }
    
    // Delete hyperEdges that couldn't be lifted
    for (const hyperEdgeId of toDelete) {
      this.removeHyperEdge(hyperEdgeId);
    }
  }

  // Helper methods
  private findLowestVisibleAncestor(entityId: string): string {
    // First check if the entity itself is visible
    if (this.isNodeOrContainerVisible(entityId)) {
      return entityId;
    }

    // If it's a node, find its visible ancestor container
    const nodeContainer = this.state.getNodeContainer(entityId);
    if (nodeContainer) {
      // Recursively find the lowest visible ancestor of the container
      return this.findLowestVisibleAncestor(nodeContainer);
    }

    // If it's a container, find its visible ancestor
    const container = this.state.getContainer(entityId);
    if (container && container.parentId) {
      return this.findLowestVisibleAncestor(container.parentId);
    }

    // If no visible ancestor found, return the entity itself
    // This shouldn't happen in well-formed data, but prevents infinite loops
    return entityId;
  }

  private isNodeInCollapsedContainer(nodeId: string): boolean {
    const parentContainerId = this.state.getNodeContainer(nodeId);
    if (!parentContainerId) return false;
    
    const parentContainer = this.state._collections.containers.get(parentContainerId);
    return parentContainer?.collapsed === true;
  }

  private isNodeOrContainerVisible(entityId: string): boolean {
    if (this.state._collections._visibleNodes.has(entityId)) return true;
    if (this.state._collections._visibleContainers.has(entityId)) return true;
    return false;
  }

  private removeFromNodeToEdges(nodeId: string, edgeId: string): void {
    const edges = this.state._collections.nodeToEdges.get(nodeId);
    if (edges) {
      edges.delete(edgeId);
      if (edges.size === 0) {
        this.state._collections.nodeToEdges.delete(nodeId);
      }
    }
  }

  private validateHyperEdgeEndpoints(): void {
    // Check all hyperEdges have visible endpoints
    for (const [hyperEdgeId, hyperEdge] of this.state._collections.hyperEdges) {
      const sourceVisible = this.isNodeOrContainerVisible(hyperEdge.source);
      const targetVisible = this.isNodeOrContainerVisible(hyperEdge.target);
      
      if (!sourceVisible || !targetVisible) {
        console.warn(`[CONTAINER_OPS] HyperEdge ${hyperEdgeId} has invalid endpoints: source ${hyperEdge.source} (visible: ${sourceVisible}), target ${hyperEdge.target} (visible: ${targetVisible})`);
      }
    }
  }

  private validateHyperEdgeLifting(): void {
    // Check that hyperEdges only exist between collapsed containers or from nodes to collapsed containers
    const invalidHyperEdges = [];
    
    for (const [hyperEdgeId, hyperEdge] of this.state._collections.hyperEdges) {
      const sourceContainer = this.state.getContainer(hyperEdge.source);
      const targetContainer = this.state.getContainer(hyperEdge.target);
      
      const sourceIsCollapsedContainer = sourceContainer?.collapsed === true;
      const targetIsCollapsedContainer = targetContainer?.collapsed === true;
      
      if (!sourceIsCollapsedContainer && !targetIsCollapsedContainer) {
        console.warn(`[CONTAINER_OPS] HyperEdge ${hyperEdgeId} exists but neither endpoint is a collapsed container: source ${hyperEdge.source}, target ${hyperEdge.target} - REMOVING`);
        invalidHyperEdges.push(hyperEdgeId);
      }
    }
    
    // Remove all invalid hyperEdges
    for (const hyperEdgeId of invalidHyperEdges) {
      this.removeHyperEdge(hyperEdgeId);
    }
    
    if (invalidHyperEdges.length > 0) {
      console.log(`[CONTAINER_OPS] Cleaned up ${invalidHyperEdges.length} invalid hyperEdges`);
    }
  }

  /**
   * Get crossing edges for a container
   * Core operation needed for container collapse/expand functionality
   */
  getCrossingEdges(containerId: string): any[] {
    const allDescendantNodes = new Set(this.getAllDescendantNodes(containerId));
    const crossingEdges: any[] = [];

    // Check ALL regular edges (both visible and hidden)
    // CRITICAL: Hidden edges might still represent connectivity that needs hyperEdges
    for (const [edgeId, edge] of this.state._collections.graphEdges) {
      const sourceInContainer = allDescendantNodes.has(edge.source);
      const targetInContainer = allDescendantNodes.has(edge.target);

      // Edge crosses boundary if exactly one endpoint is in container
      if (sourceInContainer !== targetInContainer) {
        crossingEdges.push(edge);
      }
    }

    // Check visible hyperedges
    for (const [hyperEdgeId, hyperEdge] of this.state._collections.hyperEdges) {
      if (hyperEdge.hidden) continue;

      const sourceInContainer = allDescendantNodes.has(hyperEdge.source);
      const targetInContainer = allDescendantNodes.has(hyperEdge.target);

      const sourceIsContainer = this.state._collections.containers.has(hyperEdge.source);
      const targetIsContainer = this.state._collections.containers.has(hyperEdge.target);
      
      if (sourceInContainer !== targetInContainer) {
        // Only consider hyperEdge as crossing if one endpoint is a descendant node (not the container itself)
        // This prevents hyperEdges connecting to other collapsed containers from being incorrectly deleted
        if (!(sourceIsContainer && hyperEdge.source !== containerId) && 
            !(targetIsContainer && hyperEdge.target !== containerId)) {
          crossingEdges.push(hyperEdge);
        }
      }
    }

    return crossingEdges;
  }

  /**
   * Get all descendant nodes for a container
   */
  getAllDescendantNodes(containerId: string): string[] {
    const descendants: string[] = [];
    const children = this.state._collections.containerChildren.get(containerId) || new Set();
    
    for (const childId of Array.from(children)) {
      const childContainer = this.state._collections.containers.get(childId);
      if (childContainer) {
        // Child is a container - recursively get its descendants
        descendants.push(...this.getAllDescendantNodes(childId as string));
      } else {
        // Child is a node
        descendants.push(childId as string);
      }
    }
    
    return descendants;
  }

  /**
   * Clean up all invalid hyperEdges - ensures no hyperEdges exist without a collapsed container endpoint
   * This fixes the INVALID_HYPEREDGE_ROUTING validation error
   */
  cleanupInvalidHyperEdges(): void {
    console.log(`[HYPEREDGE_CLEANUP] Starting cleanup of invalid hyperEdges`);
    
    const hyperEdgesToRemove: string[] = [];
    
    for (const [hyperEdgeId, hyperEdge] of this.state._collections.hyperEdges) {
      if (hyperEdge.hidden) continue;
      
      // Check if either endpoint is a collapsed container
      const sourceContainer = this.state.getContainer(hyperEdge.source);
      const targetContainer = this.state.getContainer(hyperEdge.target);
      
      const sourceIsCollapsedContainer = sourceContainer && sourceContainer.collapsed && !sourceContainer.hidden;
      const targetIsCollapsedContainer = targetContainer && targetContainer.collapsed && !targetContainer.hidden;
      
      // If neither endpoint is a collapsed container, this hyperEdge is invalid
      if (!sourceIsCollapsedContainer && !targetIsCollapsedContainer) {
        console.log(`[HYPEREDGE_CLEANUP] Removing invalid hyperEdge ${hyperEdgeId}: source=${hyperEdge.source} (collapsed=${sourceIsCollapsedContainer}), target=${hyperEdge.target} (collapsed=${targetIsCollapsedContainer})`);
        hyperEdgesToRemove.push(hyperEdgeId);
      }
    }
    
    // Remove invalid hyperEdges
    for (const hyperEdgeId of hyperEdgesToRemove) {
      this.removeHyperEdge(hyperEdgeId);
    }
    
    console.log(`[HYPEREDGE_CLEANUP] Cleaned up ${hyperEdgesToRemove.length} invalid hyperEdges`);
  }

  /**
   * Validate that no hyperEdges exist for a container before collapse
   * This ensures that expansion cleanup worked correctly
   */
  private validateNoExistingHyperEdges(containerId: string): void {
    const existingHyperEdges = [];
    
    for (const [hyperEdgeId, hyperEdge] of this.state._collections.hyperEdges) {
      if (hyperEdge.hidden) continue;
      
      // Check if this hyperEdge has the container as either endpoint
      if (hyperEdge.source === containerId || hyperEdge.target === containerId) {
        existingHyperEdges.push(hyperEdgeId);
      }
    }
    
    if (existingHyperEdges.length > 0) {
      console.log(`[VALIDATION] ❌ EXPANSION CLEANUP FAILURE for ${containerId}:`);
      console.log(`[VALIDATION] Found ${existingHyperEdges.length} existing hyperEdges: ${existingHyperEdges.join(', ')}`);
      console.log(`[VALIDATION] Total hyperEdges in system: ${this.state._collections.hyperEdges.size}`);
      
      // Log details of each existing hyperEdge
      for (const hyperEdgeId of existingHyperEdges) {
        const hyperEdge = this.state._collections.hyperEdges.get(hyperEdgeId);
        console.log(`[VALIDATION] HyperEdge ${hyperEdgeId}: ${hyperEdge.source} -> ${hyperEdge.target}, hidden: ${hyperEdge.hidden}`);
      }
      
      throw new Error(`EXPANSION CLEANUP FAILURE: Container ${containerId} is being collapsed but ${existingHyperEdges.length} hyperEdges still exist: ${existingHyperEdges.join(', ')}. This indicates incomplete cleanup during previous expansion.`);
    }
  }
}
