/**
 * Visualization State Invari  validateInvariants(): ValidationResult {
    const errors: ValidationError[] = [];
    const warnings: ValidationError[] = []; Validation System
 * 
 * Centralizes all validation logic for VisualizationState to ensure consistency
 * and catch bugs early. Extracted from VisState.ts for better separation of concerns.
 */

import { LAYOUT_CONSTANTS, HYPEREDGE_CONSTANTS } from '../../shared/config';

export interface InvariantViolation {
  type: string;
  message: string;
  entityId?: string;
  severity: 'error' | 'warning';
}

/**
 * Centralized VisualizationState Invariant Validation
 * 
 * All public VisualizationState APIs should call validateInvariants() before returning
 * to ensure the state remains consistent and catch bugs early.
 */
export class VisualizationStateInvariantValidator {
  private readonly state: any;

  constructor(state: any) {
    this.state = state;
  }

  /**
   * Validate all VisualizationState invariants
   * Throws an error if any critical invariants are violated
   */
  validateInvariants(): void {
    const violations: InvariantViolation[] = [];

    // Container State Invariants
    violations.push(...this.validateContainerStates());
    violations.push(...this.validateContainerHierarchy());
    
    // Node State Invariants  
    violations.push(...this.validateNodeContainerRelationships());
    violations.push(...this.validateOrphanedNodes());
    
    // Edge and Hyperedge Invariants
    violations.push(...this.validateEdgeNodeConsistency());
    violations.push(...this.validateHyperedgeValidity());
    violations.push(...this.validateDanglingHyperedges());
    violations.push(...this.validateNoEdgesToInvalidOrHiddenContainers());
    violations.push(...this.validateHyperEdgeRouting());
    
    // Layout Invariants
    violations.push(...this.validateCollapsedContainerDimensions());
    violations.push(...this.validatePositionedContainerConsistency());

    // Report violations
    this.reportViolations(violations);
  }

  private reportViolations(violations: InvariantViolation[]): void {
    const errors = violations.filter(v => v.severity === 'error');
    const warnings = violations.filter(v => v.severity === 'warning');

    if (warnings.length > 0) {
      this.reportWarnings(warnings);
    }

    if (errors.length > 0) {
      console.error(`[VisState] CRITICAL: Invariant violations (${errors.length}):`, errors);
      throw new Error(`VisualizationState invariant violations detected: ${errors.map(e => e.message).join('; ')}`);
    }
  }

  private reportWarnings(warnings: InvariantViolation[]): void {
    // Group warnings by type to reduce console noise
    const warningsByType = warnings.reduce((acc, warning) => {
      acc[warning.type] = (acc[warning.type] || 0) + 1;
      return acc;
    }, {} as Record<string, number>);
    
    // Special handling for smart collapse warnings that are expected
    const hyperEdgeWarnings = warningsByType['HYPEREDGE_TO_HIDDEN_CONTAINER'] || 0;
    if (hyperEdgeWarnings > 0) {
      // ALWAYS suppress individual HYPEREDGE_TO_HIDDEN_CONTAINER warnings during smart collapse
      // These are expected behavior and create console noise
      if (hyperEdgeWarnings > 10) {
        console.warn(`[VisState] Smart collapse in progress: ${hyperEdgeWarnings} hyperEdge routing adjustments (normal)`);
      }
      // Remove from individual logging regardless of count
      delete warningsByType['HYPEREDGE_TO_HIDDEN_CONTAINER'];
    }
    
    // Special handling for container dimension warnings during collapse transitions
    const dimensionWarnings = warningsByType['COLLAPSED_CONTAINER_LARGE_DIMENSIONS'] || 0;
    if (dimensionWarnings > 0) {
      // Suppress dimension warnings during active collapse operations
      // These are expected during layout transitions
      delete warningsByType['COLLAPSED_CONTAINER_LARGE_DIMENSIONS'];
    }
    
    // Log other warnings normally (excluding suppressed types)
    const otherWarnings = warnings.filter(w => 
      w.type !== 'HYPEREDGE_TO_HIDDEN_CONTAINER' && 
      w.type !== 'COLLAPSED_CONTAINER_LARGE_DIMENSIONS'
    );
    if (otherWarnings.length > 0) {
      console.warn(`[VisState] Invariant warnings (${otherWarnings.length}):`, otherWarnings);
    }
    
    // Show summary only for non-hyperEdge warnings
    if (Object.keys(warningsByType).length > 0) {
      console.warn(`[VisState] Warning summary:`, warningsByType);
    }
  }

