/**
 * Visualization State - Core Data Structure (Refactored)
 * 
 * Maintains the mutable state of the visualization including nodes, edges, containers, and hyperEdges.
 * Provides efficient access to visible/non-hidden elements through Maps and collections.
 * 
 * This refactored version delegates to specialized operation classes for better maintainability.
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
import { LAYOUT_CONSTANTS, HYPEREDGE_CONSTANTS, SIZES } from '../shared/config';
import { ContainerPadding } from './ContainerPadding';

// Import specialized operation classes
import { VisualizationStateInvariantValidator } from './validation/VisualizationStateValidator';
import { ContainerOperations } from './operations/ContainerOperations';
import { VisibilityManager } from './operations/VisibilityManager';
import { LayoutOperations } from './operations/LayoutOperations';
import { BridgeCompatibility } from './compatibility/BridgeCompatibility';

// Simple assertion function that works in both Node.js and browser environments
function assert(condition: any, message?: string): asserts condition {
  if (!condition) {
    throw new Error(message || 'Assertion failed');
  }
}

// Constants for consistent string literals
const DEFAULT_STYLE = 'default';

/**
 * Read-only interface for container hierarchy information
 * Used by external systems that need hierarchy access without mutation capabilities
 */
export interface ContainerHierarchyView {
  getContainerChildren(containerId: string): ReadonlySet<string>;
  getNodeContainer(nodeId: string): string | undefined;
}

/**
 * Core visualization state class that manages all graph elements including nodes, edges, 
 * containers, and hyperEdges. Provides both mutable APIs for state management and 
 * read-only APIs for rendering systems.
 * 
 * Key Design Principles:
 * - Encapsulated state with controlled access via getters/setters
 * - Automatic consistency maintenance through invariant validation
 * - Separation of concerns through specialized operation classes
 * - Bridge pattern for backwards compatibility with existing systems
 * 
 * @class VisualizationState
 * 
 * @example
 * ```typescript
 * const state = new VisualizationState();
 * state.addGraphNode('node1', { type: 'operator', style: 'default' });
 * state.addContainer('container1', { children: ['node1'] });
 * state.setContainerState('container1', { collapsed: true });
 * ```
 */
export class VisualizationState implements ContainerHierarchyView {
  // Protected state collections - NEVER access directly!
  private readonly _collections = {
    graphNodes: new Map<string, any>(),
    graphEdges: new Map<string, any>(),
    containers: new Map<string, any>(),
    hyperEdges: new Map<string, any>(),
    _visibleNodes: new Map<string, any>(),
    _visibleEdges: new Map<string, any>(),
    _visibleContainers: new Map<string, any>(),
    _expandedContainers: new Map<string, any>(),
    collapsedContainers: new Map<string, any>(),
    nodeToEdges: new Map<string, Set<string>>(),
    manualPositions: new Map<string, {x: number, y: number}>(),
    containerChildren: new Map<string, Set<string>>(),
    nodeContainers: new Map<string, string>()
  };
  
  // Specialized operation classes
  private readonly invariantValidator: VisualizationStateInvariantValidator;
  private readonly containerOps: ContainerOperations;
  private readonly visibilityManager: VisibilityManager;
  private readonly layoutOps: LayoutOperations;
  private readonly bridgeCompat: BridgeCompatibility;

  // Track containers in transition state to suppress spurious warnings
  private readonly _recentlyCollapsedContainers = new Set<string>();

