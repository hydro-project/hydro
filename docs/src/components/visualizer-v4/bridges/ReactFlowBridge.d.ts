/**
 * @fileoverview ReactFlow Bridge - Converts VisualizationState to ReactFlow format
 *
 * This bridge converts VisualizationState to ReactFlow's expected data structures.
 * ReactFlow only sees unified edges (hyperedges are included transparently).
 * Uses configurable handle system for maximum layout flexibility.
 */
import type { VisualizationState } from '../core/VisState';
import { MarkerType } from '@xyflow/react';
export interface ReactFlowNode {
    id: string;
    type: 'standard' | 'container';
    position: {
        x: number;
        y: number;
    };
    data: {
        label: string;
        style: string;
        collapsed?: boolean;
        width?: number;
        height?: number;
        [key: string]: any;
    };
    style?: {
        width?: number;
        height?: number;
    };
    parentId?: string;
}
export interface ReactFlowEdge {
    id: string;
    type: 'standard' | 'hyper';
    source: string;
    target: string;
    sourceHandle?: string;
    targetHandle?: string;
    markerEnd?: {
        type: typeof MarkerType.ArrowClosed;
        width: number;
        height: number;
        color: string;
    };
    data: {
        style: string;
    };
}
export interface ReactFlowData {
    nodes: ReactFlowNode[];
    edges: ReactFlowEdge[];
}
export declare class ReactFlowBridge {
    /**
     * Convert positioned VisState data to ReactFlow format
     * Pure data transformation - no layout logic
     */
    visStateToReactFlow(visState: VisualizationState): ReactFlowData;
    /**
     * Build parent-child relationship map
     */
    private buildParentMap;
    /**
     * Convert containers to ReactFlow container nodes
     */
    private convertContainers;
    /**
     * Convert regular nodes to ReactFlow standard nodes
     */
    private convertNodes;
    /**
     * Convert regular edges to ReactFlow edges
     */
    private convertEdges;
    /**
     * Extract custom properties from graph elements
     */
    private extractCustomProperties;
}
//# sourceMappingURL=ReactFlowBridge.d.ts.map