  // ============ Container State Invariants ============

  private validateContainerStates(): InvariantViolation[] {
    const violations: InvariantViolation[] = [];

    for (const [containerId, container] of this.state.containers) {
      const { collapsed, hidden } = container;
      
      // Check for illegal Expanded/Hidden state
      if (!collapsed && hidden) {
        violations.push({
          type: 'ILLEGAL_CONTAINER_STATE',
          message: `Container ${containerId} is in illegal Expanded/Hidden state (collapsed: false, hidden: true)`,
          entityId: containerId,
          severity: 'error'
        });
      }
    }

    return violations;
  }

  private validateContainerHierarchy(): InvariantViolation[] {
    const violations: InvariantViolation[] = [];

    for (const [containerId, container] of this.state.containers) {
      // If container is collapsed, all descendants must be collapsed/hidden
      if (container.collapsed) {
        this.validateDescendantsCollapsed(containerId, violations);
      }
      
      // If container is visible, all ancestors must be visible
      if (!container.hidden) {
        this.validateAncestorsVisible(containerId, violations);
      }
    }

    return violations;
  }

  private validateDescendantsCollapsed(containerId: string, violations: InvariantViolation[]): void {
    const children = this.state.getContainerChildren(containerId);
    
    for (const childId of children) {
      const childContainer = this.state.getContainer(childId);
      if (childContainer) {
        // Child container must be collapsed and hidden
        if (!childContainer.collapsed){
          violations.push({
            type: 'DESCENDANT_NOT_COLLAPSED',
            message: `Container ${childId} should be collapsed/hidden because ancestor ${containerId} is collapsed`,
            entityId: childId,
            severity: 'error'
          });
        }
        if (!childContainer.hidden) {
          violations.push({
            type: 'DESCENDANT_NOT_HIDDEN',
            message: `Container ${childId} should be collapsed/hidden because ancestor ${containerId} is collapsed`,
            entityId: childId,
            severity: 'error'
          });
        } 
        // Recursively check descendants
        this.validateDescendantsCollapsed(childId, violations);
      } else {
        // Child node must be hidden
        const childNode = this.state.getGraphNode(childId);
        if (childNode && !childNode.hidden) {
          violations.push({
            type: 'DESCENDANT_NODE_NOT_HIDDEN',
            message: `Node ${childId} should be hidden because container ${containerId} is collapsed`,
            entityId: childId,
            severity: 'error'
          });
        }
      }
    }
  }

  private validateAncestorsVisible(containerId: string, violations: InvariantViolation[]): void {
    let current = this.state.getNodeContainer(containerId);
    
    while (current) {
      const ancestorContainer = this.state.getContainer(current);
      if (ancestorContainer && ancestorContainer.hidden) {
        violations.push({
          type: 'ANCESTOR_NOT_VISIBLE',
          message: `Container ${containerId} is visible but ancestor ${current} is hidden`,
          entityId: containerId,
          severity: 'error'
        });
      }
      current = this.state.getNodeContainer(current);
    }
  }

  // ============ Node State Invariants ============

  private validateNodeContainerRelationships(): InvariantViolation[] {
    const violations: InvariantViolation[] = [];

    for (const [nodeId, node] of this.state.graphNodes) {
      const containerName = this.state.getNodeContainer(nodeId);
      
      if (containerName) {
        const container = this.state.getContainer(containerName);
        
        // If node belongs to collapsed container, node must be hidden
        if (container && container.collapsed && !node.hidden) {
          violations.push({
            type: 'NODE_NOT_HIDDEN_IN_COLLAPSED_CONTAINER',
            message: `Node ${nodeId} should be hidden because it belongs to collapsed container ${containerName}`,
            entityId: nodeId,
            severity: 'error'
          });
        }
      }
    }

    return violations;
  }

