/**
 * @fileoverview Visualization Component - Clean React integration
 *
 * Demonstrates the new bridge architecture:
 * - Uses VisualizationEngine for orchestration
 * - Handles loading and error states
 * - Provides clean interface for ReactFlow integration
 */
import React from 'react';
import '@xyflow/react/dist/style.css';
import type { VisualizationState } from '../core/VisState';
import type { UseVisualizationConfig } from '../hooks/useVisualization';
export interface VisualizationComponentProps {
    visState: VisualizationState;
    config?: UseVisualizationConfig;
    className?: string;
    style?: React.CSSProperties;
}
export declare function VisualizationComponent({ visState, config, className, style }: VisualizationComponentProps): JSX.Element;
/**
 * Example usage component
 */
export interface ExampleVisualizationProps {
    visState: VisualizationState;
}
export declare function ExampleVisualization({ visState }: ExampleVisualizationProps): JSX.Element;
//# sourceMappingURL=VisualizationComponent.d.ts.map