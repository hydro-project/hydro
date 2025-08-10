/**
 * Visualization State - Core Data Structure
 * 
 * Maintains the mutable state of the visualization including nodes, edges, containers, and hyperEdges.
 * Provides efficient access to visible/non-hidden elements through Maps and collections.
 */

import {
  NODE_STYLES,
  EDGE_STYLES, 
  CONTAINER_STYLES,
  CreateNodeProps,
  CreateEdgeProps,
  CreateContainerProps,
  GraphNode,
  GraphEdge,
  Container
} from '../shared/types';
import { LAYOUT_CONSTANTS, HYPEREDGE_CONSTANTS } from '../shared/config';
import { ContainerPadding } from './ContainerPadding';
// Simple assertion function that works in both Node.js and browser environments
function assert(condition: any, message?: string): asserts condition {
  if (!condition) {
    throw new Error(message || 'Assertion failed');
  }
}

// Constants for consistent string literals
const DEFAULT_STYLE = 'default';

// ============ VISUALIZATION STATE INVARIANTS ============

interface InvariantViolation {
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
class VisualizationStateInvariantValidator {
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
    const errors = violations.filter(v => v.severity === 'error');
    const warnings = violations.filter(v => v.severity === 'warning');

    if (warnings.length > 0) {
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
      
      // Log other warnings normally (HYPEREDGE_TO_HIDDEN_CONTAINER now excluded)
      const otherWarnings = warnings.filter(w => w.type !== 'HYPEREDGE_TO_HIDDEN_CONTAINER');
      if (otherWarnings.length > 0) {
        console.warn(`[VisState] Invariant warnings (${otherWarnings.length}):`, otherWarnings);
      }
      
      // Show summary only for non-hyperEdge warnings
      if (Object.keys(warningsByType).length > 0) {
        console.warn(`[VisState] Warning summary:`, warningsByType);
      }
    }

    if (errors.length > 0) {
      console.error(`[VisState] CRITICAL: Invariant violations (${errors.length}):`, errors);
      throw new Error(`VisualizationState invariant violations detected: ${errors.map(e => e.message).join('; ')}`);
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
        // instead of looking for a specific naming pattern
        const representingHyperEdges = Array.from(this.state.hyperEdges.values()).filter((hyperEdge: any) => {
          // Check if this hyperEdge represents the same connectivity as the crossing edge
          return (
            (hyperEdge.source === crossingEdge.source || hyperEdge.source === containerId) &&
            (hyperEdge.target === crossingEdge.target || hyperEdge.target === containerId) &&
            !hyperEdge.hidden
          );
        });
        
        if (representingHyperEdges.length === 0) {
          violations.push({
            type: 'MISSING_HYPEREDGE_FOR_CROSSING',
            message: `Collapsed container ${containerId} has crossing edge ${crossingEdge.id} but no corresponding hyperEdge`,
            entityId: containerId,
            severity: 'error'
          });
        }
      }
    }

    return violations;
  }

  // ============ Layout Invariants ============

  private validateCollapsedContainerDimensions(): InvariantViolation[] {
    const violations: InvariantViolation[] = [];

    for (const [containerId, container] of this.state.containers) {
      if (container.collapsed) {
        const layout = this.state.getContainerLayout(containerId);
        if (layout && layout.dimensions) {
          const { width, height } = layout.dimensions;
          
          // Collapsed containers should have standard small dimensions
          if (width > 300 || height > 200) {
            violations.push({
              type: 'COLLAPSED_CONTAINER_LARGE_DIMENSIONS',
              message: `Collapsed container ${containerId} has unexpectedly large dimensions: ${width}x${height}`,
              entityId: containerId,
              severity: 'warning'
            });
          }
        }
      }
    }

    return violations;
  }

  private validatePositionedContainerConsistency(): InvariantViolation[] {
    const violations: InvariantViolation[] = [];

    for (const [containerId, container] of this.state.containers) {
      const layout = this.state.getContainerLayout(containerId);
      
      // If container has position, it should probably be visible (unless temporarily hidden)
      if (layout && layout.position && container.hidden) {
        violations.push({
          type: 'POSITIONED_CONTAINER_HIDDEN',
          message: `Container ${containerId} has layout position but is hidden - might indicate stale layout data`,
          entityId: containerId,
          severity: 'warning'
        });
      }
    }

    return violations;
  }

  /**
   * Helper method to check if an entity (node or container) is visible
   */
  private _isEntityVisible(entityId: string, container?: any, node?: any): boolean {
    // If it's a visible node
    if (node && !node.hidden) {
      // Check if the node is inside a collapsed container
      const parentContainerId = this.state.getNodeContainer(entityId);
      if (parentContainerId) {
        const parentContainer = this.state.getContainer(parentContainerId);
        if (parentContainer && parentContainer.collapsed) {
          return false; // Node inside collapsed container is effectively hidden
        }
      }
      return true;
    }
    
    // If it's a visible container
    if (container && !container.hidden) {
      return true;
    }
    
    return false;
  }
}

// Entity types for generic operations
const ENTITY_TYPES = {
  NODE: 'node',
  EDGE: 'edge',
  CONTAINER: 'container',
  HYPER_EDGE: 'hyperEdge'
};

/**
 * Read-only interface for container hierarchy information
 * Prevents external code from modifying the internal structure
 */
export interface ContainerHierarchyView {
  getContainerChildren(containerId: string): ReadonlySet<string>;
  getNodeContainer(nodeId: string): string | undefined;
}

/**
 * Core visualization state class that manages all graph elements including nodes, edges, 
 * containers, and hyperEdges with efficient visibility tracking and hierarchy management.
 * 
 * Features:
 * - O(1) element lookups using Maps
 * - Automatic visibility management
 * - Hierarchical container support with collapse/expand
 * - Edge <-> hyperEdge conversion for collapse/expand
 * - Efficient update operations
 * - Runtime-enforced encapsulation for container hierarchy
 * 
 * @class VisualizationState
 * @example
 * ```javascript
 * // Modern idiomatic usage with getters
 * const state = createVisualizationState()
 *   .setGraphNode('n1', { label: 'Node 1' })
 *   .setGraphNode('n2', { label: 'Node 2' })
 *   .setGraphEdge('e1', { source: 'n1', target: 'n2' })
 *   .setContainer('c1', { children: ['n1', 'n2'] });
 * 
 * // ============ RECOMMENDED PUBLIC API FOR EXTERNAL SYSTEMS ============
 * // Bridges and external systems should ONLY use these getters:
 * // console.log(state.visibleNodes);       // Array of visible nodes  
 * // console.log(state.visibleEdges);       // Array of visible edges
 * // console.log(state.visibleContainers);  // Array of visible containers (includes collapsed)
 * // console.log(state.getExpandedContainers()); // Array of expanded containers (recommended)
 * 
 * // ============ DEPRECATED/INTERNAL STATE (avoid in bridges) ============
 * // console.log(state.expandedContainers); // DEPRECATED - exposes internal state
 * 
 * // Update properties idiomatically  
 * state.updateNode('n1', { hidden: true, style: 'highlighted' });
 * state.updateContainer('c1', { collapsed: true });
 * ```
 */
export class VisualizationState implements ContainerHierarchyView {
  // Protected state collections - NEVER access directly!
  private readonly _collections = {
    // Core data stores
    graphNodes: new Map<string, any>(),
    graphEdges: new Map<string, any>(),
    containers: new Map<string, any>(),
    hyperEdges: new Map<string, any>(),
    
    // Visibility caches (derived state)
    _visibleNodes: new Map<string, any>(),
    _visibleEdges: new Map<string, any>(),
    _visibleContainers: new Map<string, any>(),
    _expandedContainers: new Map<string, any>(),
    
    // Specialized collections
    collapsedContainers: new Map<string, any>(),
    nodeToEdges: new Map<string, Set<string>>(),
    manualPositions: new Map<string, {x: number, y: number}>(),
    
    // Hierarchy tracking
    containerChildren: new Map<string, Set<string>>(),
    nodeContainers: new Map<string, string>()
  };
  
  // Invariant validation system
  private readonly invariantValidator: VisualizationStateInvariantValidator;

  // ============ PROTECTED ACCESSORS (Internal use only) ============
  // These provide controlled access to collections for internal methods
  
  private get graphNodes(): Map<string, any> { return this._collections.graphNodes; }
  private get graphEdges(): Map<string, any> { return this._collections.graphEdges; }
  private get containers(): Map<string, any> { return this._collections.containers; }
  private get hyperEdges(): Map<string, any> { return this._collections.hyperEdges; }
  private get _visibleNodes(): Map<string, any> { return this._collections._visibleNodes; }
  private get _visibleEdges(): Map<string, any> { return this._collections._visibleEdges; }
  private get _visibleContainers(): Map<string, any> { return this._collections._visibleContainers; }
  private get _expandedContainers(): Map<string, any> { return this._collections._expandedContainers; }
  private get collapsedContainers(): Map<string, any> { return this._collections.collapsedContainers; }
  private get nodeToEdges(): Map<string, Set<string>> { return this._collections.nodeToEdges; }
  private get manualPositions(): Map<string, {x: number, y: number}> { return this._collections.manualPositions; }
  