  // Flag to track recursive operations
  private _inRecursiveOperation = false;

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
    // Initialize specialized operation classes
    this.invariantValidator = new VisualizationStateInvariantValidator(this);
    this.containerOps = new ContainerOperations(this);
    this.visibilityManager = new VisibilityManager(this);
    this.layoutOps = new LayoutOperations(this);
    this.bridgeCompat = new BridgeCompatibility(this);
  }

  // ============ SAFE BRIDGE API (Read-only access for external systems) ============
  
  /**
   * Get visible nodes for rendering (safe read-only access)
   * Bridges should ONLY use this method, never access internal maps directly
   */
  get visibleNodes(): ReadonlyArray<any> {
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
      return !edge.hidden;
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
      const adjustedDimensions = this.layoutOps.getContainerAdjustedDimensions(container.id);
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
      return !edge.hidden;
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

  // ============ LAYOUT API (Delegate to LayoutOperations) ============
  
  getAllManualPositions(): Map<string, { x: number; y: number }> {
    return this.layoutOps.getAllManualPositions();
  }

  hasAnyManualPositions(): boolean {
    return this.layoutOps.hasAnyManualPositions();
  }

  setManualPosition(entityId: string, x: number, y: number): void {
    this.layoutOps.setManualPosition(entityId, x, y);
  }

  setContainerLayout(containerId: string, layout: any): void {
    this.layoutOps.setContainerLayout(containerId, layout);
  }
  
  setNodeLayout(nodeId: string, layout: any): void {
    this.layoutOps.setNodeLayout(nodeId, layout);
  }

  getContainerLayout(containerId: string): { position?: { x: number; y: number }; dimensions?: { width: number; height: number } } | undefined {
    return this.layoutOps.getContainerLayout(containerId);
  }

  getNodeLayout(nodeId: string): { position?: { x: number; y: number }; dimensions?: { width: number; height: number } } | undefined {
    return this.layoutOps.getNodeLayout(nodeId);
  }

  getContainerAdjustedDimensions(containerId: string): { width: number; height: number } {
    return this.layoutOps.getContainerAdjustedDimensions(containerId);
  }

  clearLayoutPositions(): void {
    this.layoutOps.clearLayoutPositions();
  }

  validateAndFixDimensions(): void {
    this.layoutOps.validateAndFixDimensions();
  }

  getEdgeLayout(edgeId: string): { sections?: any[]; [key: string]: any } | undefined {
    return this.layoutOps.getEdgeLayout(edgeId);
  }

  setEdgeLayout(edgeId: string, layout: { sections?: any[]; [key: string]: any }): void {
    this.layoutOps.setEdgeLayout(edgeId, layout);
  }

  // ============ CORE API - Direct Entity Management ============
  
  /**
   * Get a graph node by ID (core API)
   */
  getGraphNode(nodeId: string): any | undefined {
    return this._collections.graphNodes.get(nodeId);
  }
  
  /**
   * Get a graph edge by ID (core API)
   */
  getGraphEdge(edgeId: string): any | undefined {
    return this._collections.graphEdges.get(edgeId);
  }
  
  /**
   * Get a container by ID (core API)
   */
  getContainer(containerId: string): any | undefined {
    return this._collections.containers.get(containerId);
  }

  /**
   * Get a hyperEdge by ID
   */
  getHyperEdge(hyperEdgeId: string): any | undefined {
    return this._collections.hyperEdges.get(hyperEdgeId);
  }
  
  /**
   * Add a graph node directly (for JSONParser and initial data loading)
   */
  addGraphNode(nodeId: string, nodeData: any): void {
    // Check if node belongs to a collapsed container and should be hidden
    const parentContainer = this._collections.nodeContainers.get(nodeId);
    let shouldBeHidden = nodeData.hidden || false;
    
    if (parentContainer) {
      const container = this._collections.containers.get(parentContainer);
      if (container && container.collapsed) {
        shouldBeHidden = true;
      }
    }
    
    // Ensure all nodes have default dimensions
    const processedData = { 
      ...nodeData, 
      id: nodeId, 
      hidden: shouldBeHidden,
      width: nodeData.width || LAYOUT_CONSTANTS.DEFAULT_NODE_WIDTH,
      height: nodeData.height || LAYOUT_CONSTANTS.DEFAULT_NODE_HEIGHT 
    };
    
    this._collections.graphNodes.set(nodeId, processedData);
    
    // Update visibility cache
    if (!shouldBeHidden) {
      this._collections._visibleNodes.set(nodeId, processedData);
    }
    
    // Update edge mappings if needed
    this._collections.nodeToEdges.set(nodeId, new Set());
  }
  
  /**
   * Add a graph edge directly (for JSONParser and initial data loading)
   */
  addGraphEdge(edgeId: string, edgeData: any): void {
    const processedData = { 
      ...edgeData, 
      id: edgeId,
      hidden: edgeData.hidden || false
    };
    this._collections.graphEdges.set(edgeId, processedData);
    
    // Update node-to-edge mappings
    const sourceSet = this._collections.nodeToEdges.get(edgeData.source) || new Set();
    sourceSet.add(edgeId);
    this._collections.nodeToEdges.set(edgeData.source, sourceSet);
    
    const targetSet = this._collections.nodeToEdges.get(edgeData.target) || new Set();
    targetSet.add(edgeId);
    this._collections.nodeToEdges.set(edgeData.target, targetSet);
    
    // Update visibility cache if edge should be visible
    const sourceExists = this._isEndpointVisible(edgeData.source);
    const targetExists = this._isEndpointVisible(edgeData.target);
    if (!processedData.hidden && sourceExists && targetExists) {
      this._collections._visibleEdges.set(edgeId, processedData);
    }
  }
  
  /**
   * Add a container directly (for JSONParser and initial data loading)
   */
  addContainer(containerId: string, containerData: any): void {
    // Ensure proper defaults
    const processedData = {
      ...containerData,
      id: containerId,
      collapsed: containerData.collapsed || false,
      hidden: containerData.hidden || false,
      children: new Set(containerData.children || []),
      width: containerData.width || LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH,
      height: containerData.height || LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT
    };
    
    this._collections.containers.set(containerId, processedData);
    
    // Update visibility caches
    this.visibilityManager.updateContainerVisibilityCaches(containerId, processedData);
    
    // Process children relationships
    if (containerData.children) {
      this._collections.containerChildren.set(containerId, new Set(containerData.children));
      for (const childId of containerData.children) {
        this._collections.nodeContainers.set(childId, containerId);
      }
    }
    
    // If container is collapsed, delegate to the full collapse handling logic
    if (processedData.collapsed) {
      this.containerOps.handleContainerCollapse(containerId);
    }
  }

  // ============ CONTROLLED STATE MUTATION API ============
  
  /**
   * Safely set node visibility with automatic cache updates and edge cascade
   */
  setNodeVisibility(nodeId: string, visible: boolean): void {
    this.visibilityManager.setNodeVisibility(nodeId, visible);
  }
  
  /**
   * Safely set container state with proper cascade and hyperEdge management  
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
      
      // CRITICAL: Always override dimensions when collapsing
      if (state.collapsed) {
        container.width = SIZES.COLLAPSED_CONTAINER_WIDTH;
        container.height = SIZES.COLLAPSED_CONTAINER_HEIGHT;
      }
    }
    if (state.hidden !== undefined) container.hidden = state.hidden;
    
    // Update visibility caches
    this.visibilityManager.updateContainerVisibilityCaches(containerId, container);
    
    // Handle collapse/expand transitions with hyperEdge management
    if (state.collapsed !== undefined && state.collapsed !== wasCollapsed) {
      if (state.collapsed) {
        this.containerOps.handleContainerCollapse(containerId);
      } else {
        this.containerOps.handleContainerExpansion(containerId);
      }
    }
    
    // Handle hide/show transitions  
    if (state.hidden !== undefined && state.hidden !== wasHidden) {
      this.visibilityManager.cascadeContainerVisibility(containerId, !state.hidden);
    }
    
    // Only validate at the end for top-level operations
    if (!this._inRecursiveOperation) {
      setTimeout(() => {
        try {
          this.validateInvariants();
        } catch (error) {
          console.error('[VisState] Validation failed:', error);
        }
      }, 0);
    }
  }
  
  /**
   * Safely set edge visibility with endpoint validation
   */
  setEdgeVisibility(edgeId: string, visible: boolean): void {
    this.visibilityManager.setEdgeVisibility(edgeId, visible);
  }

  // ============ LEGACY/COMPATIBILITY API ============
  
  /**
   * Set a graph node (legacy compatibility - forwards to addGraphNode)
   * @deprecated Use addGraphNode() for new code
   */
  setGraphNode(nodeId: string, nodeData: any): VisualizationState {
    this.addGraphNode(nodeId, nodeData);
    return this;
  }
  
  /**
   * Set a graph edge (legacy compatibility - forwards to addGraphEdge)
   * @deprecated Use addGraphEdge() for new code
   */
  setGraphEdge(edgeId: string, edgeData: any): VisualizationState {
    this.addGraphEdge(edgeId, edgeData);
    return this;
  }
  
  /**
   * Set a container (legacy compatibility - forwards to addContainer)
   * @deprecated Use addContainer() for new code
   */
  setContainer(containerId: string, containerData: any): VisualizationState {
    this.addContainer(containerId, containerData);
    return this;
  }

  /**
   * Collapse a container (legacy compatibility method)
   */
  collapseContainer(containerId: string): void {
    const container = this._collections.containers.get(containerId);
    if (!container) {
      throw new Error(`Cannot collapse non-existent container: ${containerId}`);
    }
    
    this._inRecursiveOperation = true;
    try {
      this._recentlyCollapsedContainers.add(containerId);
      this.setContainerState(containerId, { collapsed: true });
      
      setTimeout(() => {
        this._recentlyCollapsedContainers.delete(containerId);
      }, 2000);
      
    } finally {
      this._inRecursiveOperation = false;
      setTimeout(() => {
        try {
          this.validateInvariants();
        } catch (error) {
          console.error('[VisState] Validation failed:', error);
        }
      }, 0);
    }
  }
  
  /**
   * Expand a container (legacy compatibility method)
   */
  expandContainer(containerId: string): void {
    const container = this._collections.containers.get(containerId);
    if (!container) {
      throw new Error(`Cannot expand non-existent container: ${containerId}`);
    }
    
    this.setContainerState(containerId, { collapsed: false });
  }

  /**
   * Update container properties (legacy compatibility method)
   */
  updateContainer(containerId: string, updates: any): void {
    const container = this._collections.containers.get(containerId);
    if (container) {
      Object.assign(container, updates);
      this.visibilityManager.updateContainerVisibilityCaches(containerId, container);
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

  // ============ BRIDGE COMPATIBILITY (Delegate to BridgeCompatibility) ============

  getParentChildMap(): Map<string, string> {
    return this.bridgeCompat.getParentChildMap();
  }

  getEdgeHandles(edgeId: string): { sourceHandle?: string; targetHandle?: string } {
    return this.bridgeCompat.getEdgeHandles(edgeId);
  }

  getCollapsedContainersAsNodes(): ReadonlyArray<any> {
    return this.bridgeCompat.getCollapsedContainersAsNodes();
  }

  getContainersRequiringLayout(): ReadonlyArray<any> {
    return this.bridgeCompat.getContainersRequiringLayout();
  }

  getTopLevelNodes(): ReadonlyArray<any> {
    return this.bridgeCompat.getTopLevelNodes();
  }

  getContainerELKFixed(containerId: string): boolean {
    return this.bridgeCompat.getContainerELKFixed(containerId);
  }

  setContainerELKFixed(containerId: string, fixed: boolean): void {
    const container = this._collections.containers.get(containerId);
    if (container) {
      container.elkFixed = fixed;
    }
  }

  // ============ CORE CONTAINER OPERATIONS (Direct access) ============

  /**
   * Get crossing edges for a container (core container operation)
   */
  getCrossingEdges(containerId: string): any[] {
    return this.containerOps.getCrossingEdges(containerId);
  }

  // ============ MINIMAL COMPATIBILITY METHODS ============

  setHyperEdge(hyperEdgeId: string, hyperEdgeData: any): this {
    this._collections.hyperEdges.set(hyperEdgeId, hyperEdgeData);
    return this;
  }

  addContainerChild(containerId: string, childId: string): void {
    const children = this._collections.containerChildren.get(containerId) || new Set();
    children.add(childId);
    this._collections.containerChildren.set(containerId, children);
    this._collections.nodeContainers.set(childId, containerId);
  }

  getNodeVisibility(nodeId: string): { hidden?: boolean } {
    const node = this._collections.graphNodes.get(nodeId);
    if (!node) return {};
    
    return {
      hidden: node.hidden || !this._collections._visibleNodes.has(nodeId)
    };
  }

  getEdgeVisibility(edgeId: string): { hidden?: boolean } {
    const edge = this._collections.graphEdges.get(edgeId);
    if (!edge) return {};
    
    return {
      hidden: edge.hidden || !this.visibleEdges.some(e => e.id === edgeId)
    };
  }

  // ============ INTERNAL HELPERS ============

  private _isEndpointVisible(endpointId: string): boolean {
    // Check if it's a visible node
    const node = this._collections.graphNodes.get(endpointId);
    if (node) return !node.hidden;
    
    // Check if it's a visible container (collapsed containers are visible)
    const container = this._collections.containers.get(endpointId);
    if (container) return !container.hidden;
    
    return false;
  }

  // ============ VALIDATION API ============

  /**
   * Validate all VisualizationState invariants
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

  // ============ INTERNAL ACCESS FOR OPERATION CLASSES ============
  // These provide controlled access for the operation classes

  get _internalCollections() {
    return this._collections;
  }

  get _containerOperations() {
    return this.containerOps;
  }

  _updateContainerVisibilityCaches(containerId: string, container: any): void {
    this.visibilityManager.updateContainerVisibilityCaches(containerId, container);
  }

  _cascadeNodeVisibilityToEdges(nodeId: string, visible: boolean): void {
    const connectedEdges = this._collections.nodeToEdges.get(nodeId) || new Set();
    
    for (const edgeId of Array.from(connectedEdges)) {
      const edge = this._collections.graphEdges.get(edgeId);
      if (!edge) continue;
      
      const sourceVisible = this._isEndpointVisible(edge.source);
      const targetVisible = this._isEndpointVisible(edge.target);
      const shouldBeVisible = sourceVisible && targetVisible;
      
      this.setEdgeVisibility(edgeId as string, shouldBeVisible);
    }
  }
}

/**
 * Create factory function for VisualizationState
 */
export function createVisualizationState(): VisualizationState {
  return new VisualizationState();
}
