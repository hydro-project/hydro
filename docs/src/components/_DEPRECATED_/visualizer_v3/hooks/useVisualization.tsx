/**
 * @fileoverview React Hook for Visualization Engine
 * 
 * Clean React integration for the visualization system:
 * - Manages VisualizationEngine lifecycle
 * - Provides reactive state updates
 * - Handles data loading and layout triggers
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import { VisualizationEngine, createVisualizationEngine } from '../core/VisualizationEngine';
import type { VisualizationEngineState, VisualizationEngineConfig } from '../core/VisualizationEngine';
import type { ReactFlowData } from '../bridges/ReactFlowBridge';
import type { VisualizationState } from '../core/VisState';

export interface UseVisualizationResult {
  // Data
  reactFlowData: ReactFlowData | null;
  engineState: VisualizationEngineState;
  
  // Actions
  runLayout: () => Promise<void>;
  visualize: () => Promise<void>;
  onDataChanged: () => void;
  
  // State
  isLoading: boolean;
  isReady: boolean;
  hasError: boolean;
  error: string | undefined;
}

export interface UseVisualizationConfig extends Partial<VisualizationEngineConfig> {
  // Additional React-specific config
  autoVisualize?: boolean;  // Automatically call visualize() after layout
}

const DEFAULT_CONFIG: UseVisualizationConfig = {
  autoLayout: true,
  layoutDebounceMs: 300,
  enableLogging: true,
  autoVisualize: true
};

/**
 * React hook for managing visualization pipeline
 */
export function useVisualization(
  visState: VisualizationState,
  config: UseVisualizationConfig = {}
): UseVisualizationResult {
  const finalConfig = { ...DEFAULT_CONFIG, ...config };
  
  // Engine state
  const engineRef = useRef<VisualizationEngine | null>(null);
  const [engineState, setEngineState] = useState<VisualizationEngineState>({
    phase: 'initial',
    lastUpdate: Date.now(),
    layoutCount: 0
  });
  
  // React state
  const [reactFlowData, setReactFlowData] = useState<ReactFlowData | null>(null);
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
        } catch (error) {
          console.error('[useVisualization] Auto-visualize failed:', error);
        }
        setIsLoading(false);
      }
      
      // Handle loading state
      if (state.phase === 'laying_out' || state.phase === 'rendering') {
        setIsLoading(true);
      } else {
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
    if (!engineRef.current) return;
    
    console.log('[useVisualization] 📊 Running layout...');
    setIsLoading(true);
    
    try {
      await engineRef.current.runLayout();
    } catch (error) {
      console.error('[useVisualization] Layout failed:', error);
      setIsLoading(false);
    }
  }, []);

  const visualize = useCallback(async () => {
    if (!engineRef.current) return;
    
    console.log('[useVisualization] 🎨 Running full visualization...');
    setIsLoading(true);
    
    try {
      const data = await engineRef.current.visualize();
      setReactFlowData(data);
      setIsLoading(false);
    } catch (error) {
      console.error('[useVisualization] Visualization failed:', error);
      setIsLoading(false);
    }
  }, []);

  const onDataChanged = useCallback(() => {
    if (!engineRef.current) return;
    
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

/**
 * Higher-order component that provides visualization context
 */
export interface VisualizationProviderProps {
  children: React.ReactNode;
  visState: VisualizationState;
  config?: UseVisualizationConfig;
}

export function VisualizationProvider({ 
  children, 
  visState, 
  config 
}: VisualizationProviderProps): JSX.Element {
  const visualization = useVisualization(visState, config);
  
  // You could use React Context here to provide visualization to children
  // For now, just render children
  return <>{children}</>;
}
