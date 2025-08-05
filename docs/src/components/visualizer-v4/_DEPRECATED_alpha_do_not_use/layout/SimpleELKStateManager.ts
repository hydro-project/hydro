/**
 * Simplified ELK State Manager - Pure Format Translator
 * 
 * This is what ELKStateManager should be: a simple format translator with no conditional logic.
 * VisState is the single source of truth, ELK is just a layout algorithm.
 * 
 * Responsibilities:
 * - Convert VisState data to ELK format
 * - Run ELK layout
 * - Convert ELK results back to VisState format
 * - NO conditional logic, NO state management, NO decision making
 */

import { LOG_PREFIXES } from '../shared/constants';

export class SimpleELKStateManager {
  
  /**
   * Pure format translation: VisState → ELK Layout → VisState
   */
  async calculateLayout(
    visibleNodes: any[],
    visibleEdges: any[], 
    visibleContainers: any[],
    visibleHyperEdges: any[]
  ) {
    console.log(`${LOG_PREFIXES.STATE_MANAGER} 🔄 TRANSLATE: Converting VisState to ELK format`);
    
    // Step 1: Pure format translation VisState → ELK
    const elkGraph = this.translateVisStateToELK(visibleNodes, visibleEdges, visibleContainers, visibleHyperEdges);
    
    // Step 2: Run ELK (the only place with actual logic)
    console.log(`${LOG_PREFIXES.STATE_MANAGER} ⚡ ELK: Running layout algorithm`);
    const elkResult = await this.runELKLayout(elkGraph);
    
    // Step 3: Pure format translation ELK → VisState compatible format
    const layoutResult = this.translateELKToVisState(elkResult);
    
    console.log(`${LOG_PREFIXES.STATE_MANAGER} ✅ TRANSLATE: Conversion complete`);
    return layoutResult;
  }

  /**
   * Pure format translator: VisState objects → ELK format
   * Zero conditional logic - just extract data that's already in VisState
   */
  private translateVisStateToELK(nodes: any[], edges: any[], containers: any[], hyperEdges: any[]) {
    // Convert nodes: extract layout data that VisState already has
    const elkNodes = nodes.map(node => ({
      id: node.id,
      width: node.layout?.dimensions?.width || 180,
      height: node.layout?.dimensions?.height || 60,
      x: node.layout?.position?.x,
      y: node.layout?.position?.y
    }));

    // Convert containers: extract layout data that VisState already has  
    const elkContainers = containers.map(container => ({
      id: container.id,
      width: container.layout?.dimensions?.width,
      height: container.layout?.dimensions?.height,
      x: container.layout?.position?.x,
      y: container.layout?.position?.y,
      children: [], // ELK will populate based on hierarchy
      layoutOptions: container.layout?.elkOptions || {
        'elk.algorithm': 'layered',
        'elk.direction': 'DOWN'
      }
    }));

    // Convert edges: just map source/target
    const elkEdges = [...edges, ...hyperEdges].map(edge => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target]
    }));

    return {
      id: 'root',
      children: [...elkNodes, ...elkContainers],
      edges: elkEdges
    };
  }

  /**
   * Pure format translator: ELK results → VisState compatible format
   * Zero conditional logic - just extract positions and dimensions
   */
  private translateELKToVisState(elkResult: any) {
    const layoutedElements = [];

    // Extract layout data for all elements
    function extractLayout(elkNode: any) {
      layoutedElements.push({
        id: elkNode.id,
        position: { x: elkNode.x || 0, y: elkNode.y || 0 },
        dimensions: { width: elkNode.width || 0, height: elkNode.height || 0 }
      });

      // Recursively process children
      if (elkNode.children) {
        elkNode.children.forEach(extractLayout);
      }
    }

    extractLayout(elkResult);
    return layoutedElements;
  }

  /**
   * Run ELK layout - the only place with actual algorithm logic
   */
  private async runELKLayout(elkGraph: any) {
    const ELK = await import('elkjs');
    const elk = new ELK.default();
    return elk.layout(elkGraph);
  }
}

/**
 * Usage Example:
 * 
 * const manager = new SimpleELKStateManager();
 * const results = await manager.calculateLayout(
 *   visState.visibleNodes,
 *   visState.visibleEdges, 
 *   visState.visibleContainers,
 *   visState.allHyperEdges
 * );
 * 
 * // Apply results back to VisState
 * results.forEach(result => {
 *   if (visState.getGraphNode(result.id)) {
 *     visState.setNodeLayout(result.id, result);
 *   } else if (visState.getContainer(result.id)) {
 *     visState.setContainerLayout(result.id, result);
 *   }
 * });
 */