  private validateOrphanedNodes(): InvariantViolation[] {
    const violations: InvariantViolation[] = [];
    // This is informational - orphaned nodes are allowed as root-level nodes
    return violations;
  }

  // ============ Edge and Hyperedge Invariants ============

  /**
   * Validate that edges don't reference hidden entities
   * Note: Edges to VISIBLE collapsed containers are perfectly valid!
   */
  private validateNoEdgesToInvalidOrHiddenContainers(): InvariantViolation[] {
    const violations: InvariantViolation[] = [];

    // Check all non-hidden edges for references to non-existent or hidden entities
    for (const edge of this.state.graphEdges.values()) {
      // Skip hidden edges - they're allowed to reference anything
      if (edge.hidden) continue;
      // Skip hyperEdges - they have different rules
      if (edge.id.startsWith(HYPEREDGE_CONSTANTS.PREFIX)) continue;

      // Check if edge references containers that are hidden (not just collapsed)
      const sourceContainer = this.state.getContainer(edge.source);
      const targetContainer = this.state.getContainer(edge.target);
      const sourceNode = this.state.getGraphNode(edge.source);
      const targetNode = this.state.getGraphNode(edge.target);

      // Source validation: must exist and be visible
      const sourceExists = sourceContainer || sourceNode;
      const sourceHidden = (sourceContainer?.hidden) || (sourceNode?.hidden);
      
      if (!sourceExists) {
        violations.push({
          type: 'EDGE_TO_NONEXISTENT_SOURCE',
          message: `Edge ${edge.id} references non-existent source ${edge.source}`,
          entityId: edge.id,
          severity: 'error'
        });
      } else if (sourceHidden) {
        violations.push({
          type: 'EDGE_TO_HIDDEN_SOURCE',
          message: `Visible edge ${edge.id} references hidden source ${edge.source}`,
          entityId: edge.id,
          severity: 'error'
        });
      }

      // Target validation: must exist and be visible  
      const targetExists = targetContainer || targetNode;
      const targetHidden = (targetContainer?.hidden) || (targetNode?.hidden);
      
      if (!targetExists) {
        violations.push({
          type: 'EDGE_TO_NONEXISTENT_TARGET',
          message: `Edge ${edge.id} references non-existent target ${edge.target}`,
          entityId: edge.id,
          severity: 'error'
        });
      } else if (targetHidden) {
        violations.push({
          type: 'EDGE_TO_HIDDEN_TARGET',
          message: `Visible edge ${edge.id} references hidden target ${edge.target}`,
          entityId: edge.id,
          severity: 'error'
        });
      }
    }

    return violations;
  }

  private validateEdgeNodeConsistency(): InvariantViolation[] {
    const violations: InvariantViolation[] = [];

    for (const [edgeId, edge] of this.state.graphEdges) {
      // Check source exists
      const sourceExists = this.state.getGraphNode(edge.source) || this.state.getContainer(edge.source);
      if (!sourceExists) {
        violations.push({
          type: 'EDGE_INVALID_SOURCE',
          message: `Edge ${edgeId} references non-existent source ${edge.source}`,
          entityId: edgeId,
          severity: 'error'
        });
      }

      // Check target exists  
      const targetExists = this.state.getGraphNode(edge.target) || this.state.getContainer(edge.target);
      if (!targetExists) {
        violations.push({
          type: 'EDGE_INVALID_TARGET',
          message: `Edge ${edgeId} references non-existent target ${edge.target}`,
          entityId: edgeId,
          severity: 'error'
        });
      }
    }

    return violations;
  }

