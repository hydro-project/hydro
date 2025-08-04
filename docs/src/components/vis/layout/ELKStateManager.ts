/**
 * ELK State Manager (TypeScript port from working visualizer)
 * 
 * This module provides wrapper functions that ensure all ELK layout interactions
 * are consistent with visualization state management as the single source of truth.
 * 
 * Key principle: ELK should only ever calculate layouts based on the exact
 * visual state requirements, and return results that perfectly match those requirements.
 */

import ELK from 'elkjs';
import { LayoutConfig } from './types';
import { GraphNode, GraphEdge, Container, HyperEdge, Dimensions } from '../shared/types';
import { 
  ELK_ALGORITHMS, 
  LAYOUT_SPACING, 
  ELK_LAYOUT_OPTIONS, 
  ELKAlgorithm, 
  getELKLayoutOptions,
  createFixedPositionOptions,
  createFreePositionOptions,
  SIZES
} from '../shared/config';

// ============ Constants ============

const VALIDATION_CONSTANTS = {
  DEFAULT_NODE_WIDTH: 180,
  DEFAULT_NODE_HEIGHT: 60,
  DEFAULT_CONTAINER_WIDTH: 400,
  DEFAULT_CONTAINER_HEIGHT: 300,
  COORDINATE_ORIGIN: 0,
} as const;

const LOG_PREFIXES = {
  STATE_MANAGER: '[ELKStateManager]',
  FULL_LAYOUT: '🏗️ FULL_LAYOUT:',
  VISUAL_LAYOUT: '🎯 VISUAL_LAYOUT:',
  VALIDATION: '🔍',
  CACHING: '💾 CACHING:',
  SUMMARY: '📊 SUMMARY:',
  CONTAINER: '📦',
  INPUT: 'INPUT',
  OUTPUT: 'OUTPUT',
  SUCCESS: '✅',
  WARNING: '⚠️',
  ERROR: '❌',
} as const;

// ============ Types ============

export interface LayoutPosition {
  x: number;
  y: number;
}

export interface LayoutDimensions {
  width: number;
  height: number;
}

export interface ContainmentValidationResult {
  isValid: boolean;
  violations: ContainmentViolation[];
}

export interface ContainmentViolation {
  childId: string;
  containerId: string;
  issue: string;
  childBounds: LayoutBounds;
  containerBounds: LayoutBounds;
}

interface LayoutBounds {
  x: number;
  y: number;
  width: number;
  height: number;
  right: number;
  bottom: number;
}

interface ELKNode {
  id: string;
  width?: number;
  height?: number;
  x?: number;
  y?: number;
  children?: ELKNode[];
  layoutOptions?: Record<string, any>;
}

interface ELKEdge {
  id: string;
  sources: string[];
  targets: string[];
}

interface ELKGraph {
  id: string;
  layoutOptions: Record<string, any>;
  children: ELKNode[];
  edges: ELKEdge[];
}

// ============ Layout Cache Management ============

/**
 * Encapsulated dimension cache with consistent interface
 */
class DimensionCache {
  private cache = new Map<string, LayoutDimensions>();

  set(id: string, dimensions: LayoutDimensions): void {
    this.cache.set(id, { ...dimensions });
  }

  get(id: string): LayoutDimensions | undefined {
    const cached = this.cache.get(id);
    return cached ? { ...cached } : undefined;
  }

  has(id: string): boolean {
    return this.cache.has(id);
  }

  clear(): void {
    this.cache.clear();
  }

  size(): number {
    return this.cache.size;
  }

  keys(): IterableIterator<string> {
    return this.cache.keys();
  }
}

// ============ ELK State Manager Interface ============

export interface ELKStateManager {
  calculateFullLayout(
    nodes: GraphNode[],
    edges: GraphEdge[],
    containers: Container[],
    layoutType?: ELKAlgorithm
  ): Promise<{
    nodes: any[];
    edges: GraphEdge[];
  }>;
  
