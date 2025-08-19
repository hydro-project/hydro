/**
 * @fileoverview Manual Position Management Hook
 * 
 * Manages manual node positions for ReactFlow nodes when users drag them.
 * Preserves user positioning across layout updates and state changes.
 */

import { useState, useCallback } from 'react';
import type { ReactFlowData } from '../bridges/ReactFlowBridge';

export interface ManualPosition {
  x: number;
  y: number;
}

export interface UseManualPositionsReturn {
  /** Current manual positions map */
  manualPositions: Map<string, ManualPosition>;
  
  /** Update manual positions map */
  setManualPositions: React.Dispatch<React.SetStateAction<Map<string, ManualPosition>>>;
  
  /** Apply manual positions to ReactFlow data */
  applyManualPositions: (baseData: ReactFlowData, manualPosMap: Map<string, ManualPosition>) => ReactFlowData;
}

/**
 * Hook to manage manual node positions in ReactFlow
 * 
 * @returns Object with manual positions state and apply function
 */
export function useManualPositions(): UseManualPositionsReturn {
  // Store manual drag positions to preserve user positioning
  const [manualPositions, setManualPositions] = useState<Map<string, ManualPosition>>(new Map());

  // Function to apply manual positions to existing ReactFlow data
  const applyManualPositions = useCallback((baseData: ReactFlowData, manualPosMap: Map<string, ManualPosition>) => {
    if (manualPosMap.size === 0) return baseData;
    
    return {
      ...baseData,
      nodes: baseData.nodes.map(node => {
        const manualPos = manualPosMap.get(node.id);
        if (manualPos) {
          return {
            ...node,
            position: { x: manualPos.x, y: manualPos.y }
          };
        }
        return node;
      })
    };
  }, []);

  return {
    manualPositions,
    setManualPositions,
    applyManualPositions
  };
}