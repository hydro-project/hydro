/**
 * @fileoverview Enhanced ELK Layout Engine with proper container hierarchy support
 */

import ELK from 'elkjs';
import type { GraphNode, GraphEdge, Container, HyperEdge } from '../shared/types';
import type { LayoutConfig, LayoutResult, LayoutEngine } from './types';
import { ELK_LAYOUT_CONFIG, LAYOUT_SPACING } from '../shared/config';

export class ELKLayoutEngine implements LayoutEngine {
  private elk = new ELK();

  async layout(
    nodes: GraphNode[],
    edges: GraphEdge[],
    containers: Container[],
    hyperEdges: HyperEdge[],
    config: LayoutConfig = {}
  ): Promise<LayoutResult> {
    const finalConfig = { ...ELK_LAYOUT_CONFIG.DEFAULT, ...config };
    
    console.log('ELK Layout Input:', {
      nodes: nodes.length,
      edges: edges.length,
      containers: containers.length,
      hyperEdges: hyperEdges.length
    });

    // Build proper ELK graph with container hierarchy
    const elkGraph = this.buildELKGraph(nodes, edges, containers, hyperEdges, finalConfig);
    
    console.log('ELK Graph Structure built with', elkGraph.children.length, 'root elements');

    // Run ELK layout
    const layouted = await this.elk.layout(elkGraph);
    
    console.log('ELK Layout completed successfully');

    // Convert back to our format
    return this.convertELKResult(layouted, nodes, edges, containers, hyperEdges);
  }

  /**
   * Build proper ELK graph structure with container hierarchy
   */
  private buildELKGraph(
    nodes: GraphNode[],
    edges: GraphEdge[],
    containers: Container[],
    hyperEdges: HyperEdge[],
    config: any
  ) {
    // Build container hierarchy map
    const containerHierarchy = this.buildContainerHierarchy(containers);
    
    // Find root containers (containers that are not children of other containers)
    const rootContainers = containers.filter(container => 
      !containers.some(otherContainer => 
        otherContainer.children.has(container.id)
      )
    );

    // Find standalone nodes (nodes not contained in any container)
    const standaloneNodes = nodes.filter(node => 
      !containers.some(container => container.children.has(node.id))
    );

    const elkGraph = {
      id: 'root',
      layoutOptions: {
        'elk.algorithm': config.algorithm,
        'elk.direction': config.direction,
        'elk.spacing.nodeNode': config.spacing.toString(),
        'elk.spacing.componentComponent': LAYOUT_SPACING.COMPONENT_TO_COMPONENT.toString(),
        'elk.padding.left': LAYOUT_SPACING.ROOT_PADDING.toString(),
        'elk.padding.right': LAYOUT_SPACING.ROOT_PADDING.toString(),
        'elk.padding.top': LAYOUT_SPACING.ROOT_PADDING.toString(),
        'elk.padding.bottom': LAYOUT_SPACING.ROOT_PADDING.toString(),
      },
      children: [
        // Add root containers with their hierarchy
        ...rootContainers.map(container => this.buildContainerNode(container, nodes, containers, config)),
        // Add standalone nodes
        ...standaloneNodes.map(node => this.buildStandaloneNode(node, config))
      ],
      edges: this.buildELKEdges(edges, hyperEdges)
    };

    return elkGraph;
  }

  /**
   * Build container hierarchy map for nested containers
   */
  private buildContainerHierarchy(containers: Container[]): Map<string, Container[]> {
    const hierarchy = new Map<string, Container[]>();
    
    containers.forEach(container => {
      container.children.forEach(childId => {
        const childContainer = containers.find(c => c.id === childId);
        if (childContainer) {
          if (!hierarchy.has(container.id)) {
            hierarchy.set(container.id, []);
          }
          hierarchy.get(container.id)!.push(childContainer);
        }
      });
    });
    
    return hierarchy;
  }

