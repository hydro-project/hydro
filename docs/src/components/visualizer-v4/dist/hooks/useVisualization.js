import { Fragment as _Fragment, jsx as _jsx } from "react/jsx-runtime";
/**
 * @fileoverview React Hook for Visualization Engine
 *
 * Clean React integration for the visualization system:
 * - Manages VisualizationEngine lifecycle
 * - Provides reactive state updates
 * - Handles data loading and layout triggers
 */
import { useState, useEffect, useCallback, useRef } from 'react';
import { createVisualizationEngine } from '../core/VisualizationEngine';
const DEFAULT_CONFIG = {
    autoLayout: true,
    layoutDebounceMs: 300,
    enableLogging: true,
    autoVisualize: true
};
/**
 * React hook for managing visualization pipeline
 */
export function useVisualization(visState, config = {}) {
    const finalConfig = { ...DEFAULT_CONFIG, ...config };
    // Engine state
    const engineRef = useRef(null);
    const [engineState, setEngineState] = useState({
        phase: 'initial',
        lastUpdate: Date.now(),
        layoutCount: 0
    });
    // React state
    const [reactFlowData, setReactFlowData] = useState(null);
    const [isLoading, setIsLoading] = useState(false);
    // Initialize engine
    useEffect(() => {
        console.log('[useVisualization] 🚀 Initializing VisualizationEngine');
        const engine = createVisualizationEngine(visState, finalConfig);
        engineRef.current = engine;
        // Listen to engine state changes
        engine.onStateChange('react-hook', (state) => {
            console.log('[useVisualization] 🔄 Engine state changed:', state.phase);
            setEngineState(state);
            // Auto-visualize after layout if enabled
            if (finalConfig.autoVisualize && state.phase === 'ready') {
                try {
                    const data = engine.getReactFlowData();
                    setReactFlowData(data);
                }
                catch (error) {
                    console.error('[useVisualization] Auto-visualize failed:', error);
                }
                setIsLoading(false);
            }
            // Handle loading state
            if (state.phase === 'laying_out' || state.phase === 'rendering') {
                setIsLoading(true);
            }
            else {
                setIsLoading(false);
            }
        });
        return () => {
            console.log('[useVisualization] 🧹 Cleaning up VisualizationEngine');
            engine.dispose();
        };
    }, [visState]); // Re-initialize if visState changes
    // Actions
    const runLayout = useCallback(async () => {
        if (!engineRef.current)
            return;
        console.log('[useVisualization] 📊 Running layout...');
        setIsLoading(true);
        try {
            await engineRef.current.runLayout();
        }
        catch (error) {
            console.error('[useVisualization] Layout failed:', error);
            setIsLoading(false);
        }
    }, []);
    const visualize = useCallback(async () => {
        if (!engineRef.current)
            return;
        console.log('[useVisualization] 🎨 Running full visualization...');
        setIsLoading(true);
        try {
            const data = await engineRef.current.visualize();
            setReactFlowData(data);
            setIsLoading(false);
        }
        catch (error) {
            console.error('[useVisualization] Visualization failed:', error);
            setIsLoading(false);
        }
    }, []);
    const onDataChanged = useCallback(() => {
        if (!engineRef.current)
            return;
        console.log('[useVisualization] 📝 Data changed notification');
        engineRef.current.onDataChanged();
        // Clear old ReactFlow data
        setReactFlowData(null);
    }, []);
    // Derived state
    const isReady = engineState.phase === 'ready' || engineState.phase === 'displayed';
    const hasError = engineState.phase === 'error';
    const error = engineState.error;
    return {
        // Data
        reactFlowData,
        engineState,
        // Actions
        runLayout,
        visualize,
        onDataChanged,
        // State
        isLoading,
        isReady,
        hasError,
        error
    };
}
export function VisualizationProvider({ children, visState, config }) {
    const visualization = useVisualization(visState, config);
    // You could use React Context here to provide visualization to children
    // For now, just render children
    return _jsx(_Fragment, { children: children });
}
//# sourceMappingURL=useVisualization.js.map