  // Hierarchy tracking (with protected access)
  private get containerChildren(): Map<string, Set<string>> { return this._collections.containerChildren; }
  private get nodeContainers(): Map<string, string> { return this._collections.nodeContainers; }

  /**
   * Create a new VisualizationState instance
   * @constructor
   */
  constructor() {
    // Initialize invariant validator
    this.invariantValidator = new VisualizationStateInvariantValidator(this);
  }

  // ============ SAFE BRIDGE API (Read-only access for external systems) ============
  
  /**
   * Get visible nodes for rendering (safe read-only access)
   * Bridges should ONLY use this method, never access internal maps directly
   */
  get visibleNodes(): ReadonlyArray<any> {
    // Development assertion: Check for consistency violations
    if (process.env.NODE_ENV !== 'production') {
      const visibleNodes = Array.from(this._collections._visibleNodes.values());
      for (const node of visibleNodes) {
        // Find the node's parent container (if any)
        for (const [containerId, children] of this._collections.containerChildren.entries()) {
          if (children.has(node.id)) {
            const container = this._collections.containers.get(containerId);
            if (container && container.collapsed) {
              throw new Error(`BUG: Node ${node.id} is in _visibleNodes but its parent container ${containerId} is collapsed`);
            }
          }
        }
      }
    }
    
    return Array.from(this._collections._visibleNodes.values());
  }
  
  /**
   * Get visible edges for rendering (safe read-only access)  
   * Bridges should ONLY use this method, never access internal maps directly
   */
  get visibleEdges(): ReadonlyArray<any> {
    // Include both regular visible edges and visible hyperEdges
    const regularEdges = Array.from(this._collections._visibleEdges.values());
    const hyperEdges = Array.from(this._collections.hyperEdges.values()).filter((edge: any) => {
      if (edge.hidden) return false;
      
      // Validate that hyperEdge endpoints still exist and are visible
      const sourceExists = this._collections.graphNodes.has(edge.source) || 
                          this._collections.containers.has(edge.source);
      const targetExists = this._collections.graphNodes.has(edge.target) || 
                          this._collections.containers.has(edge.target);
      
      if (!sourceExists || !targetExists) {
        console.warn(`[HYPEREDGE] Filtering out hyperEdge ${edge.id} - invalid endpoints (source: ${sourceExists}, target: ${targetExists})`);
        // NOTE: Do NOT mutate state in getter - just filter out for now
        return false;
      }
      
      const sourceVisible = this._isNodeOrContainerVisible(edge.source);
      const targetVisible = this._isNodeOrContainerVisible(edge.target);
      
      if (!sourceVisible || !targetVisible) {
        console.warn(`[HYPEREDGE] Filtering out hyperEdge ${edge.id} - endpoints not visible (source: ${sourceVisible}, target: ${targetVisible})`);
        // NOTE: Do NOT mutate state in getter - just filter out for now
        return false;
      }
      
      return true;
    });
    
    return [...regularEdges, ...hyperEdges];
  }
  
  /**
   * Get visible containers for rendering (safe read-only access)
   * Bridges should ONLY use this method, never access internal maps directly
   * Returns containers with dimensions adjusted for labels.
   */
  get visibleContainers(): ReadonlyArray<any> {
    return Array.from(this._collections._visibleContainers.values()).map(container => {
      const adjustedDimensions = this.getContainerAdjustedDimensions(container.id);
      return {
        ...container,
        width: adjustedDimensions.width,
        height: adjustedDimensions.height
      };
    });
  }
  
  /**
   * Get visible hyperEdges for rendering (safe read-only access)
   * Used by tests and debugging - filters out hidden hyperEdges
   */
  get visibleHyperEdges(): ReadonlyArray<any> {
    return Array.from(this._collections.hyperEdges.values()).filter((edge: any) => {
      if (edge.hidden) return false;
      
      // Validate that hyperEdge endpoints still exist and are visible
      const sourceExists = this._collections.graphNodes.has(edge.source) || 
                          this._collections.containers.has(edge.source);
      const targetExists = this._collections.graphNodes.has(edge.target) || 
                          this._collections.containers.has(edge.target);
      
      if (!sourceExists || !targetExists) {
        return false;
      }
      
      const sourceVisible = this._isNodeOrContainerVisible(edge.source);
      const targetVisible = this._isNodeOrContainerVisible(edge.target);
      
      return sourceVisible && targetVisible;
    });
  }
  
  /**
   * Get expanded containers (safe read-only access)
   * Bridges should ONLY use this method, never access internal maps directly
   */
  getExpandedContainers(): ReadonlyArray<any> {
    return Array.from(this._collections._expandedContainers.values());
  }

  /**
   * Container hierarchy access (backwards compatibility)
   */
  getContainerChildren(containerId: string): ReadonlySet<string> {
    return this._collections.containerChildren.get(containerId) || new Set();
  }
  
  getNodeContainer(nodeId: string): string | undefined {
    return this._collections.nodeContainers.get(nodeId);
  }

  // ============ LEGACY/COMPATIBILITY API ============
  // These methods maintain compatibility with existing JSONParser and test code
  
  /**
   * Set a graph node (legacy compatibility method)
   * @deprecated Use the new controlled mutation API when possible
   */
  setGraphNode(nodeId: string, nodeData: any): VisualizationState {
    // Check if node belongs to a collapsed container and should be hidden
    const parentContainer = this._collections.nodeContainers.get(nodeId);
    let shouldBeHidden = nodeData.hidden;
    
    if (parentContainer) {
      const container = this._collections.containers.get(parentContainer);
      if (container && container.collapsed) {
        shouldBeHidden = true; // Force hidden if parent container is collapsed
      }
    }
    
    // Ensure all nodes have default dimensions
    const processedData = { 
      ...nodeData, 
      id: nodeId, 
      hidden: shouldBeHidden,
      width: nodeData.width || 180,  // DEFAULT_NODE_WIDTH
      height: nodeData.height || 60  // DEFAULT_NODE_HEIGHT
    };
    
    // Store the node data with correct hidden state and dimensions
    this._collections.graphNodes.set(nodeId, processedData);
    
    // Add to visible nodes only if not hidden
    if (!shouldBeHidden) {
      this._collections._visibleNodes.set(nodeId, this._collections.graphNodes.get(nodeId));
    }
    
    return this;
  }
  
  /**
   * Set a graph edge (legacy compatibility method)
   * @deprecated Use the new controlled mutation API when possible
   */
  setGraphEdge(edgeId: string, edgeData: any): VisualizationState {
    // Store the edge data
    this._collections.graphEdges.set(edgeId, { ...edgeData, id: edgeId });
    
    // Update node-to-edges mapping
    const sourceEdges = this._collections.nodeToEdges.get(edgeData.source) || new Set();
    sourceEdges.add(edgeId);
    this._collections.nodeToEdges.set(edgeData.source, sourceEdges);
    
    const targetEdges = this._collections.nodeToEdges.get(edgeData.target) || new Set();
    targetEdges.add(edgeId);
    this._collections.nodeToEdges.set(edgeData.target, targetEdges);
    
    // Add to visible edges if not hidden and endpoints are visible
    if (!edgeData.hidden && this._isEndpointVisible(edgeData.source) && this._isEndpointVisible(edgeData.target)) {
      this._collections._visibleEdges.set(edgeId, this._collections.graphEdges.get(edgeId));
    }
    
    return this;
  }
  
  /**
   * Set a container (legacy compatibility method)
   * @deprecated Use the new controlled mutation API when possible
   */
  setContainer(containerId: string, containerData: any): VisualizationState {
    // Convert children array to Set for ELKBridge compatibility
    const processedData = { ...containerData, id: containerId };
    if (containerData.children && Array.isArray(containerData.children)) {
      processedData.children = new Set<string>(containerData.children);
    }
    
    // ENCAPSULATION: External code CANNOT set dimensions - only VisState controls dimensions
    // Dimensions are set by ELK results via setContainerLayout, or by internal defaults
    // Remove any dimension-related properties that external code might try to set
    delete processedData.width;
    delete processedData.height;
    delete processedData.expandedDimensions;
    
    // Set internal default dimensions - external code has no control over these
    if (containerData.collapsed) {
      processedData.width = LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH;
      processedData.height = LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT;
    } else {
      processedData.width = LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH;
      processedData.height = LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT;
    }
    
    // Store the container data with Set children and default dimensions
    this._collections.containers.set(containerId, processedData);
    
    // Update hierarchy tracking
    if (containerData.children) {
      const childrenSet = new Set<string>(containerData.children);
      this._collections.containerChildren.set(containerId, childrenSet);
      
      // Update node-to-container mapping
      for (const childId of containerData.children) {
        this._collections.nodeContainers.set(childId, containerId);
      }
    }
    
    // Update visibility caches
    const container = this._collections.containers.get(containerId)!;
    this._updateContainerVisibilityCaches(containerId, container);
    
    // CRITICAL: If container is created with collapsed=true, immediately hide all children
    if (containerData.collapsed) {
      this._inRecursiveOperation = true;
      try {
        this._hideAllDescendants(containerId);
      } finally {
        this._inRecursiveOperation = false;
      }
    }
    
    return this;
  }
  
