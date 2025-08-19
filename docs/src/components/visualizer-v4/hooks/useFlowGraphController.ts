/**
 * useFlowGraphController
 * Encapsulates FlowGraph state management: engine/bridge, layout runs,
 * config updates, ReactFlow data, and event handlers. Behavior-preserving.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useReactFlow, applyNodeChanges } from '@xyflow/react';

import { createVisualizationEngine } from '../core/VisualizationEngine';
import { ReactFlowBridge } from '../bridges/ReactFlowBridge';
import { useManualPositions } from './useManualPositions';
import type { VisualizationState } from '../core/VisualizationState';
import type { ReactFlowData } from '../bridges/ReactFlowBridge';
import type { RenderConfig, FlowGraphEventHandlers, LayoutConfig } from '../core/types';

export interface UseFlowGraphControllerArgs {
  visualizationState: VisualizationState;
  config: RenderConfig;
  layoutConfig?: LayoutConfig;
  eventHandlers?: FlowGraphEventHandlers;
}

export interface UseFlowGraphControllerReturn {
  reactFlowData: ReactFlowData | null;
  loading: boolean;
  error: string | null;
  refreshLayout: () => Promise<void>;
  fitOnce: () => void;
  // Handlers
  onNodeClick: (event: any, node: any) => void;
  onEdgeClick: (event: any, edge: any) => void;
  onNodeDrag: (event: any, node: any) => void;
  onNodeDragStop: (event: any, node: any) => void;
  onNodesChange: (changes: any[]) => void;
}

export function useFlowGraphController({
  visualizationState,
  config,
  layoutConfig,
  eventHandlers,
}: UseFlowGraphControllerArgs): UseFlowGraphControllerReturn {
  const [reactFlowData, setReactFlowData] = useState<ReactFlowData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const { fitView } = useReactFlow();

  // Manual position management
  const { manualPositions, applyManualPositions } = useManualPositions();

  // Track the base layout data (before manual positioning)
  const baseReactFlowDataRef = useRef<ReactFlowData | null>(null);

  // Track the last fit operation to prevent excessive fits
  const lastFitTimeRef = useRef<number>(0);
  const autoFitTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Create bridge and engine (stable instances)
  const bridge = useMemo(() => new ReactFlowBridge(), []);
  const engine = useMemo(() => createVisualizationEngine(visualizationState, {
    autoLayout: true,
    enableLogging: false,
    layoutConfig: layoutConfig,
  }), [visualizationState, layoutConfig]);

  // Cleanup timeout on unmount
  useEffect(() => () => {
    if (autoFitTimeoutRef.current) clearTimeout(autoFitTimeoutRef.current);
  }, []);

  const fitOnce = useCallback(() => {
    try {
      fitView({ padding: 0.1, maxZoom: 1.2, duration: 300 });
      lastFitTimeRef.current = Date.now();
    } catch (err) {
      console.warn('[FlowGraph] ⚠️ fitOnce failed:', err);
    }
  }, [fitView]);

  const refreshLayout = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);

      await engine.runLayout();
      const baseData = bridge.visStateToReactFlow(visualizationState);
      baseReactFlowDataRef.current = baseData;
      const dataWithManual = applyManualPositions(baseData, manualPositions);
      setReactFlowData(dataWithManual);

      if (config.fitView !== false) {
        setTimeout(() => {
          try {
            fitView({ padding: 0.1, maxZoom: 1.2, duration: 300 });
            lastFitTimeRef.current = Date.now();
            // console.log('[FlowGraph] 🎯 Auto-fit applied after refresh');
          } catch (err) {
            console.warn('[FlowGraph] ⚠️ Auto-fit failed during refresh:', err);
          }
        }, 200);
      }
    } catch (err) {
      console.error('[FlowGraph] ❌ Failed to refresh layout:', err);
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [applyManualPositions, bridge, config.fitView, engine, fitView, manualPositions, visualizationState]);

  // Listen to layout config changes (when data already present)
  useEffect(() => {
    if (!layoutConfig || !reactFlowData) return;
    const run = async () => {
      try {
        setLoading(true);
        setError(null);
        engine.updateLayoutConfig(layoutConfig, false);
        await engine.runLayout();
        const baseData = bridge.visStateToReactFlow(visualizationState);
        baseReactFlowDataRef.current = baseData;
        const withManual = applyManualPositions(baseData, visualizationState.getAllManualPositions());
        setReactFlowData(withManual);
      } catch (err) {
        console.error('[FlowGraph] ❌ Failed to apply layout change:', err);
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setLoading(false);
      }
    };
    run();
  }, [layoutConfig]);

  // Listen to config changes (palette, edge styles)
  useEffect(() => {
    // Palette
    if (config?.colorPalette) {
      bridge.setColorPalette(config.colorPalette);
      setReactFlowData(prev => {
        if (!prev) return prev;
        return {
          ...prev,
          nodes: prev.nodes.map(n => ({
            ...n,
            data: { ...n.data, colorPalette: config.colorPalette },
          })),
        };
      });
    }
    // Edge style config
    if (config?.edgeStyleConfig) {
      bridge.setEdgeStyleConfig(config.edgeStyleConfig);
    }
  }, [config?.colorPalette, config?.edgeStyleConfig, bridge]);

  // Listen to visualization state changes
  useEffect(() => {
    if (engine.getState().isRunningSmartCollapse) return;

    const handle = async () => {
      try {
        const state = engine.getState();
        if (state.phase === 'laying_out' || state.isRunningSmartCollapse) {
          // Skip; engine busy
          return;
        }

        setLoading(true);
        setError(null);
        await engine.runLayout();
        const baseData = bridge.visStateToReactFlow(visualizationState);
        baseReactFlowDataRef.current = baseData;
        const withManual = applyManualPositions(baseData, manualPositions);
        setReactFlowData(withManual);

        if (config.fitView !== false) {
          const now = Date.now();
          const since = now - lastFitTimeRef.current;
          if (autoFitTimeoutRef.current) clearTimeout(autoFitTimeoutRef.current);
          const delay = since > 500 ? 100 : 300;
          autoFitTimeoutRef.current = setTimeout(() => {
            try {
              fitView({ padding: 0.1, maxZoom: 1.2, duration: 300 });
              lastFitTimeRef.current = Date.now();
            } catch (err) {
              console.warn('[FlowGraph] ⚠️ Auto-fit failed:', err);
            }
          }, delay);
        }
      } catch (err) {
        console.error('[FlowGraph] ❌ Failed to update visualization:', err);
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setLoading(false);
      }
    };

    handle();
    // Alpha: only initial render; real change detection would add listeners
  }, [visualizationState, engine, bridge, applyManualPositions]);

  // Update ReactFlow positions when manual positions change (no re-layout)
  useEffect(() => {
    if (baseReactFlowDataRef.current && visualizationState.hasAnyManualPositions()) {
      const updated = applyManualPositions(baseReactFlowDataRef.current, visualizationState.getAllManualPositions());
      setReactFlowData(updated);
    }
  }, [visualizationState, applyManualPositions]);

  // Event handlers
  const onNodeClick = useCallback((event: any, node: any) => {
    eventHandlers?.onNodeClick?.(event, node);
  }, [eventHandlers]);

  const onEdgeClick = useCallback((event: any, edge: any) => {
    eventHandlers?.onEdgeClick?.(event, edge);
  }, [eventHandlers]);

  const onNodeDrag = useCallback((event: any, node: any) => {
    eventHandlers?.onNodeDrag?.(event, node);
  }, [eventHandlers]);

  const onNodeDragStop = useCallback((event: any, node: any) => {
    visualizationState.setManualPosition(node.id, node.position.x, node.position.y);

    if (config.fitView !== false) {
      const now = Date.now();
      const since = now - lastFitTimeRef.current;
      if (autoFitTimeoutRef.current) clearTimeout(autoFitTimeoutRef.current);
      if (since > 500) {
        autoFitTimeoutRef.current = setTimeout(() => {
          try {
            fitView({ padding: 0.1, maxZoom: 1.2, duration: 300 });
            lastFitTimeRef.current = Date.now();
          } catch (err) {
            console.warn('[FlowGraph] ⚠️ Auto-fit after drag failed:', err);
          }
        }, 200);
      }
    }
  }, [visualizationState, config.fitView, fitView]);

  const onNodesChange = useCallback((changes: any[]) => {
    setReactFlowData(prev => {
      if (!prev) return prev;
      return { ...prev, nodes: applyNodeChanges(changes, prev.nodes) };
    });
  }, []);

  return {
    reactFlowData,
    loading,
    error,
    refreshLayout,
    fitOnce,
    onNodeClick,
    onEdgeClick,
    onNodeDrag,
    onNodeDragStop,
    onNodesChange,
  };
}
