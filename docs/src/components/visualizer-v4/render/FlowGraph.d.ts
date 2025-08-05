/**
 * @fileoverview Bridge-Based FlowGraph Component
 *
 * Complete replacement for alpha FlowGraph using o        console.log('[FlowGraph] ✅ Updated ReactFlow data:', {
          nodes: dataWithManualPositions.nodes.length,
          edges: dataWithManualPositions.edges.length
        });
        
        setReactFlowData(dataWithManualPositions);ture.
 * Maintains identical API while using the new VisualizationEngine internally.
 */
import React from 'react';
import '@xyflow/react/dist/style.css';
import type { VisualizationState } from '../core/VisState';
import type { RenderConfig, FlowGraphEventHandlers, LayoutConfig } from '../core/types';
export interface FlowGraphProps {
    visualizationState: VisualizationState;
    config?: RenderConfig;
    layoutConfig?: LayoutConfig;
    eventHandlers?: FlowGraphEventHandlers;
    className?: string;
    style?: React.CSSProperties;
}
export interface FlowGraphRef {
    fitView: () => void;
}
export declare function FlowGraph({ visualizationState, config, layoutConfig, eventHandlers, className, style }: FlowGraphProps): JSX.Element;
//# sourceMappingURL=FlowGraph.d.ts.map