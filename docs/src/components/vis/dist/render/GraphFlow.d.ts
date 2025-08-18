/**
 * @fileoverview Main ReactFlow Visualization Component
 *
 * The primary component that renders generic graphs using ReactFlow.
 * Independent of any specific framework - receives data via JSON/props.
 */
import React from 'react';
import '@xyflow/react/dist/style.css';
import { VisualizationState } from '../shared/types';
import { LayoutConfig } from '../layout/index';
import { RenderConfig, GraphFlowEventHandlers } from './types';
export interface GraphFlowProps {
    visualizationState: VisualizationState;
    metadata?: {
        nodeTypeConfig?: any;
        [key: string]: any;
    };
    layoutConfig?: Partial<LayoutConfig>;
    renderConfig?: Partial<RenderConfig>;
    eventHandlers?: Partial<GraphFlowEventHandlers>;
    onLayoutComplete?: () => void;
    onError?: (error: Error) => void;
    className?: string;
    style?: React.CSSProperties;
}
export declare const GraphFlow: React.FC<GraphFlowProps>;
export default GraphFlow;
//# sourceMappingURL=GraphFlow.d.ts.map