  calculateVisualLayout(
    nodes: GraphNode[],
    edges: GraphEdge[],
    containers: Container[],
    hyperEdges: HyperEdge[],
    layoutType?: ELKAlgorithm,
    dimensionsCache?: Map<string, LayoutDimensions>
  ): Promise<{
    nodes: any[];
    edges: GraphEdge[];
    elkResult: any;
  }>;
}

// ============ Validation Utilities ============

/**
 * Encapsulated validation with proper error collection
 */
class ContainmentValidator {
  private violations: ContainmentViolation[] = [];

  /**
   * Validate that nodes fit within their parent containers
   */
  validateContainment(layoutedNodes: any[], containers: Container[]): ContainmentValidationResult {
    this.violations = [];
    
    this.logValidationStart();
    
    containers.forEach(container => {
      this.validateSingleContainer(container, layoutedNodes);
    });
    
    this.logValidationResults();
    
    return {
      isValid: this.violations.length === 0,
      violations: [...this.violations]
    };
  }

  private validateSingleContainer(container: Container, layoutedNodes: any[]): void {
    const containerNode = this.findContainerNode(container.id, layoutedNodes);
    if (!containerNode) {
      this.logWarning(`Container ${container.id} not found in layout result`);
      return;
    }

    const childNodes = this.findChildNodes(container, layoutedNodes);
    this.logContainerValidation(container, containerNode, childNodes);

    childNodes.forEach(childNode => {
      this.validateChildContainment(childNode, container.id, containerNode);
    });
  }

  private findContainerNode(containerId: string, layoutedNodes: any[]): any | null {
    return layoutedNodes.find(n => n.id === containerId) || null;
  }

  private findChildNodes(container: Container, layoutedNodes: any[]): any[] {
    return layoutedNodes.filter(node => container.children.has(node.id));
  }

  private validateChildContainment(childNode: any, containerId: string, containerNode: any): void {
    const childBounds = this.calculateNodeBounds(childNode);
    const containerBounds = this.calculateContainerBounds(containerNode);

    const fitsHorizontally = childBounds.x >= VALIDATION_CONSTANTS.COORDINATE_ORIGIN && 
                           childBounds.right <= containerBounds.width;
    const fitsVertically = childBounds.y >= VALIDATION_CONSTANTS.COORDINATE_ORIGIN && 
                         childBounds.bottom <= containerBounds.height;

    if (!fitsHorizontally || !fitsVertically) {
      this.addViolation(childNode.id, containerId, childBounds, containerBounds, fitsHorizontally, fitsVertically);
    } else {
      this.logSuccess(`Node ${childNode.id} fits in container ${containerId}`);
    }
  }

  private calculateNodeBounds(node: any): LayoutBounds {
    const x = node.position?.x || VALIDATION_CONSTANTS.COORDINATE_ORIGIN;
    const y = node.position?.y || VALIDATION_CONSTANTS.COORDINATE_ORIGIN;
    const width = node.width || VALIDATION_CONSTANTS.DEFAULT_NODE_WIDTH;
    const height = node.height || VALIDATION_CONSTANTS.DEFAULT_NODE_HEIGHT;

    return {
      x,
      y,
      width,
      height,
      right: x + width,
      bottom: y + height
    };
  }

  private calculateContainerBounds(containerNode: any): LayoutBounds {
    const x = VALIDATION_CONSTANTS.COORDINATE_ORIGIN; // Container coordinates are relative
    const y = VALIDATION_CONSTANTS.COORDINATE_ORIGIN;
    const width = containerNode.width || VALIDATION_CONSTANTS.DEFAULT_CONTAINER_WIDTH;
    const height = containerNode.height || VALIDATION_CONSTANTS.DEFAULT_CONTAINER_HEIGHT;

    return {
      x,
      y,
      width,
      height,
      right: x + width,
      bottom: y + height
    };
  }

