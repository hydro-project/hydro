/**
 * @fileoverview Bridge-Based GraphFlow Component
 *
 * Complete replacement for alpha GraphFlow using our bridge architecture.
 * Maintains identical API while using the new VisualizationEngine internally.
 */
import React from 'react';
import '@xyflow/react/dist/style.css';
import type { VisualizationState } from '../core/VisState';
import type { RenderConfig, GraphFlowEventHandlers } from '../core/types';
export interface GraphFlowProps {
    visualizationState: VisualizationState;
    config?: RenderConfig;
    eventHandlers?: GraphFlowEventHandlers;
    className?: string;
    style?: React.CSSProperties;
}
export declare function GraphFlow({ visualizationState, config, eventHandlers, className, style }: GraphFlowProps): JSX.Element;
//# sourceMappingURL=GraphFlow.d.ts.map