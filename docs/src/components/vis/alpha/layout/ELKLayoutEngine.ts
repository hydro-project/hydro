/**
 * @fileoverview ELK Layout Engine (Enhanced with working patterns)
 * 
 * ELK-based automatic layout engine using proven patterns from the working visualizer.
 * Handles hierarchical layouts with proper container dimension management.
 */

import { LayoutEngine, LayoutResult, LayoutConfig } from './types';
import { GraphNode, GraphEdge, Container, HyperEdge } from '../shared/types';
import { createELKStateManager, ELKStateManager, LayoutDimensions } from './ELKStateManager';
import { SIZES } from '../shared/config';
import { ELK_ALGORITHMS, ELKAlgorithm } from '../shared/config';

// ============ Constants ============

const ENGINE_CONSTANTS = {
  DEFAULT_ALGORITHM: ELK_ALGORITHMS.LAYERED,
  DEFAULT_CONTAINER_WIDTH: 400,
  DEFAULT_CONTAINER_HEIGHT: 300,
  DEFAULT_NODE_WIDTH: 180,
  DEFAULT_NODE_HEIGHT: 60,
} as const;

const LOG_PREFIXES = {
  ENGINE: '[ELKLayoutEngine]',
  CACHING: '💾 CACHING:',
  SUCCESS: '✅',
  ERROR: '❌',
} as const;

// ============ Dimension Cache Management ============

/**
 * Encapsulated cache for container dimensions with type safety
 */
class ContainerDimensionCache {
  private cache = new Map<string, LayoutDimensions>();

  set(containerId: string, dimensions: LayoutDimensions): void {
    this.cache.set(containerId, { ...dimensions });
    console.log(`${LOG_PREFIXES.ENGINE} ${LOG_PREFIXES.CACHING} ${containerId} → ${dimensions.width}x${dimensions.height}`);
  }

  get(containerId: string): LayoutDimensions | undefined {
    const cached = this.cache.get(containerId);
    return cached ? { ...cached } : undefined;
  }

  clear(): void {
    this.cache.clear();
  }

  size(): number {
    return this.cache.size;
  }
}

// ============ Layout Result Converter ============

/**
 * Converts ELK state manager results to LayoutResult format
 */
class LayoutResultConverter {
  convert(
    elkResult: any,
    originalNodes: GraphNode[],
    originalEdges: GraphEdge[],
    originalContainers: Container[],
    originalHyperEdges: HyperEdge[],
    dimensionCache: ContainerDimensionCache
  ): LayoutResult {
    return {
      nodes: this.convertNodes(elkResult.nodes, originalNodes),
      edges: this.convertEdges(originalEdges),
      containers: this.convertContainers(elkResult.nodes, originalContainers, dimensionCache),
      hyperEdges: this.convertHyperEdges(originalHyperEdges)
    };
  }

  private convertNodes(elkNodes: any[], originalNodes: GraphNode[]): LayoutResult['nodes'] {
    return elkNodes
      .filter(node => originalNodes.find(n => n.id === node.id)) // Only include actual nodes
      .map(node => {
        const originalNode = originalNodes.find(n => n.id === node.id)!;
        return {
          ...originalNode,
          x: node.position?.x || 0,
          y: node.position?.y || 0,
          width: node.width || node.dimensions?.width || ENGINE_CONSTANTS.DEFAULT_NODE_WIDTH,
          height: node.height || node.dimensions?.height || ENGINE_CONSTANTS.DEFAULT_NODE_HEIGHT
        };
      });
  }

  private convertEdges(originalEdges: GraphEdge[]): LayoutResult['edges'] {
    return originalEdges.map(edge => ({
      ...edge,
      points: [] // ELK routing will be added later if needed
    }));
  }

  private convertContainers(
    elkNodes: any[], 
    originalContainers: Container[], 
    dimensionCache: ContainerDimensionCache
  ): LayoutResult['containers'] {
    return originalContainers.map(container => {
      const layoutedNode = elkNodes.find(n => n.id === container.id);
      const cachedDimensions = dimensionCache.get(container.id);
      
      // Priority: ELK-calculated > cached > original > fallback
      const baseWidth = this.getDimension(
        layoutedNode?.width,
        layoutedNode?.dimensions?.width,
        cachedDimensions?.width,
        container.expandedDimensions?.width,
        ENGINE_CONSTANTS.DEFAULT_CONTAINER_WIDTH
      );
      
      const baseHeight = this.getDimension(
        layoutedNode?.height,
        layoutedNode?.dimensions?.height,
        cachedDimensions?.height,
        container.expandedDimensions?.height,
        ENGINE_CONSTANTS.DEFAULT_CONTAINER_HEIGHT
      );
      
      // Add padding to height for title area
      const adjustedHeight = baseHeight + SIZES.CONTAINER_TITLE_AREA_PADDING;
      
      return {
        ...container,
        x: layoutedNode?.position?.x || 0,
        y: layoutedNode?.position?.y || 0,
        width: baseWidth,
        height: adjustedHeight
      };
    });
  }

