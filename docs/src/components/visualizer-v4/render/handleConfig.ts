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
export type HandleStrategy = 'continuous' | 'discrete' | 'floating' | 'none';

/**
 * Handle styles for different strategies
 */
export const HANDLE_STYLES = {
  continuous: {
    background: 'transparent',
    border: 'none',
    width: '100%',
    height: '100%',
  },
  discrete: {
    background: '#555',
    border: '2px solid #222',
    width: '8px',
    height: '8px',
  },
  floating: {
    background: 'transparent',
    border: 'none',
    width: '8px',
    height: '8px',
    opacity: 0, // Invisible handles for floating edges
  }
} as const;

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
   * Floating handles - whole node connectivity with smart edge attachment
   * Uses custom floating edge component for continuous-handle-like UX
   * Includes discrete handles for React Flow v12 compatibility but FloatingEdge ignores positions
   */
  floating: {
    enableContinuousHandles: false,
    sourceHandles: [
      { id: 'out-top', position: Position.Top, style: { opacity: 0, width: '8px', height: '8px' } },
      { id: 'out-right', position: Position.Right, style: { opacity: 0, width: '8px', height: '8px' } },
      { id: 'out-bottom', position: Position.Bottom, style: { opacity: 0, width: '8px', height: '8px' } },
      { id: 'out-left', position: Position.Left, style: { opacity: 0, width: '8px', height: '8px' } },
    ] as HandleConfig[],
    targetHandles: [
      { id: 'in-top', position: Position.Top, style: { opacity: 0, width: '8px', height: '8px' } },
      { id: 'in-right', position: Position.Right, style: { opacity: 0, width: '8px', height: '8px' } },
      { id: 'in-bottom', position: Position.Bottom, style: { opacity: 0, width: '8px', height: '8px' } },
      { id: 'in-left', position: Position.Left, style: { opacity: 0, width: '8px', height: '8px' } },
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
 * FLOATING: Using floating edges for continuous-handle-like UX
 */
export const CURRENT_HANDLE_STRATEGY: HandleStrategy = 'floating';

/**
 * Get the current handle configuration
 */
export function getHandleConfig() {
  return HANDLE_STRATEGIES[CURRENT_HANDLE_STRATEGY];
}

/**
 * Default handle style for continuous handles
 * @deprecated Use HANDLE_STYLES.continuous instead
 */
export const CONTINUOUS_HANDLE_STYLE: React.CSSProperties = HANDLE_STYLES.continuous;
