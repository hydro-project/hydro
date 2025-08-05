/**
 * @fileoverview React Hook for Visualization Engine
 *
 * Clean React integration for the visualization system:
 * - Manages VisualizationEngine lifecycle
 * - Provides reactive state updates
 * - Handles data loading and layout triggers
 */
import type { VisualizationEngineState, VisualizationEngineConfig } from '../core/VisualizationEngine';
import type { ReactFlowData } from '../bridges/ReactFlowBridge';
import type { VisualizationState } from '../core/VisState';
export interface UseVisualizationResult {
    reactFlowData: ReactFlowData | null;
    engineState: VisualizationEngineState;
    runLayout: () => Promise<void>;
    visualize: () => Promise<void>;
    onDataChanged: () => void;
    isLoading: boolean;
    isReady: boolean;
    hasError: boolean;
    error: string | undefined;
}
export interface UseVisualizationConfig extends Partial<VisualizationEngineConfig> {
    autoVisualize?: boolean;
}
/**
 * React hook for managing visualization pipeline
 */
export declare function useVisualization(visState: VisualizationState, config?: UseVisualizationConfig): UseVisualizationResult;
/**
 * Higher-order component that provides visualization context
 */
export interface VisualizationProviderProps {
    children: React.ReactNode;
    visState: VisualizationState;
    config?: UseVisualizationConfig;
}
export declare function VisualizationProvider({ children, visState, config }: VisualizationProviderProps): JSX.Element;
//# sourceMappingURL=useVisualization.d.ts.map