  private addViolation(
    childId: string, 
    containerId: string, 
    childBounds: LayoutBounds, 
    containerBounds: LayoutBounds,
    fitsHorizontally: boolean,
    fitsVertically: boolean
  ): void {
    const issue = `Does not fit ${!fitsHorizontally ? 'horizontally' : ''} ${!fitsVertically ? 'vertically' : ''}`.trim();
    
    this.violations.push({
      childId,
      containerId,
      issue,
      childBounds,
      containerBounds
    });

    this.logError(`CONTAINMENT VIOLATION: Node ${childId} does not fit in container ${containerId}`);
    this.logError(`  Child (relative): (${childBounds.x}, ${childBounds.y}) ${childBounds.width}x${childBounds.height} -> (${childBounds.right}, ${childBounds.bottom})`);
    this.logError(`  Container bounds: (${containerBounds.x}, ${containerBounds.y}) ${containerBounds.width}x${containerBounds.height} -> (${containerBounds.right}, ${containerBounds.bottom})`);
    this.logError(`  Fits horizontally: ${fitsHorizontally}, Fits vertically: ${fitsVertically}`);
  }

  private logValidationStart(): void {
    // Only log validation details in debug mode
    if (process.env.NODE_ENV === 'development') {
      console.log(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.VALIDATION} Checking containment relationships...`);
    }
  }

  private logValidationResults(): void {
    if (this.violations.length > 0) {
      console.error(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.ERROR} Found ${this.violations.length} containment violations!`);
    } else if (process.env.NODE_ENV === 'development') {
      console.log(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.SUCCESS} All containment relationships are valid`);
    }
  }

  private logContainerValidation(container: Container, containerNode: any, childNodes: any[]): void {
    // Only log detailed container validation in debug mode
    if (process.env.NODE_ENV === 'development') {
      console.log(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.CONTAINER} Validating container ${container.id}:`);
      console.log(`  Container bounds: (${containerNode.position?.x || 0}, ${containerNode.position?.y || 0}) ${containerNode.width}x${containerNode.height}`);
      console.log(`  Child nodes: ${childNodes.length}`);
    }
  }

  private logWarning(message: string): void {
    console.warn(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.WARNING} ${message}`);
  }

  private logError(message: string): void {
    console.error(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.ERROR} ${message}`);
  }

  private logSuccess(message: string): void {
    console.log(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.SUCCESS} ${message}`);
  }
}

// ============ ELK Configuration Utilities ============

/**
 * Encapsulated ELK configuration management
 */
class ELKConfigurationManager {
  /**
   * Get ELK configuration for different contexts
   */
  getConfig(layoutType: ELKAlgorithm, context: 'root' | 'container' = 'root'): Record<string, any> {
    const baseConfig = this.getBaseConfig(layoutType);
    const contextConfig = this.getContextConfig(context);
    
    return { ...baseConfig, ...contextConfig };
  }

  private getBaseConfig(layoutType: ELKAlgorithm): Record<string, any> {
    const algorithm = ELK_ALGORITHMS[layoutType as keyof typeof ELK_ALGORITHMS] || ELK_ALGORITHMS.LAYERED;
    
    return {
      'elk.algorithm': algorithm,
      'elk.direction': 'DOWN',
      'elk.spacing.nodeNode': LAYOUT_SPACING.NODE_TO_NODE_NORMAL.toString(),
      'elk.spacing.edgeEdge': LAYOUT_SPACING.EDGE_TO_EDGE.toString(),
      'elk.spacing.edgeNode': LAYOUT_SPACING.EDGE_TO_NODE.toString(),
      'elk.spacing.componentComponent': LAYOUT_SPACING.COMPONENT_TO_COMPONENT.toString(),
    };
  }

  private getContextConfig(context: 'root' | 'container'): Record<string, any> {
    const padding = context === 'root' 
      ? LAYOUT_SPACING.ROOT_PADDING 
      : LAYOUT_SPACING.CONTAINER_PADDING;

    return {
      'elk.padding.left': padding.toString(),
      'elk.padding.right': padding.toString(),
      'elk.padding.top': padding.toString(),
      'elk.padding.bottom': padding.toString(),
    };
  }
}

// ============ ELK Hierarchy Builder ============

/**
 * Builds ELK graph hierarchy with proper type safety
 */
class ELKHierarchyBuilder {
  constructor(
    private nodes: GraphNode[],
    private containers: Container[],
    private edges: GraphEdge[],
    private configManager: ELKConfigurationManager
  ) {}

  buildElkGraph(layoutType: ELKAlgorithm): ELKGraph {
    return {
      id: 'full_layout_root',
      layoutOptions: this.configManager.getConfig(layoutType, 'root'),
      children: this.buildHierarchy(null, layoutType),
      edges: this.buildEdges(),
    };
  }

  private buildHierarchy(parentId: string | null, layoutType: ELKAlgorithm): ELKNode[] {
    const children: ELKNode[] = [];
    
    // Add containers at this level
    const levelContainers = this.findContainersAtLevel(parentId);
    levelContainers.forEach(container => {
      children.push(this.buildContainerNode(container, layoutType));
    });

    // Add regular nodes at this level
    const levelNodes = this.findNodesAtLevel(parentId);
    levelNodes.forEach(node => {
      children.push(this.buildRegularNode(node));
    });

    return children;
  }

  private findContainersAtLevel(parentId: string | null): Container[] {
    return this.containers.filter(container => {
      if (parentId === null) {
        // Root level - containers not contained by any other container
        return !this.containers.some(otherContainer => 
          otherContainer.children.has(container.id)
        );
      } else {
        // Non-root level - containers contained by the parent
        const parentContainer = this.containers.find(c => c.id === parentId);
        return parentContainer?.children.has(container.id) || false;
      }
    });
  }

  private findNodesAtLevel(parentId: string | null): GraphNode[] {
    const regularNodes = this.nodes.filter(node => node.type !== 'container');
    
    return regularNodes.filter(node => {
      if (parentId === null) {
        // Root level - nodes not contained by any container
        return !this.containers.some(container => 
          container.children.has(node.id)
        );
      } else {
        // Non-root level - nodes contained by the parent
        const parentContainer = this.containers.find(c => c.id === parentId);
        return parentContainer?.children.has(node.id) || false;
      }
    });
  }

  private buildContainerNode(container: Container, layoutType: ELKAlgorithm): ELKNode {
    const elkNode: ELKNode = {
      id: container.id,
      layoutOptions: this.configManager.getConfig(layoutType, 'container'),
      children: this.buildHierarchy(container.id, layoutType),
    };

    // Check for dimensions in container.layout.dimensions (for collapsed containers)
    const layoutDimensions = container.layout?.dimensions;
    if (layoutDimensions) {
      elkNode.width = layoutDimensions.width;
      elkNode.height = layoutDimensions.height;
      console.log(`${LOG_PREFIXES.STATE_MANAGER} 🔍 CONTAINER_DIMS: ${container.id} from VisState.layout: ${layoutDimensions.width}x${layoutDimensions.height} (collapsed=${container.collapsed})`);
    }
    // Fallback: check for direct dimensions property
    else if (container.dimensions) {
      elkNode.width = container.dimensions.width;
      elkNode.height = container.dimensions.height;
      console.log(`${LOG_PREFIXES.STATE_MANAGER} 🔍 CONTAINER_DIMS: ${container.id} from VisState.dimensions: ${container.dimensions.width}x${container.dimensions.height} (collapsed=${container.collapsed})`);
    } else {
      console.log(`${LOG_PREFIXES.STATE_MANAGER} 🔍 CONTAINER_DIMS: ${container.id} NO dimensions in VisState (collapsed=${container.collapsed}) - letting ELK calculate`);
    }
    // For expanded containers without explicit dimensions, let ELK calculate based on children

    return elkNode;
  }

  private buildRegularNode(node: GraphNode): ELKNode {
    const width = node.dimensions?.width || VALIDATION_CONSTANTS.DEFAULT_NODE_WIDTH;
    const height = node.dimensions?.height || VALIDATION_CONSTANTS.DEFAULT_NODE_HEIGHT;
    
    return {
      id: node.id,
      width,
      height,
    };
  }

  private buildEdges(): ELKEdge[] {
    // Use all edges from this.edges (which now includes both regular edges and hyperedges)
    const elkEdges = this.edges.map(edge => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target],
    }));
    
    // Log what edges are being sent to ELK
    console.log(`${LOG_PREFIXES.STATE_MANAGER} 🔥 BUILDING EDGES FOR ELK:`);
    console.log(`  📊 Total edges: ${elkEdges.length}`);
    
    elkEdges.forEach(edge => {
      console.log(`    🔗 EDGE ${edge.id}: ${edge.sources[0]} → ${edge.targets[0]}`);
    });
    
    return elkEdges;
  }
}

// ============ Position Application Utilities ============

/**
 * Applies ELK layout results back to nodes with proper type safety
 */
class PositionApplicator {
  applyPositions(elkNodes: ELKNode[], originalNodes: GraphNode[], containers: Container[]): any[] {
    return this.processElkNodes(elkNodes, originalNodes, containers, 0);
  }

  private processElkNodes(elkNodes: ELKNode[], originalNodes: GraphNode[], containers: Container[], depth: number): any[] {
    const layoutedNodes: any[] = [];
    
    elkNodes.forEach(elkNode => {
      const processedNode = this.processElkNode(elkNode, originalNodes, containers);
      if (processedNode) {
        layoutedNodes.push(processedNode);
      }
      
      // Recursively process children
      if (elkNode.children) {
        layoutedNodes.push(...this.processElkNodes(elkNode.children, originalNodes, containers, depth + 1));
      }
    });
    
    return layoutedNodes;
  }

  private processElkNode(elkNode: ELKNode, originalNodes: GraphNode[], containers: Container[]): any | null {
    const originalNode = originalNodes.find(n => n.id === elkNode.id);
    const originalContainer = containers.find(c => c.id === elkNode.id);
    const original = originalNode || originalContainer;
    
    if (!original) {
      return null;
    }

    return {
      ...original,
      width: elkNode.width || VALIDATION_CONSTANTS.DEFAULT_NODE_WIDTH,
      height: elkNode.height || VALIDATION_CONSTANTS.DEFAULT_NODE_HEIGHT,
      position: {
        x: elkNode.x || VALIDATION_CONSTANTS.COORDINATE_ORIGIN,
        y: elkNode.y || VALIDATION_CONSTANTS.COORDINATE_ORIGIN,
      },
      dimensions: {
        width: elkNode.width || VALIDATION_CONSTANTS.DEFAULT_NODE_WIDTH,
        height: elkNode.height || VALIDATION_CONSTANTS.DEFAULT_NODE_HEIGHT,
      },
    };
  }
}

// ============ Node Sorting Utilities ============

/**
 * Sorts nodes to ensure parents come before children (ReactFlow requirement)
 */
class NodeSorter {
  sortNodesForReactFlow(layoutedNodes: any[], containers: Container[]): any[] {
    const sortedNodes: any[] = [];
    const nodeMap = new Map(layoutedNodes.map(node => [node.id, node]));
    const visited = new Set<string>();
    
    layoutedNodes.forEach(node => this.addNodeAndParents(node.id, nodeMap, containers, visited, sortedNodes));
    
    return sortedNodes;
  }

  private addNodeAndParents(
    nodeId: string, 
    nodeMap: Map<string, any>, 
    containers: Container[], 
    visited: Set<string>, 
    sortedNodes: any[]
  ): void {
    if (visited.has(nodeId)) return;
    
    const node = nodeMap.get(nodeId);
    if (!node) return;
    
    // Find parent container
    const parentContainer = containers.find(container => 
      container.children.has(nodeId)
    );
    
    if (parentContainer && !visited.has(parentContainer.id)) {
      this.addNodeAndParents(parentContainer.id, nodeMap, containers, visited, sortedNodes);
    }
    
    visited.add(nodeId);
    sortedNodes.push(node);
  }
}

/**
 * Create an ELK state manager that wraps all ELK layout interactions
 * with proper state management as the single source of truth.
 */
export function createELKStateManager(): ELKStateManager {
  const elk = new ELK();
  const validator = new ContainmentValidator();
  const configManager = new ELKConfigurationManager();
  const positionApplicator = new PositionApplicator();
  const nodeSorter = new NodeSorter();

  /**
   * Calculate full layout for dimension caching (expanded state).
   * This is used to populate the dimension cache with expanded container sizes.
   */
  async function calculateFullLayout(
    nodes: GraphNode[],
    edges: GraphEdge[],
    containers: Container[],
    layoutType: ELKAlgorithm = ELK_ALGORITHMS.LAYERED
  ): Promise<{
    nodes: any[];
    edges: GraphEdge[];
  }> {
    console.log(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.FULL_LAYOUT} Calculating expanded layout for dimension caching`);
    
    logLayoutSummary(nodes, edges, containers);

    try {
      const hierarchyBuilder = new ELKHierarchyBuilder(nodes, containers, edges, configManager);
      const elkGraph = hierarchyBuilder.buildElkGraph(layoutType);

      logELKInput(elkGraph);
      
      const layoutResult = await elk.layout(elkGraph);

      logELKOutput(layoutResult);

      // Apply positions back to nodes
      const layoutedNodes = positionApplicator.applyPositions(layoutResult.children || [], nodes, containers);

      // Validate containment relationships
      console.log(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.VALIDATION} CONTAINMENT VALIDATION:`);
      const validationResult = validator.validateContainment(layoutedNodes, containers);
      
      if (!validationResult.isValid) {
        console.warn(`${LOG_PREFIXES.STATE_MANAGER} Layout validation found issues, but proceeding with layout.`);
      }

      // Sort nodes so parents come before children (ReactFlow requirement)
      const sortedNodes = nodeSorter.sortNodesForReactFlow(layoutedNodes, containers);
      
      return {
        nodes: sortedNodes,
        edges: edges,
      };

    } catch (error) {
      console.error(`${LOG_PREFIXES.STATE_MANAGER} Full layout failed:`, error);
      throw error;
    }
  }

  /**
   * Calculate layout with optional position fixing.
   * If changedContainerId is provided, only that container can move (selective layout).
   * If changedContainerId is null, all containers can move (full layout).
   * ALL DATA FLOWS THROUGH VISSTATE!
   */
  async function calculateVisualLayout(
    nodes: GraphNode[],
    edges: GraphEdge[],
    containers: Container[],
    hyperEdges: HyperEdge[],
    layoutType: ELKAlgorithm = ELK_ALGORITHMS.LAYERED,
    dimensionsCache?: Map<string, LayoutDimensions>,
    changedContainerId?: string | null,
    visualizationState?: any // VisState reference for centralized state management
  ): Promise<{
    nodes: any[];
    edges: GraphEdge[];
    elkResult: any;
  }> {
    const isSelectiveLayout = changedContainerId !== undefined && changedContainerId !== null;
    console.log(`${LOG_PREFIXES.STATE_MANAGER} ${isSelectiveLayout ? '🔄 SELECTIVE' : '🏗️ FULL'}_LAYOUT: ${isSelectiveLayout ? `Changed: ${changedContainerId}` : 'All containers free'}`);
    
    // For selective layout: Use full hierarchical layout with position fixing
    if (isSelectiveLayout && visualizationState) {
      console.log(`${LOG_PREFIXES.STATE_MANAGER} 📌 POSITION_FIXING: Setting up container fixed/free states in VisState`);
      
      // Use VisState method to set up position fixing - CENTRALIZED LOGIC
      const containersWithFixing = visualizationState.getContainersRequiringLayout(changedContainerId);
      
      console.log(`${LOG_PREFIXES.STATE_MANAGER} 🏗️ SELECTIVE_HIERARCHICAL_LAYOUT: Running full layout with position fixing`);
      
      // Combine regular edges and hyperEdges for ELK layout
      // Ensure hyperEdges have the hidden property for type compatibility
      const hyperEdgesWithHidden = hyperEdges.map(he => ({ ...he, hidden: false }));
      const allEdges = [...edges, ...hyperEdgesWithHidden];
      
      // Run full hierarchical layout but with position constraints
      const result = await calculateFullLayout(nodes, allEdges, containersWithFixing, layoutType);
      
      return {
        ...result,
        elkResult: null, // No separate ELK result for hierarchical layout
      };
    } else {
      // Combine regular edges and hyperEdges for ELK layout
      // Ensure hyperEdges have the hidden property for type compatibility
      const hyperEdgesWithHidden = hyperEdges.map(he => ({ ...he, hidden: false }));
      const allEdges = [...edges, ...hyperEdgesWithHidden];
      
      const result = await calculateFullLayout(nodes, allEdges, containers, layoutType);
      return {
        ...result,
        elkResult: null,
      };
    }
  }

  /**
   * Simple container repositioning with position fixing - ALL DATA FROM VISSTATE
   */
  async function calculateContainerLayout(
    containers: Container[],
    layoutType: ELKAlgorithm,
    dimensionsCache?: Map<string, LayoutDimensions>,
    changedContainerId?: string | null
  ): Promise<{
    nodes: any[];
    edges: GraphEdge[];
    elkResult: any;
  }> {
    const visibleContainers = containers.filter(c => !c.hidden);
    
    if (visibleContainers.length === 0) {
      return { nodes: [], edges: [], elkResult: null };
    }

    console.log(`${LOG_PREFIXES.STATE_MANAGER} 🏗️ CONTAINER_LAYOUT: Building ELK graph from VisState data`);

    // Create ELK nodes - ALL configuration comes from VisState
    const elkContainers = visibleContainers.map(container => {
      console.log(`${LOG_PREFIXES.STATE_MANAGER} 🔍 CONTAINER_DEBUG: ${container.id}`);
      console.log(`${LOG_PREFIXES.STATE_MANAGER} 🔍   - collapsed: ${container.collapsed}`);
      console.log(`${LOG_PREFIXES.STATE_MANAGER} 🔍   - layout: ${JSON.stringify(container.layout)}`);
      
      // Get dimensions from VisState layout (handles collapsed/expanded automatically)
      const layout = container.layout || {};
      const dimensions = layout.dimensions || { 
        width: container.collapsed ? SIZES.COLLAPSED_CONTAINER_WIDTH : (dimensionsCache?.get(container.id)?.width || 400),
        height: container.collapsed ? SIZES.COLLAPSED_CONTAINER_HEIGHT : (dimensionsCache?.get(container.id)?.height || 300)
      };

      console.log(`${LOG_PREFIXES.STATE_MANAGER} 🔍   - final dimensions: ${dimensions.width}x${dimensions.height}`);

      // Get position from VisState layout
      const position = layout.position || { x: 0, y: 0 };

      // Get ELK options from VisState (handles fixed/free automatically)  
      const elkFixed = layout.elkFixed;
      const layoutOptions = elkFixed 
        ? createFixedPositionOptions(position.x, position.y)
        : createFreePositionOptions();

      console.log(`${LOG_PREFIXES.STATE_MANAGER} 📦 CONTAINER: ${container.id} ${dimensions.width}x${dimensions.height} at (${position.x},${position.y}) ${elkFixed ? 'FIXED' : 'FREE'}`);

      return {
        id: container.id,
        width: dimensions.width,
        height: dimensions.height,
        layoutOptions
      };
    });

    const elkGraph = {
      id: 'container_layout',
      layoutOptions: getELKLayoutOptions(layoutType),
      children: elkContainers,
      edges: []
    };

    console.log(`${LOG_PREFIXES.STATE_MANAGER} 🚀 CONTAINER_LAYOUT: Running ELK with ${elkContainers.length} containers`);
    const layoutResult = await elk.layout(elkGraph);
    console.log(`${LOG_PREFIXES.STATE_MANAGER} ✅ CONTAINER_LAYOUT: ELK completed`);

    return {
      nodes: [], // Container layout doesn't affect regular nodes
      edges: [],
      elkResult: layoutResult,
    };
  }

  return {
    calculateFullLayout,
    calculateVisualLayout,
  };
}

// ============ Logging Utilities ============

function logLayoutSummary(nodes: GraphNode[], edges: GraphEdge[], containers: Container[]): void {
  // Only log summary in debug mode
  if (process.env.NODE_ENV === 'development') {
    console.log(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.SUMMARY}`);
    console.log(`  Nodes: ${nodes.length}`);
    console.log(`  Containers: ${containers.length}`);
    containers.forEach(container => {
      console.log(`    Container ${container.id}: ${container.children.size} children`);
    });
    console.log(`  Edges: ${edges.length}`);
  }
}

