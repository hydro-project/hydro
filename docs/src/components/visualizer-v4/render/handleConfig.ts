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
   * Made barely visible for better UX
   */
  discrete: {
    enableContinuousHandles: false,
    sourceHandles: [
      { id: 'out-top', position: Position.Top, style: { opacity: 0.1, width: '6px', height: '6px', background: '#666' } },
      { id: 'out-right', position: Position.Right, style: { opacity: 0.1, width: '6px', height: '6px', background: '#666' } },
      { id: 'out-bottom', position: Position.Bottom, style: { opacity: 0.1, width: '6px', height: '6px', background: '#666' } },
      { id: 'out-left', position: Position.Left, style: { opacity: 0.1, width: '6px', height: '6px', background: '#666' } },
    ] as HandleConfig[],
    targetHandles: [
      { id: 'in-top', position: Position.Top, style: { opacity: 0.1, width: '6px', height: '6px', background: '#666' } },
      { id: 'in-right', position: Position.Right, style: { opacity: 0.1, width: '6px', height: '6px', background: '#666' } },
      { id: 'in-bottom', position: Position.Bottom, style: { opacity: 0.1, width: '6px', height: '6px', background: '#666' } },
      { id: 'in-left', position: Position.Left, style: { opacity: 0.1, width: '6px', height: '6px', background: '#666' } },
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
 * DISCRETE: Using discrete handles with smart routing based on node positions
 * - Connection points are barely visible (opacity 0.1)
 * - Intelligent handle selection: prefers horizontal connections when nodes are side-by-side
 * - Follows the rule: inputs at top/left, outputs at bottom/right
 * - Uses a 1.2x threshold to determine primary direction (horizontal vs vertical)
 */
export const CURRENT_HANDLE_STRATEGY: HandleStrategy = 'discrete';

/**
 * Get the current handle configuration
 */
export function getHandleConfig() {
  return HANDLE_STRATEGIES[CURRENT_HANDLE_STRATEGY];
}