  public validateHyperedgeValidity(): InvariantViolation[] {
    const violations: InvariantViolation[] = [];

    for (const [hyperEdgeId, hyperEdge] of this.state.hyperEdges) {
      if (hyperEdge.hidden) continue;

      // Check that both endpoints exist
      const sourceContainer = this.state.getContainer(hyperEdge.source);
      const targetContainer = this.state.getContainer(hyperEdge.target);
      const sourceNode = this.state.getGraphNode(hyperEdge.source);
      const targetNode = this.state.getGraphNode(hyperEdge.target);
      
      const sourceExists = sourceContainer || sourceNode;
      const targetExists = targetContainer || targetNode;
      
      if (!sourceExists || !targetExists) {
        violations.push({
          type: 'INVALID_HYPEREDGE',
          message: `Hyperedge ${hyperEdgeId} has non-existent endpoints (source exists: ${!!sourceExists}, target exists: ${!!targetExists})`,
          entityId: hyperEdgeId,
          severity: 'error'
        });
        continue;
      }
      
      // Check if endpoints are effectively visible
      const sourceVisible = this._isEntityVisible(hyperEdge.source, sourceContainer, sourceNode);
      const targetVisible = this._isEntityVisible(hyperEdge.target, targetContainer, targetNode);
      
      if (!sourceVisible || !targetVisible) {
        violations.push({
          type: 'HYPEREDGE_TO_HIDDEN_ENDPOINT',
          message: `Hyperedge ${hyperEdgeId} connects to hidden endpoint(s) (source visible: ${sourceVisible}, target visible: ${targetVisible})`,
          entityId: hyperEdgeId,
          severity: 'warning' // This can be expected during transitions
        });
      }

      // CRITICAL: HyperEdges should have at least one collapsed container endpoint
      const sourceIsCollapsedContainer = sourceContainer && sourceContainer.collapsed && !sourceContainer.hidden;
      const targetIsCollapsedContainer = targetContainer && targetContainer.collapsed && !targetContainer.hidden;
      
      if (!sourceIsCollapsedContainer && !targetIsCollapsedContainer) {
        violations.push({
          type: 'INVALID_HYPEREDGE_ROUTING',
          message: `Hyperedge ${hyperEdgeId} exists but neither endpoint is a collapsed container`,
          entityId: hyperEdgeId,
          severity: 'error' // This is a fundamental hyperEdge requirement
        });
      }
    }

    return violations;
  }

  /**
   * Validate that hyperEdges with both endpoints hidden are properly cleaned up
   * Made public for testing
   */
  public validateDanglingHyperedges(): InvariantViolation[] {
    const violations: InvariantViolation[] = [];

    for (const [hyperEdgeId, hyperEdge] of this.state.hyperEdges) {
      if (hyperEdge.hidden) continue;

      // Check if endpoints exist and their visibility state
      const sourceContainer = this.state.getContainer(hyperEdge.source);
      const targetContainer = this.state.getContainer(hyperEdge.target);
      const sourceNode = this.state.getGraphNode(hyperEdge.source);
      const targetNode = this.state.getGraphNode(hyperEdge.target);

      // An endpoint is invalid if it doesn't exist OR if it exists but is hidden
      const sourceExists = sourceContainer !== undefined || sourceNode !== undefined;
      const targetExists = targetContainer !== undefined || targetNode !== undefined;
      const sourceHidden = sourceExists && ((sourceContainer && sourceContainer.hidden) || (sourceNode && sourceNode.hidden));
      const targetHidden = targetExists && ((targetContainer && targetContainer.hidden) || (targetNode && targetNode.hidden));

      const sourceInvalid = !sourceExists || sourceHidden;
      const targetInvalid = !targetExists || targetHidden;

      // Report violation if either endpoint is invalid
      if (sourceInvalid || targetInvalid) {
        let reason = '';
        if (!sourceExists && !targetExists) {
          reason = 'both endpoints don\'t exist';
        } else if (!sourceExists) {
          reason = 'source doesn\'t exist';
        } else if (!targetExists) {
          reason = 'target doesn\'t exist';
        } else if (sourceHidden && targetHidden) {
          reason = 'both endpoints are hidden';
        } else if (sourceHidden) {
          reason = 'source is hidden';
        } else if (targetHidden) {
          reason = 'target is hidden';
        }

        violations.push({
          type: 'DANGLING_HYPEREDGE',
          message: `Hyperedge ${hyperEdgeId} should be hidden because ${reason}`,
          entityId: hyperEdgeId,
          severity: 'warning'
        });
      }
    }

    return violations;
  }