  /**
   * Calculate proper container dimensions based on contained elements
   */
  private calculateContainerDimensions(
    containedNodes: GraphNode[],
    childContainers: Container[],
    config: any,
    container: Container
  ): { width: number; height: number } {
    // Use explicit dimensions if available
    if (container.expandedDimensions?.width && container.expandedDimensions?.height) {
      return {
        width: container.expandedDimensions.width,
        height: container.expandedDimensions.height
      };
    }

    // Constants for calculation
    const nodeWidth = 180;  // Default node width
    const nodeHeight = 60;  // Default node height
    const padding = 40;     // Container padding
    const spacing = 20;     // Inter-element spacing
    const minWidth = 250;   // Minimum container width
    const minHeight = 150;  // Minimum container height

    // Calculate required space for direct nodes
    const directNodesCount = containedNodes.length;
    const directChildContainers = childContainers.length;
    const totalElements = directNodesCount + directChildContainers;

    if (totalElements === 0) {
      return { width: minWidth, height: minHeight };
    }

    // Calculate layout dimensions
    let requiredWidth = minWidth;
    let requiredHeight = minHeight;

    if (totalElements > 0) {
      // Estimate layout based on element count and arrangement
      const elementsPerRow = Math.max(1, Math.min(4, Math.ceil(Math.sqrt(totalElements))));
      const rows = Math.ceil(totalElements / elementsPerRow);
      
      // Calculate width: elements + spacing + padding
      requiredWidth = Math.max(
        minWidth,
        (elementsPerRow * nodeWidth) + ((elementsPerRow - 1) * spacing) + (2 * padding)
      );
      
      // Calculate height: rows + spacing + padding
      requiredHeight = Math.max(
        minHeight,
        (rows * nodeHeight) + ((rows - 1) * spacing) + (2 * padding)
      );

      // Add extra space for child containers (they need more room)
      if (directChildContainers > 0) {
        requiredHeight += directChildContainers * 100; // Extra height for nested containers
        requiredWidth = Math.max(requiredWidth, 400); // Minimum width for containers
      }
    }

    return {
      width: Math.round(requiredWidth),
      height: Math.round(requiredHeight)
    };
  }

  /**
   * Recursively build ELK container node with proper containment
   */
  private buildContainerNode(
    container: Container,
    allNodes: GraphNode[],
    allContainers: Container[],
    config: any
  ): any {
    // Get nodes directly contained in this container
    const containedNodes = allNodes.filter(node => container.children.has(node.id));
    
    // Get child containers
    const childContainers = allContainers.filter(childContainer => 
      container.children.has(childContainer.id)
    );

    // Calculate proper container dimensions
    const { width: expandedWidth, height: expandedHeight } = this.calculateContainerDimensions(
      containedNodes,
      childContainers,
      config,
      container
    );

    console.log(`Container ${container.id}: calculated dimensions ${expandedWidth}x${expandedHeight} for ${containedNodes.length} nodes + ${childContainers.length} containers`);

    const elkContainer = {
      id: container.id,
      width: expandedWidth,
      height: expandedHeight,
      layoutOptions: {
        'elk.padding.left': LAYOUT_SPACING.CONTAINER_PADDING.toString(),
        'elk.padding.right': LAYOUT_SPACING.CONTAINER_PADDING.toString(),
        'elk.padding.top': LAYOUT_SPACING.CONTAINER_PADDING.toString(),
        'elk.padding.bottom': LAYOUT_SPACING.CONTAINER_PADDING.toString(),
        'elk.spacing.nodeNode': LAYOUT_SPACING.NODE_TO_NODE_NORMAL.toString(),
      },
      children: [
        // Add contained nodes
        ...containedNodes.map(node => ({
          id: node.id,
          width: config.nodeSize.width,
          height: config.nodeSize.height
        })),
        // Recursively add child containers
        ...childContainers.map(childContainer => 
          this.buildContainerNode(childContainer, allNodes, allContainers, config)
        )
      ]
    };

    console.log(`Container ${container.id}: ${containedNodes.length} nodes, ${childContainers.length} child containers`);

    return elkContainer;
  }

