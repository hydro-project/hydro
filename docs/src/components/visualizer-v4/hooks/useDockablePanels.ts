/**
 * @fileoverview Hook for managing dockable panel state
 * 
 * Provides centralized state management for multiple dockable panels
 * with persistence and coordination between panels.
 */

import { useState, useCallback, useEffect } from 'react';

export interface PanelState {
  open: boolean;
  pinned: boolean;
  collapsed: boolean;
  placement: 'top' | 'right' | 'bottom' | 'left';
  width?: number | string;
  height?: number | string;
}

export interface PanelConfig {
  id: string;
  title: string;
  defaultState?: Partial<PanelState>;
}

const DEFAULT_PANEL_STATE: PanelState = {
  open: true,
  pinned: true,
  collapsed: false,
  placement: 'right',
  width: 320
};

export function useDockablePanels(configs: PanelConfig[]) {
  const [panelStates, setPanelStates] = useState<Record<string, PanelState>>(() => {
    const initialStates: Record<string, PanelState> = {};
    
    configs.forEach(config => {
      // Try to load from localStorage
      const saved = localStorage.getItem(`panel-${config.id}`);
      let state = DEFAULT_PANEL_STATE;
      
      if (saved) {
        try {
          state = { ...DEFAULT_PANEL_STATE, ...JSON.parse(saved) };
        } catch (e) {
          console.warn(`Failed to parse saved state for panel ${config.id}`);
        }
      }
      
      // Override with config defaults
      if (config.defaultState) {
        state = { ...state, ...config.defaultState };
      }
      
      initialStates[config.id] = state;
    });
    
    return initialStates;
  });

  // Save to localStorage when state changes
  useEffect(() => {
    Object.entries(panelStates).forEach(([id, state]) => {
      localStorage.setItem(`panel-${id}`, JSON.stringify(state));
    });
  }, [panelStates]);

  const updatePanelState = useCallback((panelId: string, updates: Partial<PanelState>) => {
    setPanelStates(prev => ({
      ...prev,
      [panelId]: {
        ...prev[panelId],
        ...updates
      }
    }));
  }, []);

  const togglePanel = useCallback((panelId: string) => {
    updatePanelState(panelId, { 
      open: !panelStates[panelId]?.open 
    });
  }, [panelStates, updatePanelState]);

  const togglePin = useCallback((panelId: string) => {
    updatePanelState(panelId, { 
      pinned: !panelStates[panelId]?.pinned 
    });
  }, [panelStates, updatePanelState]);

  const toggleCollapse = useCallback((panelId: string) => {
    updatePanelState(panelId, { 
      collapsed: !panelStates[panelId]?.collapsed 
    });
  }, [panelStates, updatePanelState]);

  const closeAllPanels = useCallback(() => {
    setPanelStates(prev => {
      const newStates = { ...prev };
      Object.keys(newStates).forEach(id => {
        newStates[id] = { ...newStates[id], open: false };
      });
      return newStates;
    });
  }, []);

  const resetPanels = useCallback(() => {
    setPanelStates(prev => {
      const newStates = { ...prev };
      Object.keys(newStates).forEach(id => {
        newStates[id] = { ...DEFAULT_PANEL_STATE };
        localStorage.removeItem(`panel-${id}`);
      });
      return newStates;
    });
  }, []);

  return {
    panelStates,
    updatePanelState,
    togglePanel,
    togglePin,
    toggleCollapse,
    closeAllPanels,
    resetPanels
  };
}
