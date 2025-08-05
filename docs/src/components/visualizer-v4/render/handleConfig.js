/**
 * @fileoverview Handle Configuration for ReactFlow Nodes
 *
 * Centralized configuration for ReactFlow v12 continuous handles to maximize layout flexibility.
 * This encapsulates handle behavior so it can be easily changed across the entire system.
 */
import { Position } from '@xyflow/react';
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
        sourceHandles: [], // No discrete handles needed
        targetHandles: [], // ReactFlow handles connections automatically
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
        ],
        targetHandles: [
            { id: 'in-top', position: Position.Top },
            { id: 'in-right', position: Position.Right },
            { id: 'in-bottom', position: Position.Bottom },
            { id: 'in-left', position: Position.Left },
        ],
    },
    /**
     * No handles - let ReactFlow auto-connect
     * Simplest approach but least control
     */
    none: {
        enableContinuousHandles: false,
        sourceHandles: [],
        targetHandles: [],
    }
};
/**
 * Current handle strategy - easily changeable
 */
export const CURRENT_HANDLE_STRATEGY = 'continuous';
/**
 * Get the current handle configuration
 */
export function getHandleConfig() {
    return HANDLE_STRATEGIES[CURRENT_HANDLE_STRATEGY];
}
/**
 * Default handle style for continuous handles
 */
export const CONTINUOUS_HANDLE_STYLE = {
    background: 'transparent',
    border: 'none',
    width: '100%',
    height: '100%',
};
//# sourceMappingURL=handleConfig.js.map