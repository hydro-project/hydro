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
export declare const HANDLE_STRATEGIES: {
    /**
     * Continuous handles (ReactFlow v12) - connections can be made anywhere on node perimeter
     * Provides maximum layout flexibility
     */
    readonly continuous: {
        readonly enableContinuousHandles: true;
        readonly sourceHandles: HandleConfig[];
        readonly targetHandles: HandleConfig[];
    };
    /**
     * Discrete handles - specific connection points
     * More controlled but less flexible
     */
    readonly discrete: {
        readonly enableContinuousHandles: false;
        readonly sourceHandles: HandleConfig[];
        readonly targetHandles: HandleConfig[];
    };
    /**
     * No handles - let ReactFlow auto-connect
     * Simplest approach but least control
     */
    readonly none: {
        readonly enableContinuousHandles: false;
        readonly sourceHandles: HandleConfig[];
        readonly targetHandles: HandleConfig[];
    };
};
/**
 * Current handle strategy - easily changeable
 */
export declare const CURRENT_HANDLE_STRATEGY: HandleStrategy;
/**
 * Get the current handle configuration
 */
export declare function getHandleConfig(): {
    readonly enableContinuousHandles: true;
    readonly sourceHandles: HandleConfig[];
    readonly targetHandles: HandleConfig[];
} | {
    readonly enableContinuousHandles: false;
    readonly sourceHandles: HandleConfig[];
    readonly targetHandles: HandleConfig[];
} | {
    readonly enableContinuousHandles: false;
    readonly sourceHandles: HandleConfig[];
    readonly targetHandles: HandleConfig[];
};
/**
 * Default handle style for continuous handles
 */
export declare const CONTINUOUS_HANDLE_STYLE: React.CSSProperties;
//# sourceMappingURL=handleConfig.d.ts.map