/**
 * @fileoverview Handle Configuration for ReactFlow Nodes
 * 
 * Centralized configuration for ReactFlow v12 continuous handles to maximize layout flexibility.
 * This encapsulates handle behavior so it can be easily changed across the entire system.
 */

import { Position } from '@xyflow/react';

export interface HandleConfig {
  id: string;
  position: Position;
  style?: React.CSSProperties;
}

/**
 * Handle strategy types
 */
export type HandleStrategy = 'continuous' | 'discrete' | 'none';

/**
 * Configuration for different handle strategies
 */
export const HANDLE_STRATEGIES = {
  /**
   * Continuous handles (ReactFlow v12) - connections can be made anywhere on node perimeter
   * Provides maximum layout flexibility
   */
  continuous: {
    enableContinuousHandles: true,
    sourceHandles: [] as HandleConfig[], // No discrete handles needed
    targetHandles: [] as HandleConfig[], // ReactFlow handles connections automatically
  },
  
  /**
   * Discrete handles - specific connection points
   * More controlled but less flexible
   */
  discrete: {
    enableContinuousHandles: false,
    sourceHandles: [
      { id: 'out-top', position: Position.Top },
      { id: 'out-right', position: Position.Right },
      { id: 'out-bottom', position: Position.Bottom },
      { id: 'out-left', position: Position.Left },
    ] as HandleConfig[],
    targetHandles: [
      { id: 'in-top', position: Position.Top },
      { id: 'in-right', position: Position.Right },
      { id: 'in-bottom', position: Position.Bottom },
      { id: 'in-left', position: Position.Left },
    ] as HandleConfig[],
  },
  
  /**
   * No handles - let ReactFlow auto-connect
   * Simplest approach but least control
   */
  none: {
    enableContinuousHandles: false,
    sourceHandles: [] as HandleConfig[],
    targetHandles: [] as HandleConfig[],
  }
} as const;

/**
 * Current handle strategy - easily changeable
 */
export const CURRENT_HANDLE_STRATEGY: HandleStrategy = 'continuous';

/**
 * Get the current handle configuration
 */
export function getHandleConfig() {
  return HANDLE_STRATEGIES[CURRENT_HANDLE_STRATEGY];
}

/**
 * Default handle style for continuous handles
 */
export const CONTINUOUS_HANDLE_STYLE: React.CSSProperties = {
  background: 'transparent',
  border: 'none',
  width: '100%',
  height: '100%',
};
