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

import type { Edge, HyperEdge } from './types';
import { LAYOUT_CONSTANTS, HYPEREDGE_CONSTANTS, SIZES } from '../shared/config';
import { ContainerPadding } from './ContainerPadding';

// Import specialized operation classes
import { VisualizationStateInvariantValidator } from './validation/VisualizationStateValidator';
import { ValidationConfigs, wrapPublicMethods } from './validation/ValidationWrapper';
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
    graphNodes: new Map<string, GraphNode>(),
    graphEdges: new Map<string, GraphEdge>(),
    containers: new Map<string, Container>(),
    hyperEdges: new Map<string, HyperEdge>(),
    _visibleNodes: new Map<string, GraphNode>(),
    _visibleEdges: new Map<string, Edge>(),
    _visibleContainers: new Map<string, Container>(),
    _expandedContainers: new Map<string, Container>(),
    collapsedContainers: new Map<string, Container>(),
    nodeToEdges: new Map<string, Set<string>>(),
    manualPositions: new Map<string, {x: number, y: number}>(),
    containerChildren: new Map<string, Set<string>>(),
    nodeContainers: new Map<string, string>(),
    
    // Track active keys to prevent duplicates at React rendering level
    _activeRenderKeys: new Set<string>()
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
  
  // Flag to control validation during transitions
  public _validationEnabled = true;
  public _validationLevel: 'strict' | 'normal' | 'minimal' | 'silent' = 'normal';

  // ============ PROTECTED ACCESSORS (Internal use only) ============
  // These provide controlled access to collections for internal methods
  
  private get graphNodes(): Map<string, GraphNode> { return this._collections.graphNodes; }
  private get graphEdges(): Map<string, GraphEdge> { return this._collections.graphEdges; }
  private get containers(): Map<string, Container> { return this._collections.containers; }
  private get hyperEdges(): Map<string, HyperEdge> { return this._collections.hyperEdges; }
  private get _visibleNodes(): Map<string, GraphNode> { return this._collections._visibleNodes; }
  private get _visibleEdges(): Map<string, Edge> { return this._collections._visibleEdges; }
  private get _visibleContainers(): Map<string, Container> { return this._collections._visibleContainers; }
  private get _expandedContainers(): Map<string, Container> { return this._collections._expandedContainers; }
  private get collapsedContainers(): Map<string, Container> { return this._collections.collapsedContainers; }
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
    
    // Wrap public APIs with validation
    this._wrapPublicMethods();
  }

  /**
   * Wraps public methods with validation at API boundaries
   * This removes the need for internal validation calls throughout the codebase
   */
  private _wrapPublicMethods(): void {
    wrapPublicMethods(this, {
      // State mutation APIs - validate after changes
      'addGraphNode': ValidationConfigs.MUTATOR,
      'addGraphEdge': ValidationConfigs.MUTATOR,  
      'addContainer': ValidationConfigs.MUTATOR,
      'setContainerState': ValidationConfigs.MUTATOR,
      'setContainerCollapsed': ValidationConfigs.MUTATOR,
      'updateNode': ValidationConfigs.MUTATOR,
      'removeGraphNode': ValidationConfigs.MUTATOR,
      'removeGraphEdge': ValidationConfigs.MUTATOR,
      
      // Internal operations called by public APIs - skip validation to prevent duplicate checks
      'setNodeVisibility': ValidationConfigs.INTERNAL,
      'setEdgeVisibility': ValidationConfigs.INTERNAL,
      
      // Read-only APIs - no validation needed (performance)
      'getVisibleNodes': ValidationConfigs.GETTER,
      'getVisibleEdges': ValidationConfigs.GETTER,
      'getVisibleContainers': ValidationConfigs.GETTER,
      'getExpandedContainers': ValidationConfigs.GETTER,
      'getCrossingEdges': ValidationConfigs.GETTER,
      
      // Legacy compatibility methods - validate after
      'setGraphNode': ValidationConfigs.MUTATOR,
      'setHyperEdge': ValidationConfigs.MUTATOR
    });
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
   * Get visible nodes as mutable array (legacy compatibility)
   * @deprecated Use visibleNodes getter for new code
   */
  getVisibleNodes(): any[] {
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
    
    const allEdges = [...regularEdges, ...hyperEdges];
    
    // Check for duplicate keys to prevent React warnings
    this._validateRenderKeys(allEdges);
    
    return allEdges;
  }

  /**
   * Get visible edges as mutable array (legacy compatibility)
   * @deprecated Use visibleEdges getter for new code
   */
  getVisibleEdges(): any[] {
    const regularEdges = Array.from(this._collections._visibleEdges.values());
    const hyperEdges = Array.from(this._collections.hyperEdges.values()).filter((edge: any) => {
      return !edge.hidden;
    });
    
    const allEdges = [...regularEdges, ...hyperEdges];
    
    // Check for duplicate keys to prevent React warnings
    this._validateRenderKeys(allEdges);
    
    return allEdges;
  }
  
  /**
   * Get visible containers for rendering (safe read-only access)
   * Bridges should ONLY use this method, never access internal maps directly
   * Returns containers with dimensions adjusted for labels.
   */
  get visibleContainers(): ReadonlyArray<any> {
    const containers = Array.from(this._collections._visibleContainers.values());
    
    return containers.map(container => {
      const adjustedDimensions = this.layoutOps.getContainerAdjustedDimensions(container.id);
      return {
        ...container,
        width: adjustedDimensions.width,
        height: adjustedDimensions.height
      };
    });
  }

  /**
   * Get visible containers as mutable array (legacy compatibility)
   * @deprecated Use visibleContainers getter for new code
   */
  getVisibleContainers(): any[] {
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
    // Check existing state BEFORE making changes
    const existingContainer = this._collections.containers.get(containerId);
    const wasCollapsed = existingContainer?.collapsed === true;
    
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
    
    // Handle state transitions
    const isNowCollapsed = processedData.collapsed === true;
    const isNowExpanded = processedData.collapsed === false;
    
    if (!wasCollapsed && isNowCollapsed) {
      // Container is being collapsed
      this.containerOps.handleContainerCollapse(containerId);
    } else if (wasCollapsed && isNowExpanded) {
      // Container is being expanded
      // this.containerOps.handleContainerExpansion(containerId);
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
        
        // Disable validation during collapse to avoid intermediate state warnings
        const originalValidation = this._validationEnabled;
        this._validationEnabled = false;
        
        try {
          this.containerOps.handleContainerCollapse(containerId);
        } finally {
          this._validationEnabled = originalValidation;
        }
      } else {
        
        // Disable validation during expansion to avoid intermediate state warnings  
        const originalValidation = this._validationEnabled;
        this._validationEnabled = false;
        
        try {
          this.containerOps.handleContainerExpansion(containerId);
        } finally {
          this._validationEnabled = originalValidation;
        }
      }
    } else {
    }
    
    // Handle hide/show transitions  
    if (state.hidden !== undefined && state.hidden !== wasHidden) {
      this.visibilityManager.cascadeContainerVisibility(containerId, !state.hidden);
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
  setContainer(containerIdOrData: string | any, containerData?: any): VisualizationState {
    if (typeof containerIdOrData === 'string') {
      // Old API: setContainer('id', { ... })
      this.addContainer(containerIdOrData, containerData);
    } else {
      // New API: setContainer({ id: 'id', ... })
      const { id, ...data } = containerIdOrData;
      this.addContainer(id, data);
    }
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
    }
  }
  
  /**
   * Expand a container with proper hyperEdge cleanup
   */
  expandContainer(containerId: string): void {
    const container = this._collections.containers.get(containerId);
    if (!container) {
      throw new Error(`Cannot expand non-existent container: ${containerId}`);
    }
    
    
    // Just update the container's collapsed state - setContainerState will handle calling handleContainerExpansion
    this.setContainerState(containerId, { collapsed: false });
    
  }

  /**
   * Recursively expand a container and all its child containers
   */
  expandContainerRecursive(containerId: string): void {
    const container = this._collections.containers.get(containerId);
    if (!container) {
      throw new Error(`Cannot expand non-existent container: ${containerId}`);
    }
    
    // Use the recursive container operations method
    this.containerOps.handleContainerExpansionRecursive(containerId);
    
    // Update all affected containers' collapsed state
    this._updateCollapsedStateRecursive(containerId, false);
  }

  /**
   * Helper to recursively update collapsed state
   */
  private _updateCollapsedStateRecursive(containerId: string, collapsed: boolean): void {
    this.setContainerState(containerId, { collapsed });
    
    const children = this.getContainerChildren(containerId) || new Set();
    for (const childId of Array.from(children)) {
      if (typeof childId === 'string') {
        const childContainer = this._collections.containers.get(childId);
        if (childContainer) {
          this._updateCollapsedStateRecursive(childId, collapsed);
        }
      }
    }
  }

  /**
   * Expand all containers in bulk with proper validation handling
   */
  expandAllContainers(): void {
    const topLevelContainers = this.getTopLevelContainers();
    const collapsedTopLevel = topLevelContainers.filter(c => c.collapsed);
    
    if (collapsedTopLevel.length === 0) return;
    
    // Expand each top-level collapsed container one by one using the basic method
    for (const container of collapsedTopLevel) {
      this.expandContainerRecursive(container.id);
    }
  }

  /**
   * Collapse all containers in bulk with proper validation handling
   */
  collapseAllContainers(): void {
    const topLevelContainers = this.getTopLevelContainers();
    const expandedTopLevel = topLevelContainers.filter(c => !c.collapsed);
    
    if (expandedTopLevel.length === 0) return;
    
    // Disable validation during bulk collapse to avoid intermediate state issues
    const originalValidation = this._validationEnabled;
    this._validationEnabled = false;
    
    try {
      // Collapse each top-level expanded container (this will cascade to children)
      for (const container of expandedTopLevel) {
        this.collapseContainer(container.id);
      }
    } finally {
      // Re-enable validation and run it once at the end
      this._validationEnabled = originalValidation;
      if (originalValidation) {
        this.validateInvariants();
      }
    }
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

  getContainersRequiringLayout(changedContainerId?: string): ReadonlyArray<any> {
    return this.bridgeCompat.getContainersRequiringLayout();
  }

  getTopLevelNodes(): ReadonlyArray<any> {
    return this.bridgeCompat.getTopLevelNodes();
  }

  getTopLevelContainers(): ReadonlyArray<any> {
    return this.bridgeCompat.getTopLevelContainers();
  }

  getContainerELKFixed(containerId: string): boolean {
    return this.bridgeCompat.getContainerELKFixed(containerId);
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
     // Update node-to-edge mappings
    const sourceSet = this._collections.nodeToEdges.get(hyperEdgeData.source) || new Set();
    sourceSet.add(hyperEdgeId);
    this._collections.nodeToEdges.set(hyperEdgeData.source, sourceSet);
    
    const targetSet = this._collections.nodeToEdges.get(hyperEdgeData.target) || new Set();
    targetSet.add(hyperEdgeId);
    this._collections.nodeToEdges.set(hyperEdgeData.target, targetSet);
    
    // Update visibility cache if edge should be visible
    const sourceExists = this._isEndpointVisible(hyperEdgeData.source);
    const targetExists = this._isEndpointVisible(hyperEdgeData.target);

    return this;
  }

  addContainerChild(containerId: string, childId: string): void {
    const children = this._collections.containerChildren.get(containerId) || new Set();
    children.add(childId);
    this._collections.containerChildren.set(containerId, children);
    this._collections.nodeContainers.set(childId, containerId);
  }

  removeContainerChild(containerId: string, childId: string): void {
    const children = this._collections.containerChildren.get(containerId);
    if (children) {
      children.delete(childId);
      if (children.size === 0) {
        this._collections.containerChildren.delete(containerId);
      }
    }
    this._collections.nodeContainers.delete(childId);
  }

  updateNode(nodeId: string, updates: any): void {
    const node = this._collections.graphNodes.get(nodeId);
    if (node) {
      Object.assign(node, updates);
      
      // Update visibility cache if hidden state changed
      if (updates.hidden !== undefined) {
        if (updates.hidden) {
          this._collections._visibleNodes.delete(nodeId);
        } else {
          this._collections._visibleNodes.set(nodeId, node);
        }
      }
    }
  }

  removeGraphNode(nodeId: string): void {
    this._collections.graphNodes.delete(nodeId);
    this._collections._visibleNodes.delete(nodeId);
    this._collections.nodeToEdges.delete(nodeId);
    this._collections.nodeContainers.delete(nodeId);
  }

  removeGraphEdge(edgeId: string): void {
    const edge = this._collections.graphEdges.get(edgeId);
    if (edge) {
      // Remove from node-to-edges mappings
      const sourceEdges = this._collections.nodeToEdges.get(edge.source);
      if (sourceEdges) sourceEdges.delete(edgeId);
      
      const targetEdges = this._collections.nodeToEdges.get(edge.target);
      if (targetEdges) targetEdges.delete(edgeId);
    }
    
    this._collections.graphEdges.delete(edgeId);
    this._collections._visibleEdges.delete(edgeId);
  }

  removeHyperEdge(edgeId: string): void {
    const edge = this._collections.hyperEdges.get(edgeId);
    if (edge) {
      // Remove from node-to-edges mappings
      const sourceEdges = this._collections.nodeToEdges.get(edge.source);
      if (sourceEdges) sourceEdges.delete(edgeId);

      const targetEdges = this._collections.nodeToEdges.get(edge.target);
      if (targetEdges) targetEdges.delete(edgeId);
    }

    this._collections.hyperEdges.delete(edgeId);
    this._collections._visibleEdges.delete(edgeId);
  }

  setContainerELKFixed(containerId: string, fixed: boolean): void {
    const container = this._collections.containers.get(containerId);
    if (container) {
      container.elkFixed = fixed;
    }
  }

  getContainerCollapsed(containerId: string): boolean {
    const container = this._collections.containers.get(containerId);
    return container?.collapsed || false;
  }

  setContainerCollapsed(containerId: string, collapsed: boolean): void {
    this.setContainerState(containerId, { collapsed });
  }

  get expandedContainers(): ReadonlyArray<any> {
    return this.getExpandedContainers();
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
    if (!this._validationEnabled) {
      return; // Skip validation during controlled transitions
    }
    
    // Skip validation during smart collapse to avoid intermediate state warnings
    if ((this as any).isRunningSmartCollapse) {
      return;
    }
    
    this.invariantValidator.validateInvariants();
  }

  /**
   * Alias for validateInvariants (backwards compatibility)
   */
  validateAllInvariants(context?: string): void {
    if (context) {
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

  /**
   * Validate render keys to prevent React duplicate key warnings
   * This method checks for duplicate IDs in the edge collection that would cause React warnings
   */
  private _validateRenderKeys(edges: any[]): void {
    const seenKeys = new Set<string>();
    const duplicateKeys = new Set<string>();
    
    for (const edge of edges) {
      if (seenKeys.has(edge.id)) {
        duplicateKeys.add(edge.id);
      } else {
        seenKeys.add(edge.id);
      }
    }
    
    if (duplicateKeys.size > 0) {
      console.error(`[DUPLICATE_KEYS] Found duplicate edge keys that will cause React warnings:`, Array.from(duplicateKeys));
      
      // Check for ID overlap between regular and hyperEdges
      const regularEdges = Array.from(this._collections._visibleEdges.values());
      const hyperEdges = Array.from(this._collections.hyperEdges.values()).filter((edge: any) => !edge.hidden);
      const regularIds = new Set(regularEdges.map(e => e.id));
      const hyperIds = new Set(hyperEdges.map(e => e.id));
      const overlap = Array.from(duplicateKeys).filter(id => regularIds.has(id) && hyperIds.has(id));
      
      if (overlap.length > 0) {
        console.error(`[DEBUG] ID OVERLAP between regular edges and hyperEdges:`, overlap);
      }
      
      // In strict validation mode, throw an error to fail tests
      if (this._validationLevel === 'strict') {
        throw new Error(`Duplicate edge keys detected: ${Array.from(duplicateKeys).join(', ')}`);
      }
      
      // In normal mode for fuzz tests, treat as test failure
      if (this._validationLevel === 'normal' && duplicateKeys.size > 0) {
        throw new Error(`FUZZ TEST FAILURE: Found ${duplicateKeys.size} duplicate edge keys that will cause React warnings. This indicates a timing issue in hyperEdge creation/removal.`);
      }
      
      // Otherwise, just log the warning but let React handle it
      console.warn(`[DUPLICATE_KEYS] React will show warnings for these duplicate keys. Check hyperEdge creation/removal timing.`);
    }
  }
}

/**
 * Create factory function for VisualizationState
 */
export function createVisualizationState(): VisualizationState {
  return new VisualizationState();
}