  /**
   * Get a graph node (legacy compatibility method)
   */
  getGraphNode(nodeId: string): any | undefined {
    const node = this._collections.graphNodes.get(nodeId);
    if (!node) return undefined;
    
    // Include visibility information for backwards compatibility with tests
    return {
      ...node,
      hidden: node.hidden || !this._collections._visibleNodes.has(nodeId)
    };
  }
  
  /**
   * Get a container (legacy compatibility method)
   */
  getContainer(containerId: string): any | undefined {
    return this._collections.containers.get(containerId);
  }
  
  /**
   * Get crossing edges for a container (legacy compatibility method)
   */
  getCrossingEdges(containerId: string): any[] {
    const allDescendantNodes = new Set(this._getAllDescendantNodes(containerId));
    const crossingEdges: any[] = [];

    // Check all visible regular edges
    for (const [edgeId, edge] of this._collections.graphEdges) {
      if (edge.hidden) continue; // Skip hidden edges

      const sourceInContainer = allDescendantNodes.has(edge.source);
      const targetInContainer = allDescendantNodes.has(edge.target);

      // Edge crosses boundary if exactly one endpoint is in container
      if (sourceInContainer !== targetInContainer) {
        crossingEdges.push(edge);
      }
    }

    // Also check visible hyperedges (for nested collapsed containers)
    for (const [hyperEdgeId, hyperEdge] of this._collections.hyperEdges) {
      if (hyperEdge.hidden) continue;

      const sourceInContainer = allDescendantNodes.has(hyperEdge.source);
      const targetInContainer = allDescendantNodes.has(hyperEdge.target);

      if (sourceInContainer !== targetInContainer) {
        crossingEdges.push(hyperEdge);
      }
    }

    return crossingEdges;
  }
  
  /**
   * Get all descendant nodes for a container (helper method)
   */
  private _getAllDescendantNodes(containerId: string): string[] {
    const descendants: string[] = [];
    const children = this._collections.containerChildren.get(containerId) || new Set();
    
    for (const childId of Array.from(children)) {
      const childContainer = this._collections.containers.get(childId);
      if (childContainer) {
        // If it's a container, recursively get its descendants
        descendants.push(...this._getAllDescendantNodes(childId));
      } else {
        // If it's a node, add it directly
        descendants.push(childId);
      }
    }
    
    return descendants;
  }
  
  /**
   * Get container layout (legacy compatibility method)
   */
  getContainerLayout(containerId: string): { position?: { x: number; y: number }; dimensions?: { width: number; height: number } } | undefined {
    const container = this._collections.containers.get(containerId);
    if (!container) return undefined;
    
    return {
      position: (container.x !== undefined && container.y !== undefined) ? { x: container.x, y: container.y } : undefined,
      dimensions: (container.width !== undefined && container.height !== undefined) ? { width: container.width, height: container.height } : undefined
    };
  }
  
  /**
   * Get container adjusted dimensions (legacy compatibility method)
   */
  getContainerAdjustedDimensions(containerId: string): { width: number; height: number } {
    const container = this._collections.containers.get(containerId);
    if (!container) {
      throw new Error(`Container ${containerId} not found`);
    }
    
    // CRITICAL: Check if collapsed FIRST - collapsed containers should always use small dimensions
    if (container.collapsed) {
      // Always use minimum dimensions for collapsed containers, regardless of expandedDimensions
      return { 
        width: LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH,
        height: LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT + 
                LAYOUT_CONSTANTS.CONTAINER_LABEL_HEIGHT + LAYOUT_CONSTANTS.CONTAINER_LABEL_PADDING
      };
    }
    
    // Get base dimensions from various possible sources (only for expanded containers)
    let baseWidth, baseHeight;
    
    if (container.expandedDimensions) {
      // expandedDimensions should already include padding from when they were first cached
      // Don't add padding again - just return the cached padded dimensions
      baseWidth = container.expandedDimensions.width;
      baseHeight = container.expandedDimensions.height;
      
      return { 
        width: baseWidth,
        height: baseHeight
      };
    } else {
      // No cached dimensions - use raw dimensions and add padding
      baseWidth = container.width;
      baseHeight = container.height;
    }
    
    // For expanded containers without cached dimensions, add label space to height
    const width = baseWidth || LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH;
    const height = baseHeight || LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT;
    
    return { 
      width: width, 
      height: height + LAYOUT_CONSTANTS.CONTAINER_LABEL_HEIGHT + LAYOUT_CONSTANTS.CONTAINER_LABEL_PADDING
    };
  }
  
  /**
   * Set container layout (applies padding and caches as expandedDimensions)
   */
  setContainerLayout(containerId: string, layout: any): void {
    // For now, just update the container with layout information
    const container = this._collections.containers.get(containerId);
    if (container) {
      container.layout = layout;
      if (layout.position) {
        container.x = layout.position.x;
        container.y = layout.position.y;
      }
      if (layout.dimensions) {
        // Raw dimensions from ELK - apply padding and cache as expandedDimensions
        const rawWidth = layout.dimensions.width;
        const rawHeight = layout.dimensions.height;
        
        // Apply label padding for expanded containers
        if (!container.collapsed) {
          const paddedHeight = rawHeight + LAYOUT_CONSTANTS.CONTAINER_LABEL_HEIGHT + LAYOUT_CONSTANTS.CONTAINER_LABEL_PADDING;
          
          // Cache the padded dimensions so subsequent calls don't need to recalculate
          container.expandedDimensions = {
            width: rawWidth,
            height: paddedHeight
          };
          
          // Also update the basic width/height for backwards compatibility
          container.width = rawWidth;
          container.height = paddedHeight;
        } else {
          // For collapsed containers, ensure minimum dimensions with padding
          const minWidth = Math.max(rawWidth, LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH);
          const minHeight = Math.max(
            rawHeight, 
            LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT
          ) + LAYOUT_CONSTANTS.CONTAINER_LABEL_HEIGHT + LAYOUT_CONSTANTS.CONTAINER_LABEL_PADDING;
          
          container.width = minWidth;
          container.height = minHeight;
          
          // Don't set expandedDimensions for collapsed containers
        }
      }
    }
  }
  