function logELKInput(elkGraph: ELKGraph): void {
  // Only log ELK input in debug mode
  if (process.env.NODE_ENV === 'development') {
    console.log(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.VALIDATION} ELK CONTAINER ${LOG_PREFIXES.INPUT}:`);
    logELKContainerHierarchy(elkGraph.children, 0, LOG_PREFIXES.INPUT);
  }
}

function logELKOutput(layoutResult: any): void {
  // Always log ELK output for hyperedge debugging
  console.log(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.VALIDATION} ELK CONTAINER ${LOG_PREFIXES.OUTPUT}:`);
  if (layoutResult.children) {
    logELKContainerHierarchy(layoutResult.children, 0, LOG_PREFIXES.OUTPUT);
  }
  
  // FOCUSED HYPEREDGE LOGGING: Check for ELK edge routing information
  console.log(`${LOG_PREFIXES.STATE_MANAGER} 🔥 ELK EDGE ROUTING ANALYSIS:`);
  if (layoutResult.edges && layoutResult.edges.length > 0) {
    console.log(`  📊 Found ${layoutResult.edges.length} edges with routing info from ELK`);
    layoutResult.edges.forEach((edge: any) => {
      console.log(`    Edge ${edge.id}:`);
      console.log(`      Sources: ${edge.sources || 'undefined'}`);
      console.log(`      Targets: ${edge.targets || 'undefined'}`);
      
      if (edge.sections && edge.sections.length > 0) {
        console.log(`      🛣️  Sections (${edge.sections.length}):`);
        edge.sections.forEach((section: any, i: number) => {
          console.log(`        Section ${i}:`);
          if (section.startPoint) {
            console.log(`          Start: (${section.startPoint.x}, ${section.startPoint.y})`);
          }
          if (section.endPoint) {
            console.log(`          End: (${section.endPoint.x}, ${section.endPoint.y})`);
          }
          if (section.bendPoints && section.bendPoints.length > 0) {
            console.log(`          Bend points: ${section.bendPoints.map((bp: any) => `(${bp.x},${bp.y})`).join(', ')}`);
          }
        });
      } else {
        console.log(`      ⚠️  No sections/routing info for edge ${edge.id}`);
      }
    });
  } else {
    console.log(`  ⚠️  No edge routing information provided by ELK`);
  }
}

function logELKContainerHierarchy(nodes: any[], depth: number, type: string): void {
  const indent = '  '.repeat(depth);
  nodes.forEach(node => {
    if (node.children && node.children.length > 0) {
      // This is a container
      const dimensionInfo = type === LOG_PREFIXES.INPUT 
        ? `width=${node.width || 'undefined'}, height=${node.height || 'undefined'}`
        : `x=${node.x}, y=${node.y}, width=${node.width}, height=${node.height}`;
      
      console.log(`${indent}${LOG_PREFIXES.CONTAINER} CONTAINER ${type} ${node.id}: children=${node.children.length}, ${dimensionInfo}`);
      
      if (type === LOG_PREFIXES.INPUT && node.layoutOptions) {
        console.log(`${indent}   layoutOptions:`, node.layoutOptions);
      }
      
      logELKContainerHierarchy(node.children, depth + 1, type);
    }
  });
}