  /**
   * Build standalone node (not in any container)
   */
  private buildStandaloneNode(node: GraphNode, config: any): any {
    return {
      id: node.id,
      width: config.nodeSize.width,
      height: config.nodeSize.height
    };
  }

  /**
   * Build ELK edges from graph edges and hyper edges
   */
  private buildELKEdges(edges: GraphEdge[], hyperEdges: HyperEdge[]): any[] {
    return [
      ...edges.map(edge => ({
        id: edge.id,
        sources: [edge.source],
        targets: [edge.target]
      })),
      ...hyperEdges.map(edge => ({
        id: edge.id,
        sources: [edge.source],
        targets: [edge.target]
      }))
    ];
  }

  /**
   * Convert ELK layout result back to our format
   */
  private convertELKResult(
    layouted: any,
    originalNodes: GraphNode[],
    originalEdges: GraphEdge[],
    originalContainers: Container[],
    originalHyperEdges: HyperEdge[]
  ): LayoutResult {
    const nodeMap = new Map<string, any>();
    const edgeMap = new Map<string, any>();

    // Process nodes and containers recursively
    this.processELKNodes(layouted.children || [], originalNodes, originalContainers, nodeMap, 0, 0);

    // Process edges
    layouted.edges?.forEach((elkEdge: any) => {
      const originalEdge = originalEdges.find(e => e.id === elkEdge.id) || 
                          originalHyperEdges.find(e => e.id === elkEdge.id);
      if (originalEdge) {
        edgeMap.set(elkEdge.id, {
          ...originalEdge,
          points: elkEdge.sections?.[0]?.bendPoints?.map((bp: any) => ({ x: bp.x, y: bp.y }))
        });
      }
    });

    const result = {
      nodes: Array.from(nodeMap.values()).filter(n => originalNodes.some(node => node.id === n.id)),
      edges: Array.from(edgeMap.values()).filter(e => originalEdges.some(edge => edge.id === e.id)),
      containers: Array.from(nodeMap.values()).filter(n => originalContainers.some(container => container.id === n.id)),
      hyperEdges: Array.from(edgeMap.values()).filter(e => originalHyperEdges.some(edge => edge.id === e.id))
    };

    console.log('Layout conversion completed:', {
      nodes: result.nodes.length,
      edges: result.edges.length,
      containers: result.containers.length,
      hyperEdges: result.hyperEdges.length
    });
    
    return result;
  }

  /**
   * Recursively process ELK nodes and containers
   */
  private processELKNodes(
    elkNodes: any[],
    originalNodes: GraphNode[],
    originalContainers: Container[],
    nodeMap: Map<string, any>,
    offsetX: number,
    offsetY: number
  ): void {
    elkNodes.forEach(elkNode => {
      const originalNode = originalNodes.find(n => n.id === elkNode.id);
      const originalContainer = originalContainers.find(c => c.id === elkNode.id);
      const original = originalNode || originalContainer;

      if (original && elkNode.x !== undefined && elkNode.y !== undefined) {
        const positioned: any = {
          ...original,
          x: elkNode.x + offsetX,
          y: elkNode.y + offsetY,
          width: elkNode.width,
          height: elkNode.height
        };

        // For containers, update expanded dimensions
        if (originalContainer) {
          positioned.expandedDimensions = {
            width: elkNode.width,
            height: elkNode.height
          };
        }

        nodeMap.set(elkNode.id, positioned);

        // Log container positioning only
        if (originalContainer) {
          console.log(`Positioned container ${original.id}:`, {
            x: positioned.x,
            y: positioned.y,
            width: positioned.width,
            height: positioned.height
          });
        }
      }

      // Recursively process children
      if (elkNode.children) {
        this.processELKNodes(
          elkNode.children,
          originalNodes,
          originalContainers,
          nodeMap,
          elkNode.x + offsetX,
          elkNode.y + offsetY
        );
      }
    });
  }
}