  /**
   * Validate that collapsed containers have proper hyperEdge routing
   */
  public validateHyperEdgeRouting(): InvariantViolation[] {
    const violations: InvariantViolation[] = [];

    // Check that when containers are collapsed, edges through them are properly
    // converted to hyperEdges and original edges are hidden
    for (const [containerId, container] of this.state.containers) {
      if (!container.collapsed) continue;
      
      const crossingEdges = this.state.getCrossingEdges(containerId);
      
      for (const crossingEdge of crossingEdges) {
        // CRITICAL: Only regular edges crossing collapsed containers should be hidden
        // HyperEdges are ALLOWED to cross collapsed containers - that's their purpose!
        const isHyperEdge = crossingEdge.id.startsWith(HYPEREDGE_CONSTANTS.PREFIX);
        
        if (!isHyperEdge && !crossingEdge.hidden) {
          violations.push({
            type: 'CROSSING_EDGE_NOT_HIDDEN',
            message: `Regular edge ${crossingEdge.id} crosses collapsed container ${containerId} but is not hidden`,
            entityId: crossingEdge.id,
            severity: 'error'
          });
        }
        
        // IMPROVED: Check if there's any hyperEdge that represents this connectivity
        // by looking at aggregatedEdges property which tracks original edges
        const representingHyperEdges = Array.from(this.state.hyperEdges.values()).filter((hyperEdge: any) => {
          // First check if this hyperEdge has aggregatedEdges tracking
          if (hyperEdge.aggregatedEdges && hyperEdge.aggregatedEdges.has(crossingEdge.id)) {
            return !hyperEdge.hidden;
          }
          
          // Fallback: Check if this hyperEdge represents the same connectivity
          // This handles cases where aggregatedEdges might not be properly set
          return (
            (hyperEdge.source === crossingEdge.source || hyperEdge.source === containerId) &&
            (hyperEdge.target === crossingEdge.target || hyperEdge.target === containerId) &&
            !hyperEdge.hidden
          );
        });
        
        if (!isHyperEdge && representingHyperEdges.length === 0) {
          violations.push({
            type: 'MISSING_HYPEREDGE',
            message: `Edge ${crossingEdge.id} crosses collapsed container ${containerId} but no hyperEdge represents this connectivity`,
            entityId: crossingEdge.id,
            severity: 'warning'
          });
        }
      }
    }

    return violations;
  }

  private validateCollapsedContainerDimensions(): InvariantViolation[] {
    const violations: InvariantViolation[] = [];

    for (const [containerId, container] of this.state.containers) {
      if (!container.collapsed) continue;

      // Check if collapsed container has suspiciously large dimensions
      const width = container.width || 0;
      const height = container.height || 0;
      const maxAllowedWidth = LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH * 3;
      const maxAllowedHeight = LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT * 3;

      if (width > maxAllowedWidth || height > maxAllowedHeight) {
        violations.push({
          type: 'COLLAPSED_CONTAINER_LARGE_DIMENSIONS',
          message: `Collapsed container ${containerId} has large dimensions (${width}x${height}) that may cause layout issues`,
          entityId: containerId,
          severity: 'warning'
        });
      }
    }

    return violations;
  }

  private validatePositionedContainerConsistency(): InvariantViolation[] {
    const violations: InvariantViolation[] = [];

    for (const [containerId, container] of this.state.containers) {
      if (container.hidden) continue;

      // Check if container has position but no dimensions (can cause layout issues)
      const hasPosition = container.x !== undefined && container.y !== undefined;
      const hasDimensions = container.width !== undefined && container.height !== undefined;

      if (hasPosition && !hasDimensions) {
        violations.push({
          type: 'POSITIONED_CONTAINER_NO_DIMENSIONS',
          message: `Container ${containerId} has position but missing dimensions`,
          entityId: containerId,
          severity: 'warning'
        });
      }
    }

    return violations;
  }

  private _isEntityVisible(entityId: string, container?: any, node?: any): boolean {
    // Check if entity is hidden
    if (container && container.hidden) return false;
    if (node && node.hidden) return false;
    
    // Check if node is inside a collapsed container
    if (node) {
      const parentContainerId = this.state.getNodeContainer(entityId);
      if (parentContainerId) {
        const parentContainer = this.state.getContainer(parentContainerId);
        if (parentContainer && parentContainer.collapsed) {
          return false; // Node inside collapsed container is not visible
        }
      }
    }
    
    return true;
  }
}
