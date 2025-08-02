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
import { ELK_ALGORITHMS, LAYOUT_SPACING } from '../shared/config';

// Position interface for layout results
export interface LayoutPosition {
  x: number;
  y: number;
}

// Dimensions interface for layout results  
export interface LayoutDimensions {
  width: number;
  height: number;
}

export interface ELKStateManager {
  calculateFullLayout(
    nodes: GraphNode[],
    edges: GraphEdge[],
    containers: Container[],
    layoutType?: string
  ): Promise<{
    nodes: any[];
    edges: GraphEdge[];
  }>;
  
  calculateVisualLayout(
    nodes: GraphNode[],
    edges: GraphEdge[],
    containers: Container[],
    hyperEdges: HyperEdge[],
    layoutType?: string,
    dimensionsCache?: Map<string, LayoutDimensions>
  ): Promise<{
    nodes: any[];
    edges: GraphEdge[];
    elkResult: any;
  }>;
}

/**
 * Create an ELK state manager that wraps all ELK layout interactions
 * with proper state management as the single source of truth.
 */
export function createELKStateManager(): ELKStateManager {
  const elk = new ELK();

  /**
   * Validate that nodes fit within their parent containers
   */
  function validateContainment(layoutedNodes: any[], containers: Container[]) {
    console.log('[ELKStateManager] 🔍 Checking containment relationships...');
    
    let validationErrors = 0;
    
    containers.forEach(container => {
      const containerNode = layoutedNodes.find(n => n.id === container.id);
      if (!containerNode) {
        console.warn(`[ELKStateManager] ⚠️ Container ${container.id} not found in layout result`);
        return;
      }
      
      // Find child nodes
      const childNodes = layoutedNodes.filter(node => 
        container.children.has(node.id)
      );
      
      console.log(`[ELKStateManager] 📦 Validating container ${container.id}:`);
      console.log(`  Container bounds: (${containerNode.position?.x || 0}, ${containerNode.position?.y || 0}) ${containerNode.width}x${containerNode.height}`);
      console.log(`  Child nodes: ${childNodes.length}`);
      
      childNodes.forEach(childNode => {
        const childX = childNode.position?.x || 0;
        const childY = childNode.position?.y || 0;
        const childWidth = childNode.width || 0;
        const childHeight = childNode.height || 0;
        
        // For child nodes, ELK coordinates are relative to parent container (0,0 is top-left of container)
        // So we need to check against container dimensions (starting from 0,0), not container position
        const containerWidth = containerNode.width || 0;
        const containerHeight = containerNode.height || 0;
        
        // Check if child fits within container bounds (relative coordinates)
        const childRight = childX + childWidth;
        const childBottom = childY + childHeight;
        
        const fitsHorizontally = childX >= 0 && childRight <= containerWidth;
        const fitsVertically = childY >= 0 && childBottom <= containerHeight;
        
        if (!fitsHorizontally || !fitsVertically) {
          console.error(`[ELKStateManager] ❌ CONTAINMENT VIOLATION: Node ${childNode.id} does not fit in container ${container.id}`);
          console.error(`  Child (relative): (${childX}, ${childY}) ${childWidth}x${childHeight} -> (${childRight}, ${childBottom})`);
          console.error(`  Container bounds: (0, 0) ${containerWidth}x${containerHeight} -> (${containerWidth}, ${containerHeight})`);
          console.error(`  Fits horizontally: ${fitsHorizontally}, Fits vertically: ${fitsVertically}`);
          validationErrors++;
        } else {
          console.log(`[ELKStateManager] ✅ Node ${childNode.id} fits in container ${container.id}`);
        }
      });
    });
    
    if (validationErrors > 0) {
      console.error(`[ELKStateManager] ❌ Found ${validationErrors} containment violations!`);
    } else {
      console.log('[ELKStateManager] ✅ All containment relationships are valid');
    }
  }

  /**
   * Get ELK configuration for different contexts
   */
  function getELKConfig(layoutType: string = 'layered', context: string = 'root'): Record<string, any> {
    const algorithm = ELK_ALGORITHMS[layoutType as keyof typeof ELK_ALGORITHMS] || ELK_ALGORITHMS.LAYERED;
    
    const baseConfig = {
      'elk.algorithm': algorithm,
      'elk.direction': 'DOWN',
      'elk.spacing.nodeNode': LAYOUT_SPACING.NODE_TO_NODE_NORMAL.toString(),
      'elk.spacing.edgeEdge': LAYOUT_SPACING.EDGE_TO_EDGE.toString(),
      'elk.spacing.edgeNode': LAYOUT_SPACING.EDGE_TO_NODE.toString(),
      'elk.spacing.componentComponent': LAYOUT_SPACING.COMPONENT_TO_COMPONENT.toString(),
    };

    if (context === 'root') {
      return {
        ...baseConfig,
        'elk.padding.left': LAYOUT_SPACING.ROOT_PADDING.toString(),
        'elk.padding.right': LAYOUT_SPACING.ROOT_PADDING.toString(),
        'elk.padding.top': LAYOUT_SPACING.ROOT_PADDING.toString(),
        'elk.padding.bottom': LAYOUT_SPACING.ROOT_PADDING.toString(),
      };
    }

    if (context === 'container') {
      return {
        ...baseConfig,
        'elk.padding.left': LAYOUT_SPACING.CONTAINER_PADDING.toString(),
        'elk.padding.right': LAYOUT_SPACING.CONTAINER_PADDING.toString(),
        'elk.padding.top': LAYOUT_SPACING.CONTAINER_PADDING.toString(),
        'elk.padding.bottom': LAYOUT_SPACING.CONTAINER_PADDING.toString(),
      };
    }

    return baseConfig;
  }

  /**
   * Calculate full layout for dimension caching (expanded state).
   * This is used to populate the dimension cache with expanded container sizes.
   */
  async function calculateFullLayout(
    nodes: GraphNode[],
    edges: GraphEdge[],
    containers: Container[],
    layoutType: string = 'layered'
  ): Promise<{
    nodes: any[];
    edges: GraphEdge[];
  }> {
    console.log(`[ELKStateManager] 🏗️ FULL_LAYOUT: Calculating expanded layout for dimension caching`);
    
    console.log('[ELKStateManager] 📊 SUMMARY:');
    console.log(`  Nodes: ${nodes.length}`);
    console.log(`  Containers: ${containers.length}`);
    containers.forEach(container => {
      console.log(`    Container ${container.id}: ${container.children.size} children`);
    });
    console.log(`  Edges: ${edges.length}`);

    const regularNodes = nodes.filter(node => node.type !== 'container');
    const containerNodes = containers;

    // Build ELK hierarchy structure
    function buildElkHierarchy(parentId: string | null = null): any[] {
      const children: any[] = [];
      
      // Add containers at this level
      const levelContainers = containerNodes.filter(container => {
        // Find containers that belong to this parent level
        if (parentId === null) {
          // Root level - containers not contained by any other container
          const isRoot = !containerNodes.some(otherContainer => 
            otherContainer.children.has(container.id)
          );
          return isRoot;
        } else {
          // Non-root level - containers contained by the parent
          const parentContainer = containerNodes.find(c => c.id === parentId);
          const isChild = parentContainer?.children.has(container.id);
          return isChild;
        }
      });

      levelContainers.forEach(container => {
        // Recursively build children for this container
        const childElkNodes = buildElkHierarchy(container.id);
        
        const elkContainer = {
          id: container.id,
          layoutOptions: getELKConfig(layoutType, 'container'),
          children: childElkNodes,
          // Let ELK calculate container size for dimension caching - DON'T specify width/height
        };
        children.push(elkContainer);
      });

      // Add regular nodes at this level
      const levelNodes = regularNodes.filter(node => {
        if (parentId === null) {
          // Root level - nodes not contained by any container
          const isRoot = !containerNodes.some(container => 
            container.children.has(node.id)
          );
          return isRoot;
        } else {
          // Non-root level - nodes contained by the parent
          const parentContainer = containerNodes.find(c => c.id === parentId);
          const isChild = parentContainer?.children.has(node.id);
          return isChild;
        }
      });

      levelNodes.forEach(node => {
        const width = node.dimensions?.width || 180;
        const height = node.dimensions?.height || 60;
        
        const elkNode = {
          id: node.id,
          width: width,
          height: height,
        };
        children.push(elkNode);
      });

      return children;
    }

    // Build the ELK graph with hierarchy
    const elkGraph = {
      id: 'full_layout_root',
      layoutOptions: getELKConfig(layoutType, 'root'),
      children: buildElkHierarchy(null),
      edges: edges.map(edge => ({
        id: edge.id,
        sources: [edge.source],
        targets: [edge.target],
      })),
    };

    try {
      const layoutResult = await elk.layout(elkGraph);

      console.log('[ELKStateManager] 🔍 ELK CONTAINER INPUT:');
      
      // Debug input hierarchy - focus on containers only
      function logELKContainerInput(nodes: any[], depth: number = 0) {
        const indent = '  '.repeat(depth);
        nodes.forEach(node => {
          if (node.children && node.children.length > 0) {
            // This is a container
            console.log(`${indent}📦 CONTAINER INPUT ${node.id}: children=${node.children.length}, width=${node.width || 'undefined'}, height=${node.height || 'undefined'}`);
            console.log(`${indent}   layoutOptions:`, node.layoutOptions);
            logELKContainerInput(node.children, depth + 1);
          }
        });
      }
      logELKContainerInput(elkGraph.children);

      console.log('[ELKStateManager] 🔍 ELK CONTAINER OUTPUT:');
      
      // Debug output hierarchy - focus on containers only  
      function logELKContainerOutput(nodes: any[], depth: number = 0) {
        const indent = '  '.repeat(depth);
        nodes.forEach(node => {
          if (node.children && node.children.length > 0) {
            // This is a container
            console.log(`${indent}📦 CONTAINER OUTPUT ${node.id}: children=${node.children.length}, x=${node.x}, y=${node.y}, width=${node.width}, height=${node.height}`);
            logELKContainerOutput(node.children, depth + 1);
          }
        });
      }
      
      if (layoutResult.children) {
        logELKContainerOutput(layoutResult.children);
      }

      // Apply positions back to nodes
      function applyPositions(elkNodes: any[], depth: number = 0): any[] {
        const layoutedNodes: any[] = [];
        
        elkNodes.forEach(elkNode => {
          // Find original node or container
          const originalNode = nodes.find(n => n.id === elkNode.id);
          const originalContainer = containers.find(c => c.id === elkNode.id);
          const original = originalNode || originalContainer;
          
          if (original) {
            const processedNode = {
              ...original,
              width: elkNode.width,
              height: elkNode.height,
              position: {
                x: elkNode.x || 0,
                y: elkNode.y || 0,
              },
              dimensions: {
                width: elkNode.width,
                height: elkNode.height,
              },
            };
            layoutedNodes.push(processedNode);
          }
          
          // Recursively apply positions to children
          if (elkNode.children) {
            layoutedNodes.push(...applyPositions(elkNode.children, depth + 1));
          }
        });
        
        return layoutedNodes;
      }

      const layoutedNodes = applyPositions(layoutResult.children || []);

      // Validate containment relationships
      console.log('[ELKStateManager] 🔍 CONTAINMENT VALIDATION:');
      validateContainment(layoutedNodes, containers);

      // Sort nodes so parents come before children (ReactFlow requirement)
      const sortedNodes: any[] = [];
      const nodeMap = new Map(layoutedNodes.map(node => [node.id, node]));
      const visited = new Set<string>();
      
      function addNodeAndParents(nodeId: string) {
        if (visited.has(nodeId)) return;
        
        const node = nodeMap.get(nodeId);
        if (!node) return;
        
        // Find parent container
        const parentContainer = containers.find(container => 
          container.children.has(nodeId)
        );
        
        if (parentContainer && !visited.has(parentContainer.id)) {
          addNodeAndParents(parentContainer.id);
        }
        
        visited.add(nodeId);
        sortedNodes.push(node);
      }
      
      layoutedNodes.forEach(node => addNodeAndParents(node.id));
      
      return {
        nodes: sortedNodes,
        edges: edges,
      };

    } catch (error) {
      console.error('[ELKStateManager] Full layout failed:', error);
      throw error;
    }
  }

  /**
   * Calculate layout based on current visualization state.
   * This handles visible/hidden containers and collapsed states.
   */
  async function calculateVisualLayout(
    nodes: GraphNode[],
    edges: GraphEdge[],
    containers: Container[],
    hyperEdges: HyperEdge[],
    layoutType: string = 'layered',
    dimensionsCache?: Map<string, LayoutDimensions>
  ): Promise<{
    nodes: any[];
    edges: GraphEdge[];
    elkResult: any;
  }> {
    console.log(`[ELKStateManager] 🎯 VISUAL_LAYOUT: Calculating layout for current state`);
    
    // For now, use the full layout approach
    // In the future, this would filter based on collapsed/expanded states
    const result = await calculateFullLayout(nodes, edges, containers, layoutType);
    
    return {
      ...result,
      elkResult: null, // Will contain ELK raw result when needed
    };
  }

  return {
    calculateFullLayout,
    calculateVisualLayout,
  };
}