  private convertHyperEdges(originalHyperEdges: HyperEdge[]): LayoutResult['hyperEdges'] {
    return originalHyperEdges.map(hyperEdge => ({
      ...hyperEdge,
      points: []
    }));
  }

  private getDimension(...values: (number | undefined)[]): number {
    return values.find(v => v !== undefined) || ENGINE_CONSTANTS.DEFAULT_CONTAINER_WIDTH;
  }
}

// ============ ELK Layout Engine Implementation ============

export class ELKLayoutEngine implements LayoutEngine {
  private elkStateManager: ELKStateManager;
  private dimensionCache = new ContainerDimensionCache();
  private resultConverter = new LayoutResultConverter();

  constructor() {
    this.elkStateManager = createELKStateManager();
  }

  async layout(
    nodes: GraphNode[],
    edges: GraphEdge[],
    containers: Container[],
    hyperEdges: HyperEdge[],
    config: LayoutConfig = {}
  ): Promise<LayoutResult> {
    return this.layoutWithChangedContainer(nodes, edges, containers, hyperEdges, config, null);
  }

  /**
   * Layout with optional selective positioning.
   * If changedContainerId is provided, only that container can move.
   * ALL RESULTS APPLIED BACK TO VISSTATE!
   */
  async layoutWithChangedContainer(
    nodes: GraphNode[],
    edges: GraphEdge[],
    containers: Container[],
    hyperEdges: HyperEdge[],
    config: LayoutConfig = {},
    changedContainerId: string | null = null,
    visualizationState?: any // VisState reference for centralized state management
  ): Promise<LayoutResult> {
    try {
      const isSelective = changedContainerId !== null;
      console.log(`${LOG_PREFIXES.ENGINE} ${isSelective ? 'Selective' : 'Full'} layout ${isSelective ? `(changed: ${changedContainerId})` : ''}`);
      
      const algorithm = this.getLayoutAlgorithm(config.algorithm);
      
      if (isSelective) {
        // Use selective layout method with VisState reference
        const result = await this.elkStateManager.calculateVisualLayout(
          nodes,
          edges,
          containers,
          hyperEdges,
          algorithm,
          this.dimensionCache,
          changedContainerId,
          visualizationState // Pass VisState for centralized state management
        );

        // Apply ELK results back to VisState for containers
        if (visualizationState && result.elkResult) {
          console.log(`${LOG_PREFIXES.ENGINE} 📝 APPLYING: ELK results back to VisState (selective)`);
          this.applyELKResultsToVisState(result.elkResult, visualizationState, changedContainerId);
        }

        // Convert to our LayoutResult format
        const layoutResult = this.resultConverter.convert(
          result,
          nodes,
          edges,
          containers,
          hyperEdges,
          this.dimensionCache
        );

        console.log(`${LOG_PREFIXES.ENGINE} Layout completed successfully`);
        return layoutResult;
        
      } else {
        // Full layout - use calculateFullLayout but still apply results to VisState
        const result = await this.elkStateManager.calculateFullLayout(
          nodes,
          edges,
          containers,
          algorithm
        );

        // For full layout, we need to apply the node positions back to VisState
        if (visualizationState) {
          console.log(`${LOG_PREFIXES.ENGINE} 📝 APPLYING: Full layout results back to VisState`);
          this.applyFullLayoutResultsToVisState(result.nodes, visualizationState);
        }

        // Cache container dimensions for future use
        this.cacheContainerDimensions(result.nodes, containers);

        // Convert to our LayoutResult format
        const layoutResult = this.resultConverter.convert(
          { nodes: result.nodes, edges: result.edges, elkResult: null },
          nodes,
          edges,
          containers,
          hyperEdges,
          this.dimensionCache
        );

        console.log(`${LOG_PREFIXES.ENGINE} Layout completed successfully`);
        return layoutResult;
      }

    } catch (error) {
      console.error(`${LOG_PREFIXES.ENGINE} Layout failed:`, error);
      throw error;
    }
  }

