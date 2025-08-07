/**
 * @fileoverview Bridge-Based ReactFlow Converter
 * 
 * Complete replacement for alpha ReactFlowConverter using our bridge architecture.
 * Maintains identical API while using the new ReactFlowBridge internally.
 */

import { ReactFlowBridge } from '../bridges/ReactFlowBridge';
import type { VisualizationState } from '../core/VisState';
import type { ReactFlowData } from '../bridges/ReactFlowBridge';

export class ReactFlowConverter {
  private bridge: ReactFlowBridge;

  constructor() {
    this.bridge = new ReactFlowBridge();
  }

  /**
   * Set the color palette for node styling
   */
  setColorPalette(palette: string): void {
    this.bridge.setColorPalette(palette);
  }

  /**
   * Convert VisualizationState to ReactFlow format - SAME API as alpha
   */
  convert(visState: VisualizationState): ReactFlowData {
    console.log('[ReactFlowConverter] 🔄 Converting with bridge architecture...');
    return this.bridge.visStateToReactFlow(visState);
  }

  /**
   * Legacy method for compatibility
   */
  convertNodes(nodes: any[]): any[] {
    console.log('[ReactFlowConverter] ⚠️ convertNodes is deprecated, use convert() instead');
    return nodes.map(node => ({
      id: node.id,
      type: 'default',
      position: { x: node.x || 0, y: node.y || 0 },
      data: { label: node.label || node.id }
    }));
  }

  /**
   * Legacy method for compatibility
   */
  convertEdges(edges: any[]): any[] {
    console.log('[ReactFlowConverter] ⚠️ convertEdges is deprecated, use convert() instead');
    return edges.map(edge => ({
      id: edge.id,
      source: edge.source,
      target: edge.target,
      type: 'standard'
    }));
  }
}