  /**
   * Set node layout (legacy compatibility method)
   */
  setNodeLayout(nodeId: string, layout: any): void {
    const node = this._collections.graphNodes.get(nodeId);
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
   * Get node layout information
   */
  getNodeLayout(nodeId: string): { position?: { x: number; y: number }; dimensions?: { width: number; height: number } } | undefined {
    const node = this._collections.graphNodes.get(nodeId);
    if (!node) return undefined;
    
    return {
      position: (node.x !== undefined && node.y !== undefined) ? { x: node.x, y: node.y } : undefined,
      dimensions: (node.width !== undefined && node.height !== undefined) ? { width: node.width, height: node.height } : undefined
    };
  }
  
  /**
   * Collapse a container (legacy compatibility method)
   * This will be replaced by setContainerState in the future
   */
  collapseContainer(containerId: string): void {
    // Check if container exists before attempting to collapse
    const container = this._collections.containers.get(containerId);
    if (!container) {
      throw new Error(`Cannot collapse non-existent container: ${containerId}`);
    }
    
    this._inRecursiveOperation = true;
    try {
      this.setContainerState(containerId, { collapsed: true });
    } finally {
      this._inRecursiveOperation = false;
      // Validate at the end of the complete operation
      this.validateInvariants();
    }
  }
  
  /**
   * Expand a container (legacy compatibility method)
   * This will be replaced by setContainerState in the future
   */
  expandContainer(containerId: string): void {
    // Check if container exists before attempting to expand
    const container = this._collections.containers.get(containerId);
    if (!container) {
      throw new Error(`Cannot expand non-existent container: ${containerId}`);
    }
    
    this.setContainerState(containerId, { collapsed: false });
  }

  // ============ MINIMAL BRIDGE COMPATIBILITY (for engines only) ============
  // These provide MINIMAL access for bridges - prefer using the safe getters above
  
  /**
   * Get container ELK fixed status (minimal bridge compatibility)
   * @internal - Only for VisualizationEngine compatibility
   */
  getContainerELKFixed(containerId: string): boolean {
    const container = this._collections.containers.get(containerId);
    return container?.elkFixed || false;
  }

  // ============ TEMPORARY TEST COMPATIBILITY (to be removed) ============
  // These methods provide temporary compatibility for tests during refactoring
  // TODO: Update tests to use proper encapsulated API and remove these methods
  
  /**
   * @deprecated Use visibleContainers.filter(c => c.collapsed) instead
   */
  getContainerCollapsed(containerId: string): boolean {
    const container = this._collections.containers.get(containerId);
    return container?.collapsed || false;
  }
  
  /**
   * @deprecated Use setContainerState() instead
   */
  setContainerCollapsed(containerId: string, collapsed: boolean): void {
    this.setContainerState(containerId, { collapsed });
  }
  
  /**
   * @deprecated Use visibleContainers.filter(c => !c.collapsed) instead
   */
  get expandedContainers(): ReadonlyArray<any> {
    return Array.from(this._collections._expandedContainers.values());
  }
  
  // ============ CONTROLLED STATE MUTATION API ============
  // These are the ONLY safe ways to modify state - ensures consistency
  
  /**
   * Get a hyperEdge (legacy compatibility method)
   */
  getHyperEdge(hyperEdgeId: string): any | undefined {
    return this._collections.hyperEdges.get(hyperEdgeId);
  }
  
  /**
   * Update container properties (legacy compatibility method)
   */
  updateContainer(containerId: string, updates: any): void {
    const container = this._collections.containers.get(containerId);
    if (container) {
      Object.assign(container, updates);
      this._updateContainerVisibilityCaches(containerId, container);
    }
  }
  
  /**
   * Update edge properties (legacy compatibility method)
   */
  updateEdge(edgeId: string, updates: any): void {
    const edge = this._collections.graphEdges.get(edgeId);
    if (edge) {
      Object.assign(edge, updates);
      
      // Update visibility cache
      if (updates.hidden !== undefined) {
        if (updates.hidden) {
          this._collections._visibleEdges.delete(edgeId);
        } else if (this._isEndpointVisible(edge.source) && this._isEndpointVisible(edge.target)) {
          this._collections._visibleEdges.set(edgeId, edge);
        }
      }
    }
  }
  
  /**
   * Get graph edge (legacy compatibility method)
   */
  getGraphEdge(edgeId: string): any | undefined {
    const edge = this._collections.graphEdges.get(edgeId);
    if (!edge) return undefined;
    
    // Include visibility information for backwards compatibility with tests
    const isVisible = this._collections._visibleEdges.has(edgeId);
    return {
      ...edge,
      hidden: edge.hidden || !isVisible
    };
  }

  // ============ CONTROLLED STATE MUTATION API ============
  // These are the ONLY safe ways to modify state - ensures consistency
  
  /**
   * Safely set node visibility with automatic cache updates and edge cascade
   * This is the ONLY way node visibility should be changed
   */
  setNodeVisibility(nodeId: string, visible: boolean): void {
    const node = this._collections.graphNodes.get(nodeId);
    if (!node) {
      console.warn(`[VisualizationState] Cannot set visibility for non-existent node: ${nodeId}`);
      return;
    }
    
    const wasVisible = !node.hidden;
    node.hidden = !visible;
    
    // Update visibility cache atomically
    if (visible) {
      this._collections._visibleNodes.set(nodeId, node);
    } else {
      this._collections._visibleNodes.delete(nodeId);
    }
    
    // Cascade visibility to connected edges
    this._cascadeNodeVisibilityToEdges(nodeId, visible);
    
    // Only validate at the end for top-level operations
    if (!this._inRecursiveOperation) {
      this.validateInvariants();
    }
  }
  
  /**
   * Safely set container state with proper cascade and hyperEdge management  
   * This is the ONLY way container state should be changed
   */
  setContainerState(containerId: string, state: {collapsed?: boolean, hidden?: boolean}): void {
    const container = this._collections.containers.get(containerId);
    if (!container) {
      console.warn(`[VisualizationState] Cannot set state for non-existent container: ${containerId}`);
      return;
    }
    
    const wasCollapsed = container.collapsed;
    const wasHidden = container.hidden;
    
    // Apply state changes
    if (state.collapsed !== undefined) {
      container.collapsed = state.collapsed;
      
      // CRITICAL: Always override dimensions when collapsing to prevent dimension explosion
      if (state.collapsed) {
        container.width = LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH;   // Force small collapsed width
        container.height = LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT;  // Force small collapsed height
      }
    }
    if (state.hidden !== undefined) container.hidden = state.hidden;
    
    // Update visibility caches
    this._updateContainerVisibilityCaches(containerId, container);
    
    // Handle collapse/expand transitions with hyperEdge management
    if (state.collapsed !== undefined && state.collapsed !== wasCollapsed) {
      if (state.collapsed) {
        this._handleContainerCollapse(containerId);
      } else {
        this._handleContainerExpansion(containerId);
      }
    }
    
    // Handle hide/show transitions  
    if (state.hidden !== undefined && state.hidden !== wasHidden) {
      this._cascadeContainerVisibility(containerId, !state.hidden);
    }
    
    // Only validate at the end for top-level operations
    // Skip validation during recursive operations to avoid performance issues
    if (!this._inRecursiveOperation) {
      this.validateInvariants();
    }
  }
  
  // Flag to track recursive operations
  private _inRecursiveOperation = false;
  
  /**
   * Safely set edge visibility with endpoint validation
   * This is the ONLY way edge visibility should be changed
   */
  setEdgeVisibility(edgeId: string, visible: boolean): void {
    const edge = this._collections.graphEdges.get(edgeId);
    if (!edge) {
      console.warn(`[VisualizationState] Cannot set visibility for non-existent edge: ${edgeId}`);
      return;
    }
    
    // Validate endpoints are visible before making edge visible
    if (visible) {
      const sourceValid = this._isEndpointVisible(edge.source);
      const targetValid = this._isEndpointVisible(edge.target);
      
      if (!sourceValid || !targetValid) {
        console.warn(`[VisualizationState] Cannot make edge ${edgeId} visible - endpoints not visible`);
        return;
      }
    }
    
    edge.hidden = !visible;
    
    // Update visibility cache
    if (visible) {
      this._collections._visibleEdges.set(edgeId, edge);
    } else {
      this._collections._visibleEdges.delete(edgeId);
    }
  }

  // ============ PRIVATE CONSISTENCY HELPERS ============
  
  private _cascadeNodeVisibilityToEdges(nodeId: string, nodeVisible: boolean): void {
    const connectedEdges = this._collections.nodeToEdges.get(nodeId) || new Set();
    
    for (const edgeId of Array.from(connectedEdges)) {
      const edge = this._collections.graphEdges.get(edgeId);
      if (!edge) continue;
      
      // Edge can only be visible if both endpoints are visible
      const sourceVisible = this._isEndpointVisible(edge.source);
      const targetVisible = this._isEndpointVisible(edge.target);
      const shouldBeVisible = sourceVisible && targetVisible;
      
      this.setEdgeVisibility(edgeId, shouldBeVisible);
    }
  }
  
  private _updateContainerVisibilityCaches(containerId: string, container: any): void {
    // Update _visibleContainers (includes collapsed containers)
    if (!container.hidden) {
      this._collections._visibleContainers.set(containerId, container);
    } else {
      this._collections._visibleContainers.delete(containerId);
    }
    
    // Update _expandedContainers (only non-collapsed containers)
    if (!container.hidden && !container.collapsed) {
      this._collections._expandedContainers.set(containerId, container);
    } else {
      this._collections._expandedContainers.delete(containerId);
    }
    
    // Update collapsedContainers
    if (container.collapsed && !container.hidden) {
      this._collections.collapsedContainers.set(containerId, container);
    } else {
      this._collections.collapsedContainers.delete(containerId);
    }
  }
  
  private _handleContainerCollapse(containerId: string): void {
    console.log(`[DEBUG] Starting collapse for container ${containerId}`);
    
    // 1. Find crossing edges BEFORE hiding descendants
    const crossingEdges = this.getCrossingEdges(containerId);
    console.log(`[COLLAPSE] Found ${crossingEdges.length} crossing edges for container ${containerId}`);
    
    // 2. Hide all descendants
    console.log(`[DEBUG] About to hide descendants of ${containerId}`);
    this._hideAllDescendants(containerId);
    console.log(`[DEBUG] After hiding descendants, visible nodes: ${this._collections._visibleNodes.size}, visible containers: ${this._collections._visibleContainers.size}`);

    assert(this._collections._visibleContainers.has(containerId), 
           `Container ${containerId} should still be visible after hiding descendants during collapse`);

    // 3. Create hyperEdges to replace crossing edges
    const hyperEdges = this._prepareHyperedges(containerId, crossingEdges);
    console.log(`[COLLAPSE] Creating ${hyperEdges.length} hyperEdges for container ${containerId}`);
    
    for (const hyperEdge of hyperEdges) {
      console.log(`[DEBUG] Creating hyperEdge: ${hyperEdge.id} (${hyperEdge.source} -> ${hyperEdge.target})`);
      // Create hyperEdge using internal API (not the removed setHyperEdge)
      this._collections.hyperEdges.set(hyperEdge.id, hyperEdge);
      
      // Update node-to-edges mapping for hyperEdges
      const sourceEdges = this._collections.nodeToEdges.get(hyperEdge.source) || new Set();
      sourceEdges.add(hyperEdge.id);
      this._collections.nodeToEdges.set(hyperEdge.source, sourceEdges);
      
      const targetEdges = this._collections.nodeToEdges.get(hyperEdge.target) || new Set();
      targetEdges.add(hyperEdge.id);
      this._collections.nodeToEdges.set(hyperEdge.target, targetEdges);
    }
    
    // 4. Hide the original crossing edges
    console.log(`[DEBUG] About to hide ${crossingEdges.length} crossing edges`);
    for (const edge of crossingEdges) {
      if (edge.id.startsWith(HYPEREDGE_CONSTANTS.PREFIX)) {
        // HYPOTHESIS 3 TEST: Is this breaking encapsulation?
        console.log(`[DEBUG] [ENCAPSULATION BREACH] Directly deleting hyperEdge: ${edge.id}`);
        // For hyperEdges, simply delete
        this._collections.hyperEdges.delete(edge.id);
        // edge.hidden = true;
      } else {
        console.log(`[DEBUG] Hiding regular edge: ${edge.id}`);
        // For regular edges, update the hidden flag
        edge.hidden = true;
        this._collections._visibleEdges.delete(edge.id);
      }
    }
    
    // 5. CRITICAL FIX: Update existing hyperEdges that now have invalid endpoints
    // This is the missing piece that causes hyperEdges to connect to hidden entities
    console.log(`[DEBUG] Updating existing hyperEdges with invalid endpoints due to collapse of ${containerId}`);
    this._updateInvalidatedHyperEdges(containerId);
    
    // HYPOTHESIS 1 TEST: Are descendants properly hidden?
    console.log(`[DEBUG] Post-collapse visibility check for ${containerId}:`);
    const children = this.getContainerChildren(containerId);
    for (const childId of children) {
      const childNode = this._collections.graphNodes.get(childId);
      const childContainer = this._collections.containers.get(childId);
      if (childNode) {
        console.log(`[DEBUG]   Child node ${childId}: hidden=${childNode.hidden}, inVisibleNodes=${this._collections._visibleNodes.has(childId)}`);
      }
      if (childContainer) {
        console.log(`[DEBUG]   Child container ${childId}: hidden=${childContainer.hidden}, inVisibleContainers=${this._collections._visibleContainers.has(childId)}`);
      }
    }
    
    // HYPOTHESIS 2 TEST: Check ELK visibility cache state
    console.log(`[DEBUG] Final visibility state: visibleNodes=${this._collections._visibleNodes.size}, visibleContainers=${this._collections._visibleContainers.size}, hyperEdges=${this._collections.hyperEdges.size}`);
    
    // Validate hyperEdge endpoints and routing after all updates
    this._validateHyperEdgeEndpoints();
    this._validateHyperEdgeLifting();
  }
  
  /**
   * Validate that hyperEdges don't have hidden endpoints
   * This is a validation-only method that doesn't mutate state
   * (Replaces the old _cleanupDanglingHyperEdges which was doing cleanup during operations)
   */
  private _validateHyperEdgeEndpoints(): void {
    const violations = [];
    
    for (const [hyperEdgeId, hyperEdge] of this._collections.hyperEdges) {
      // Skip already hidden hyperEdges
      if (hyperEdge.hidden) continue;
      
      // Check if endpoints exist and are visible
      const sourceContainer = this._collections.containers.get(hyperEdge.source);
      const targetContainer = this._collections.containers.get(hyperEdge.target);
      const sourceNode = this._collections.graphNodes.get(hyperEdge.source);
      const targetNode = this._collections.graphNodes.get(hyperEdge.target);
      
      const sourceExists = sourceContainer || sourceNode;
      const targetExists = targetContainer || targetNode;
      
      // An endpoint should be hidden if:
      // 1. It doesn't exist 
      // 2. It's a container/node explicitly marked as hidden (hidden=true)
      // 3. It's a node that belongs to a collapsed container (since nodes inside collapsed containers should be hidden)
      
      let sourceEffectivelyHidden = !sourceExists || 
        (sourceContainer?.hidden === true) || 
        (sourceNode?.hidden === true);
        
      let targetEffectivelyHidden = !targetExists || 
        (targetContainer?.hidden === true) || 
        (targetNode?.hidden === true);
      
      // Special case: If source/target is a node, check if its parent container is collapsed
      // If so, the node should be considered hidden (not visible in the final layout)
      if (sourceNode && !sourceEffectivelyHidden) {
        const parentContainerId = this.getNodeContainer(sourceNode.id);
        const parentContainer = parentContainerId ? this._collections.containers.get(parentContainerId) : null;
        if (parentContainer && parentContainer.collapsed) {
          sourceEffectivelyHidden = true;  // Node inside collapsed container is effectively hidden
        }
      }
      
      if (targetNode && !targetEffectivelyHidden) {
        const parentContainerId = this.getNodeContainer(targetNode.id);
        const parentContainer = parentContainerId ? this._collections.containers.get(parentContainerId) : null;
        if (parentContainer && parentContainer.collapsed) {
          targetEffectivelyHidden = true;  // Node inside collapsed container is effectively hidden
        }
      }
      
      // Report validation issues (but don't mutate state)
      if (sourceEffectivelyHidden || targetEffectivelyHidden) {
        violations.push(`HyperEdge ${hyperEdgeId} has hidden endpoints (source: ${sourceEffectivelyHidden}, target: ${targetEffectivelyHidden})`);
      }
    }
    
    // Log validation issues but don't fix them - that should be done elsewhere
    if (violations.length > 0) {
      console.warn(`[VisState] HyperEdge validation issues found: ${violations.join(', ')}`);
    }
  }
  
  /**
   * Update existing hyperEdges that now have invalid endpoints due to container collapse
   * This fixes the core issue where hyperEdges point to hidden entities after a collapse operation
   */
  private _updateInvalidatedHyperEdges(newlyCollapsedContainerId: string): void {
    const updatedHyperEdges: Array<{oldId: string, newHyperEdge: any}> = [];
    const toDelete: string[] = [];
    
    console.log(`[HYPEREDGE_LIFTING] Checking existing hyperEdges for invalidation due to collapse of ${newlyCollapsedContainerId}`);
    
    for (const [hyperEdgeId, hyperEdge] of this._collections.hyperEdges) {
      if (hyperEdge.hidden) continue;
      
      // Check if either endpoint is now invalid (hidden or doesn't exist)
      const sourceContainer = this._collections.containers.get(hyperEdge.source);
      const sourceNode = this._collections.graphNodes.get(hyperEdge.source);
      const targetContainer = this._collections.containers.get(hyperEdge.target);
      const targetNode = this._collections.graphNodes.get(hyperEdge.target);
      
      const sourceExists = sourceContainer || sourceNode;
      const targetExists = targetContainer || targetNode;
      
      if (!sourceExists || !targetExists) {
        console.warn(`[HYPEREDGE_LIFTING] Removing hyperEdge ${hyperEdgeId} - endpoint doesn't exist (source: ${!!sourceExists}, target: ${!!targetExists})`);
        toDelete.push(hyperEdgeId);
        continue;
      }
      
      // Check if endpoints are effectively hidden
      const sourceHidden = (sourceContainer?.hidden) || (sourceNode?.hidden) || 
                          (sourceNode && this._isNodeInCollapsedContainer(hyperEdge.source));
      const targetHidden = (targetContainer?.hidden) || (targetNode?.hidden) || 
                          (targetNode && this._isNodeInCollapsedContainer(hyperEdge.target));
      
      let needsUpdate = false;
      let newSource = hyperEdge.source;
      let newTarget = hyperEdge.target;
      
      // If source is hidden/invalid, find its visible ancestor
      if (sourceHidden) {
        const visibleAncestor = this._findLowestVisibleAncestor(hyperEdge.source);
        if (visibleAncestor !== hyperEdge.source) {
          console.log(`[HYPEREDGE_LIFTING] Source ${hyperEdge.source} is hidden, lifting to ancestor ${visibleAncestor}`);
          newSource = visibleAncestor;
          needsUpdate = true;
        }
      }
      
      // If target is hidden/invalid, find its visible ancestor
      if (targetHidden) {
        const visibleAncestor = this._findLowestVisibleAncestor(hyperEdge.target);
        if (visibleAncestor !== hyperEdge.target) {
          console.log(`[HYPEREDGE_LIFTING] Target ${hyperEdge.target} is hidden, lifting to ancestor ${visibleAncestor}`);
          newTarget = visibleAncestor;
          needsUpdate = true;
        }
      }
      
      if (needsUpdate) {
        // Check if the new routing would create a self-loop
        if (newSource === newTarget) {
          console.log(`[HYPEREDGE_LIFTING] Removing hyperEdge ${hyperEdgeId} - would create self-loop ${newSource} -> ${newTarget}`);
          toDelete.push(hyperEdgeId);
          continue;
        }
        
        // Create new hyperEdge with updated endpoints
        const newHyperEdgeId = `${HYPEREDGE_CONSTANTS.PREFIX}${newSource}${HYPEREDGE_CONSTANTS.SEPARATOR}${newTarget}`;
        
        // Check if this hyperEdge already exists
        if (this._collections.hyperEdges.has(newHyperEdgeId)) {
          console.log(`[HYPEREDGE_LIFTING] HyperEdge ${newHyperEdgeId} already exists, removing duplicate ${hyperEdgeId}`);
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
        console.log(`[HYPEREDGE_LIFTING] Lifting hyperEdge ${hyperEdgeId} to ${newHyperEdgeId} (${newSource} -> ${newTarget})`);
      }
    }
    
    // Apply all updates atomically
    for (const { oldId, newHyperEdge } of updatedHyperEdges) {
      // Remove old hyperEdge
      this._collections.hyperEdges.delete(oldId);
      
      // Add new hyperEdge
      this._collections.hyperEdges.set(newHyperEdge.id, newHyperEdge);
      
      // Update node-to-edges mapping
      const sourceEdges = this._collections.nodeToEdges.get(newHyperEdge.source) || new Set();
      sourceEdges.delete(oldId);  // Remove old mapping
      sourceEdges.add(newHyperEdge.id);
      this._collections.nodeToEdges.set(newHyperEdge.source, sourceEdges);
      
      const targetEdges = this._collections.nodeToEdges.get(newHyperEdge.target) || new Set();
      targetEdges.delete(oldId);  // Remove old mapping
      targetEdges.add(newHyperEdge.id);
      this._collections.nodeToEdges.set(newHyperEdge.target, targetEdges);
    }
    
    // Delete hyperEdges that couldn't be lifted
    for (const hyperEdgeId of toDelete) {
      this._collections.hyperEdges.delete(hyperEdgeId);
      console.log(`[HYPEREDGE_LIFTING] Deleted unrecoverable hyperEdge ${hyperEdgeId}`);
    }
    
    console.log(`[HYPEREDGE_LIFTING] Completed lifting: ${updatedHyperEdges.length} updated, ${toDelete.length} deleted`);
  }
  
  /**
   * Validate that hyperEdge lifting was performed correctly
   * This is a new validation specifically for the lifting concern
   */
  private _validateHyperEdgeLifting(): void {
    const violations = [];
    
    for (const [hyperEdgeId, hyperEdge] of this._collections.hyperEdges) {
      if (hyperEdge.hidden) continue;
      
      // Validate that no hyperEdge points to hidden entities
      const sourceContainer = this._collections.containers.get(hyperEdge.source);
      const sourceNode = this._collections.graphNodes.get(hyperEdge.source);
      const targetContainer = this._collections.containers.get(hyperEdge.target);
      const targetNode = this._collections.graphNodes.get(hyperEdge.target);
      
      // Check for hidden sources
      const sourceHidden = (sourceContainer?.hidden) || (sourceNode?.hidden) || 
                          (sourceNode && this._isNodeInCollapsedContainer(hyperEdge.source));
      
      // Check for hidden targets
      const targetHidden = (targetContainer?.hidden) || (targetNode?.hidden) || 
                          (targetNode && this._isNodeInCollapsedContainer(hyperEdge.target));
      
      if (sourceHidden) {
        violations.push(`HyperEdge ${hyperEdgeId} source ${hyperEdge.source} should have been lifted - it's hidden/invalid`);
      }
      
      if (targetHidden) {
        violations.push(`HyperEdge ${hyperEdgeId} target ${hyperEdge.target} should have been lifted - it's hidden/invalid`);
      }
      
      // Additional check: Both endpoints should exist
      if (!sourceContainer && !sourceNode) {
        violations.push(`HyperEdge ${hyperEdgeId} source ${hyperEdge.source} doesn't exist`);
      }
      
      if (!targetContainer && !targetNode) {
        violations.push(`HyperEdge ${hyperEdgeId} target ${hyperEdge.target} doesn't exist`);
      }
    }
    
    // Log validation issues but don't fix them - lifting should have already handled this
    if (violations.length > 0) {
      console.error(`[VisState] CRITICAL: HyperEdge lifting validation failed: ${violations.join(', ')}`);
      throw new Error(`HyperEdge lifting validation failed: ${violations.length} violations found`);
    } else {
      console.log(`[HYPEREDGE_LIFTING] Validation passed: All hyperEdges have valid endpoints`);
    }
  }
  
  /**
   * Helper method to check if a node is inside a collapsed container
   */
  private _isNodeInCollapsedContainer(nodeId: string): boolean {
    const parentContainerId = this.getNodeContainer(nodeId);
    if (!parentContainerId) return false;
    
    const parentContainer = this._collections.containers.get(parentContainerId);
    return parentContainer?.collapsed === true;
  }
  
  /**
   * Find the lowest visible ancestor for a node/container ID
   * This is used when routing hyperEdges to ensure they connect to visible entities
   * 
   * IMPORTANT: This method must return the final visible entity that will be accessible
   * after all collapse operations complete. During smart collapse, intermediate containers
   * may get hidden when their parents are also collapsed.
   */
  private _findLowestVisibleAncestor(entityId: string): string {
    // Check if entity exists
    const container = this._collections.containers.get(entityId);
    const node = this._collections.graphNodes.get(entityId);
    
    if (!container && !node) {
      return entityId; // Entity doesn't exist - let validation catch this
    }
    
    // For containers: check if it will remain visible after potential parent collapses
    if (container) {
      // If container is hidden, find its visible parent
      if (container.hidden) {
        const parentId = this._findContainerParent(entityId);
        if (parentId) {
          return this._findLowestVisibleAncestor(parentId); // Recurse up the hierarchy
        }
        return entityId; // No parent - this will be caught by validation
      }
      
      // Container is currently visible, but check if it will be hidden by parent collapse
      const parentId = this._findContainerParent(entityId);
      if (parentId) {
        const parentContainer = this._collections.containers.get(parentId);
        if (parentContainer && parentContainer.collapsed) {
          // Parent is collapsed, so this container should be hidden
          return this._findLowestVisibleAncestor(parentId);
        }
      }
      
      // Container is and will remain visible
      return entityId;
    }
    
    // For nodes: trace up to find the visible container ancestor
    if (node) {
      // If node is directly hidden, find its container
      if (node.hidden) {
        const containerId = this.getNodeContainer(entityId);
        if (containerId) {
          return this._findLowestVisibleAncestor(containerId);
        }
        return entityId; // No container - this will be caught by validation
      }
      
      // Node is not directly hidden, but check if it's inside a collapsed container
      const containerId = this.getNodeContainer(entityId);
      if (containerId) {
        const parentContainer = this._collections.containers.get(containerId);
        if (parentContainer && parentContainer.collapsed) {
          // Node is inside a collapsed container - route to the container
          return this._findLowestVisibleAncestor(containerId);
        }
      }
      
      // Node is visible and not inside a collapsed container
      return entityId;
    }
    
    return entityId;
  }
  
  /**
   * Find the parent container for a given container
   * Returns the container ID that contains this container as a child
   */
  private _findContainerParent(containerId: string): string | undefined {
    for (const [parentId, children] of this._collections.containerChildren) {
      if (children.has(containerId)) {
        return parentId;
      }
    }
    return undefined;
  }
  
  /**
   * Prepare hyperEdges for a collapsed container
   */
  private _prepareHyperedges(containerId: string, crossingEdges: any[]): any[] {
    const children = this.getContainerChildren(containerId);
    const edgeGroups = new Map<string, { incoming: any[], outgoing: any[] }>();

    // Group edges by external endpoint (routed to lowest visible ancestor)
    for (const edge of crossingEdges) {
      const sourceInContainer = children.has(edge.source);
      const rawExternalEndpoint = sourceInContainer ? edge.target : edge.source;
      
      // CRITICAL: Route the external endpoint to its lowest visible ancestor
      // This ensures hyperEdges connect to visible containers, not hidden internal nodes
      const externalEndpoint = this._findLowestVisibleAncestor(rawExternalEndpoint);
      
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
    
    for (const [externalEndpoint, group] of edgeGroups) {
      // Validate that the external endpoint exists and is visible
      const endpointExists = this._collections.graphNodes.has(externalEndpoint) || 
                           this._collections.containers.has(externalEndpoint);
      const endpointVisible = this._isNodeOrContainerVisible(externalEndpoint);
      
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
        hyperEdges.push(this._createHyperedgeObject(hyperEdgeId, externalEndpoint, containerId, group.incoming, containerId));
      }

      // Create hyperedge for outgoing connections (container -> external)
      if (group.outgoing.length > 0) {
        const hyperEdgeId = `${HYPEREDGE_CONSTANTS.PREFIX}${containerId}${HYPEREDGE_CONSTANTS.SEPARATOR}${externalEndpoint}`;
        hyperEdges.push(this._createHyperedgeObject(hyperEdgeId, containerId, externalEndpoint, group.outgoing, containerId));
      }
    }

    return hyperEdges;
  }
  
  /**
   * Create a hyperEdge object from original edges
   */
  private _createHyperedgeObject(hyperEdgeId: string, source: string, target: string, originalEdges: any[], collapsedContainerId: string): any {
    // Store original edge information for restoration
    const originalEndpoints = new Map();
    for (const edge of originalEdges) {
      originalEndpoints.set(edge.id, {
        source: edge.source,
        target: edge.target
      });
    }

    // Use the highest priority style from the aggregated edges
    const style = this._aggregateStyles(originalEdges);

    return {
      id: hyperEdgeId,
      source,
      target,
      style,
      originalEndpoints,
      hidden: false,
      // Mark this hyperEdge as created for container collapse
      createdForCollapse: true,
      // Track which container this hyperEdge was created for
      collapsedContainerId: collapsedContainerId
    };
  }
  
  /**
   * Aggregate styles from multiple edges (highest priority wins)
   */
  private _aggregateStyles(edges: any[]): string {
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
  
  private _handleContainerExpansion(containerId: string): void {
    // Clean up hyperEdges related to this container FIRST
    this._cleanupHyperEdgesForExpansion(containerId);
    
    // Then show immediate children
    this._showImmediateChildren(containerId);
    
    // CRITICAL: After expansion, create new hyperEdges from revealed nodes to still-collapsed containers
    this._createHyperEdgesForExpandedContainer(containerId);
  }
  
  /**
   * Clean up hyperEdges when a container is expanded
   * HyperEdges that were created when this container was collapsed should be removed
   * and the original edges should be restored
   */
  private _cleanupHyperEdgesForExpansion(containerId: string): void {
    const hyperEdgesToRemove = [];
    
    // Find hyperEdges that were specifically created for this container's collapse
    // Use the property instead of string parsing for robustness
    for (const [hyperEdgeId, hyperEdge] of this._collections.hyperEdges) {
      // Check if this hyperEdge was created for container collapse and involves this container
      if (hyperEdge.createdForCollapse && hyperEdge.collapsedContainerId === containerId) {
        hyperEdgesToRemove.push(hyperEdgeId);
      }
    }
    
    // Remove the hyperEdges
    for (const hyperEdgeId of hyperEdgesToRemove) {
      const hyperEdge = this._collections.hyperEdges.get(hyperEdgeId);
      if (hyperEdge) {
        this._collections.hyperEdges.delete(hyperEdgeId);
        
        // Clean up node-to-edges mapping
        this._removeFromNodeToEdges(hyperEdge.source, hyperEdgeId);
        this._removeFromNodeToEdges(hyperEdge.target, hyperEdgeId);
        
        console.log(`[DEBUG] Removed hyperEdge during expansion: ${hyperEdgeId}`);
      }
    }
    
    // Restore original edges that were hidden during collapse
    // Find edges that should be restored
    for (const [edgeId, edge] of this._collections.graphEdges) {
      if (edge.hidden) {
        // Check if this edge should be restored (both endpoints are now visible)
        const sourceVisible = this._isNodeOrContainerVisible(edge.source);
        const targetVisible = this._isNodeOrContainerVisible(edge.target);
        
        if (sourceVisible && targetVisible) {
          // Restore the edge
          edge.hidden = false;
          this._collections._visibleEdges.set(edgeId, edge);
          console.log(`[DEBUG] Restored edge during expansion: ${edgeId}`);
        }
      }
    }
  }
  
  /**
   * Create hyperEdges from newly revealed nodes to still-collapsed containers
   * This is called after expanding a container to maintain connectivity
   */
  private _createHyperEdgesForExpandedContainer(expandedContainerId: string): void {
    const expandedContainer = this._collections.containers.get(expandedContainerId);
    if (!expandedContainer) return;
    
    // Get all children of the expanded container
    const children = this._collections.containerChildren.get(expandedContainerId) || new Set();
    
    // For each child node, check if it has edges to nodes in collapsed containers
    for (const childId of children) {
      const childNode = this._collections.graphNodes.get(childId);
      if (!childNode || childNode.hidden) continue;
      
      // Get all edges from this child node
      const childEdges = this._collections.nodeToEdges.get(childId) || new Set();
      
      for (const edgeId of childEdges) {
        const edge = this._collections.graphEdges.get(edgeId);
        if (!edge || !edge.hidden) continue; // Only process hidden edges
        
        // Find the other endpoint of this edge
        const otherEndpoint = edge.source === childId ? edge.target : edge.source;
        
        // Check if the other endpoint is in a collapsed container
        const otherParentContainer = this._collections.nodeContainers.get(otherEndpoint);
        if (otherParentContainer) {
          const otherContainer = this._collections.containers.get(otherParentContainer);
          if (otherContainer && otherContainer.collapsed && !otherContainer.hidden) {
            // Create a hyperEdge from this child node to the collapsed container
            const hyperEdgeId = `hyper_${childId}_to_${otherParentContainer}`;
            
            // Don't create duplicate hyperEdges
            if (this._collections.hyperEdges.has(hyperEdgeId)) continue;
            
            const hyperEdge = {
              id: hyperEdgeId,
              source: childId,
              target: otherParentContainer,
              style: edge.style || 'default', // Use the original edge's style
              hidden: false,
              createdForCollapse: true,
              collapsedContainerId: otherParentContainer, // The container that's still collapsed
              originalEdges: [edgeId]
            };
            
            this._collections.hyperEdges.set(hyperEdgeId, hyperEdge);
            
            // Update node-to-edges mapping using the same pattern as existing code
            const sourceEdges = this._collections.nodeToEdges.get(childId) || new Set();
            sourceEdges.add(hyperEdgeId);
            this._collections.nodeToEdges.set(childId, sourceEdges);
            
            const targetEdges = this._collections.nodeToEdges.get(otherParentContainer) || new Set();
            targetEdges.add(hyperEdgeId);
            this._collections.nodeToEdges.set(otherParentContainer, targetEdges);
            
            console.log(`[DEBUG] Created hyperEdge for expansion: ${hyperEdgeId} (${childId} -> ${otherParentContainer})`);
          }
        }
        
        // Also handle the reverse direction (from collapsed container to this child)
        if (edge.target === childId) {
          const sourceParentContainer = this._collections.nodeContainers.get(edge.source);
          if (sourceParentContainer) {
            const sourceContainer = this._collections.containers.get(sourceParentContainer);
            if (sourceContainer && sourceContainer.collapsed && !sourceContainer.hidden) {
              const hyperEdgeId = `hyper_${sourceParentContainer}_to_${childId}`;
              
              // Don't create duplicate hyperEdges
              if (this._collections.hyperEdges.has(hyperEdgeId)) continue;
              
              const hyperEdge = {
                id: hyperEdgeId,
                source: sourceParentContainer,
                target: childId,
                style: edge.style || 'default', // Use the original edge's style
                hidden: false,
                createdForCollapse: true,
                collapsedContainerId: sourceParentContainer,
                originalEdges: [edgeId]
              };
              
              this._collections.hyperEdges.set(hyperEdgeId, hyperEdge);
              
              // Update node-to-edges mapping
              const sourceEdges = this._collections.nodeToEdges.get(sourceParentContainer) || new Set();
              sourceEdges.add(hyperEdgeId);
              this._collections.nodeToEdges.set(sourceParentContainer, sourceEdges);
              
              const targetEdges = this._collections.nodeToEdges.get(childId) || new Set();
              targetEdges.add(hyperEdgeId);
              this._collections.nodeToEdges.set(childId, targetEdges);
              
              console.log(`[DEBUG] Created hyperEdge for expansion: ${hyperEdgeId} (${sourceParentContainer} -> ${childId})`);
            }
          }
        }
      }
    }
  }
  
  /**
   * Helper to remove edge from node-to-edges mapping
   */
  private _removeFromNodeToEdges(nodeId: string, edgeId: string): void {
    const edges = this._collections.nodeToEdges.get(nodeId);
    if (edges) {
      edges.delete(edgeId);
      if (edges.size === 0) {
        this._collections.nodeToEdges.delete(nodeId);
      }
    }
  }
  
  /**
   * Check if a node or container is currently visible
   */
  private _isNodeOrContainerVisible(entityId: string): boolean {
    // Check if it's a visible node
    if (this._collections._visibleNodes.has(entityId)) {
      return true;
    }
    
    // Check if it's a visible container
    if (this._collections._visibleContainers.has(entityId)) {
      return true;
    }
    
    return false;
  }
  
  private _cascadeContainerVisibility(containerId: string, visible: boolean): void {
    if (!visible) {
      // When hiding container, hide all descendants
      this._hideAllDescendants(containerId);
    }
    // Note: When showing container, we don't automatically show descendants
    // They may have been individually hidden
  }
  
  private _hideAllDescendants(containerId: string): void {
    const children = this._collections.containerChildren.get(containerId) || new Set();
    
    for (const childId of Array.from(children)) {
      // Check if it's a container or node
      const childContainer = this._collections.containers.get(childId);
      if (childContainer) {
        // First, recursively collapse and hide ALL descendants of the child container
        this._hideAllDescendants(childId);
        
        // THEN mark the child container as collapsed and hidden
        childContainer.collapsed = true; // Must be collapsed  
        childContainer.hidden = true;   // Must be hidden
        
        // Update visibility caches
        this._updateContainerVisibilityCaches(childId, childContainer);
      } else {
        // If it's a node, hide it directly without triggering validateInvariants
        const node = this._collections.graphNodes.get(childId);
        if (node) {
          node.hidden = true;
          this._collections._visibleNodes.delete(childId);
          
          // Cascade to connected edges
          this._cascadeNodeVisibilityToEdges(childId, false);
        }
      }
    }
  }
  
  private _showImmediateChildren(containerId: string): void {
    const children = this._collections.containerChildren.get(containerId) || new Set();
    
    for (const childId of Array.from(children)) {
      // Check if it's a container or node
      const childContainer = this._collections.containers.get(childId);
      if (childContainer) {
        // Show and expand child containers by default when parent is expanded
        this.setContainerState(childId, { hidden: false, collapsed: false });
      } else {
        this.setNodeVisibility(childId, true);
      }
    }
  }
  
  private _isEndpointVisible(endpointId: string): boolean {
    // Check if it's a visible node
    const node = this._collections.graphNodes.get(endpointId);
    if (node) return !node.hidden;
    
    // Check if it's a visible container (collapsed containers are visible)
    const container = this._collections.containers.get(endpointId);
    if (container) return !container.hidden;
    
    return false;
  }

  /**
   * Validate all VisualizationState invariants
   * Called by public APIs to ensure state consistency
   */
  validateInvariants(): void {
    this.invariantValidator.validateInvariants();
  }

  /**
   * Alias for validateInvariants (backwards compatibility)
   */
  validateAllInvariants(context?: string): void {
    if (context) {
      console.log(`[VisState] Validating invariants: ${context}`);
    }
    this.validateInvariants();
  }

  // ============ Bridge Support Methods (Backwards Compatibility) ============

  /**
   * Get parent-child mapping for ReactFlow bridge
   */
  getParentChildMap(): Map<string, string> {
    const parentMap = new Map<string, string>();
    
    // Map visible nodes to their expanded parent containers
    for (const node of this.visibleNodes) {
      const parentContainer = this._collections.nodeContainers.get(node.id);
      if (parentContainer) {
        const container = this._collections.containers.get(parentContainer);
        // Only include if parent container is expanded (visible)
        if (container && !container.collapsed && this._collections._expandedContainers.has(parentContainer)) {
          parentMap.set(node.id, parentContainer);
        }
      }
    }
    
    // Also handle containers defined with children arrays (for test compatibility)
    for (const [containerId, container] of this._collections.containers) {
      if (!container.collapsed && !container.hidden && container.children) {
        for (const childId of container.children) {
          parentMap.set(childId, containerId);
        }
      }
    }
    
    // Also map visible containers to their parent containers
    // We need to check which container has this container as a child
    for (const container of this.visibleContainers) {
      for (const [parentId, children] of this._collections.containerChildren) {
        if (children.has(container.id)) {
          const parentObj = this._collections.containers.get(parentId);
          // Only include if parent container is expanded (visible)
          if (parentObj && !parentObj.collapsed && this._collections._expandedContainers.has(parentId)) {
            parentMap.set(container.id, parentId);
            break;
          }
        }
      }
    }
    
    return parentMap;
  }

  /**
   * Get edge handles for ReactFlow bridge
   */
  getEdgeHandles(edgeId: string): { sourceHandle?: string; targetHandle?: string } {
    const edge = this._collections.graphEdges.get(edgeId);
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
    
    for (const container of this._collections.containers.values()) {
      if (container.collapsed && !container.hidden) {
        // Convert collapsed container to node format for ELK
        collapsedAsNodes.push({
          id: container.id,
          label: container.label || container.id,
          width: container.width || 200,
          height: container.height || 150,
          x: container.x || 0,
          y: container.y || 0,
          hidden: false,
          style: container.style || 'default'
          // Note: No 'type' field for bridge migration compatibility
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
    // For fresh layout, all containers should have elkFixed=false
    return this.visibleContainers.map(container => ({
      ...container,
      elkFixed: false // Ensure fresh layout
    }));
  }

  /**
   * Get top-level nodes (not inside any expanded container)
   */
  getTopLevelNodes(): ReadonlyArray<any> {
    const topLevelNodes = [];
    
    for (const node of this.visibleNodes) {
      const parentContainer = this._collections.nodeContainers.get(node.id);
      
      if (!parentContainer) {
        // Node has no parent container, so it's top-level
        topLevelNodes.push(node);
      } else {
        const container = this._collections.containers.get(parentContainer);
        // If parent container is collapsed or hidden, the node is effectively top-level
        if (!container || container.collapsed || container.hidden || !this._collections._expandedContainers.has(parentContainer)) {
          topLevelNodes.push(node);
        }
      }
    }
    
    return topLevelNodes;
  }

  /**
   * Set hyperEdge (backwards compatibility)
   */
  setHyperEdge(hyperEdgeId: string, hyperEdgeData: any): this {
    const processedData = { ...hyperEdgeData, id: hyperEdgeId };
    this._collections.hyperEdges.set(hyperEdgeId, processedData);
    return this;
  }

  /**
   * Add a child to a container (needed by JSONParser)
   */
  addContainerChild(containerId: string, childId: string): void {
    // Update the container's children set
    const children = this._collections.containerChildren.get(containerId) || new Set();
    children.add(childId);
    this._collections.containerChildren.set(containerId, children);
    
    // Update the node-to-container mapping
    this._collections.nodeContainers.set(childId, containerId);
    
    // Update the container object if it exists
    const container = this._collections.containers.get(containerId);
    if (container) {
      if (!container.children) {
        container.children = new Set();
      }
      container.children.add(childId);
    }
  }

  /**
   * Validate and fix invalid dimensions
   */
  validateAndFixDimensions(): void {
    let fixedCount = 0;
    
    // Fix node dimensions
    for (const [nodeId, node] of this._collections.graphNodes) {
      let needsUpdate = false;
      const updates: any = {};
      
      if (!node.width || node.width <= 0) {
        updates.width = 180; // LAYOUT_CONSTANTS.DEFAULT_NODE_WIDTH
        needsUpdate = true;
      }
      
      if (!node.height || node.height <= 0) {
        updates.height = 60; // LAYOUT_CONSTANTS.DEFAULT_NODE_HEIGHT
        needsUpdate = true;
      }
      
      if (needsUpdate) {
        Object.assign(node, updates);
        fixedCount++;
      }
    }
    
    // Fix container dimensions
    for (const [containerId, container] of this._collections.containers) {
      let needsUpdate = false;
      const updates: any = {};
      
      if (!container.width || container.width <= 0) {
        updates.width = 200; // LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH
        needsUpdate = true;
      }
      
      if (!container.height || container.height <= 0) {
        updates.height = 150; // LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT
        needsUpdate = true;
      }
      
      if (needsUpdate) {
        Object.assign(container, updates);
        fixedCount++;
      }
    }
    
    if (fixedCount > 0) {
      console.log(`[VisState] Fixed ${fixedCount} invalid dimensions`);
    }
  }

  /**
   * Get edge layout information (sections, routing)
   */
  getEdgeLayout(edgeId: string): { sections?: any[]; [key: string]: any } | undefined {
    const edge = this._collections.graphEdges.get(edgeId);
    if (!edge) return undefined;
    
    return {
      sections: edge.sections || [],
      // Include any other layout properties
      ...edge
    };
  }

  /**
   * Set edge layout information
   */
  setEdgeLayout(edgeId: string, layout: { sections?: any[]; [key: string]: any }): void {
    const edge = this._collections.graphEdges.get(edgeId);
    if (!edge) return;
    
    Object.assign(edge, layout);
  }

  /**
   * Get node visibility state (for tests)
   */
  getNodeVisibility(nodeId: string): { hidden?: boolean } {
    const node = this._collections.graphNodes.get(nodeId);
    if (!node) return {};
    
    return {
      hidden: node.hidden || !this._collections._visibleNodes.has(nodeId)
    };
  }

  /**
   * Get edge visibility state (for tests)  
   */
  getEdgeVisibility(edgeId: string): { hidden?: boolean } {
    const edge = this._collections.graphEdges.get(edgeId);
    if (!edge) return {};
    
    return {
      hidden: edge.hidden || !this.visibleEdges.some(e => e.id === edgeId)
    };
  }
}

/**
 * Create factory function for VisualizationState
 */
export function createVisualizationState(): VisualizationState {
  return new VisualizationState();
}