  /**
   * Get cached container dimensions
   */
  getCachedDimensions(containerId: string): LayoutDimensions | undefined {
    return this.dimensionCache.get(containerId);
  }

  /**
   * Clear the dimensions cache
   */
  clearCache(): void {
    this.dimensionCache.clear();
  }

  /**
   * Get cache statistics
   */
  getCacheStats(): { size: number } {
    return { size: this.dimensionCache.size() };
  }

  private getLayoutAlgorithm(algorithm?: string): ELKAlgorithm {
    if (algorithm && Object.values(ELK_ALGORITHMS).includes(algorithm as ELKAlgorithm)) {
      return algorithm as ELKAlgorithm;
    }
    return ENGINE_CONSTANTS.DEFAULT_ALGORITHM;
  }

  /**
   * Apply ELK layout results back to VisState - CENTRALIZED STATE MANAGEMENT
   */
  private applyELKResultsToVisState(elkResult: any, visualizationState: any, changedContainerId: string | null): void {
    console.log(`${LOG_PREFIXES.ENGINE} 📝 APPLYING: ELK results to VisState for containers`);
    
    if (!elkResult.children) {
      console.warn(`${LOG_PREFIXES.ENGINE} ⚠️ No ELK children to apply`);
      return;
    }

    elkResult.children.forEach((elkContainer: any) => {
      console.log(`${LOG_PREFIXES.ENGINE} 📦 UPDATING: Container ${elkContainer.id} position in VisState: (${elkContainer.x}, ${elkContainer.y})`);
      
      // Update position in VisState - SINGLE SOURCE OF TRUTH
      visualizationState.setContainerLayout(elkContainer.id, {
        position: {
          x: elkContainer.x || 0,
          y: elkContainer.y || 0
        }
      });
    });
    
    console.log(`${LOG_PREFIXES.ENGINE} ✅ APPLIED: All ELK results to VisState`);
  }

  /**
   * Apply full layout results (nodes with positions) back to VisState
   * PUBLIC METHOD for VisualizationService
   */
  applyFullLayoutResultsToVisState(layoutedNodes: any[], visualizationState: any): void {
    console.log(`${LOG_PREFIXES.ENGINE} 📝 APPLYING: Full layout results to VisState`);
    console.log(`${LOG_PREFIXES.ENGINE} 🔍 DEBUG: Received ${layoutedNodes.length} nodes to apply`);
    
    layoutedNodes.forEach((node: any) => {
      // DEBUG: Log the entire node structure to see what we're getting
      console.log(`${LOG_PREFIXES.ENGINE} 🔍 DEBUG: Node structure for ${node.id}:`, {
        id: node.id,
        type: node.type,
        position: node.position,
        dimensions: node.dimensions,
        width: node.width,
        height: node.height,
        x: node.x,
        y: node.y,
        hasChildren: !!node.children
      });
      
      const position = node.position || { x: node.x || 0, y: node.y || 0 };
      const dimensions = node.dimensions || { width: node.width || 180, height: node.height || 60 };
      
      console.log(`${LOG_PREFIXES.ENGINE} 📍 UPDATING: ${node.id} position in VisState: (${position.x}, ${position.y})`);
      
      // Check if it's a container or regular node
      if (node.type === 'container' || node.children) {
        // Update container layout in VisState
        visualizationState.setContainerLayout(node.id, {
          position: position,
          dimensions: dimensions
        });
      } else {
        // Update node layout in VisState
        visualizationState.setNodeLayout(node.id, {
          position: position,
          dimensions: dimensions
        });
      }
    });
    
    console.log(`${LOG_PREFIXES.ENGINE} ✅ APPLIED: All full layout results to VisState`);
  }

  private cacheContainerDimensions(elkNodes: any[], containers: Container[]): void {
    elkNodes.forEach(node => {
      // Check if this node is actually a container
      const correspondingContainer = containers.find(c => c.id === node.id);
      if (correspondingContainer) {
        const baseWidth = node.width || node.dimensions?.width || ENGINE_CONSTANTS.DEFAULT_CONTAINER_WIDTH;
        const baseHeight = node.height || node.dimensions?.height || ENGINE_CONSTANTS.DEFAULT_CONTAINER_HEIGHT;
        
        const dimensions: LayoutDimensions = {
          width: baseWidth,
          height: baseHeight + SIZES.CONTAINER_TITLE_AREA_PADDING // Add padding for title area
        };
        this.dimensionCache.set(node.id, dimensions);
      }
    });
  }
}
