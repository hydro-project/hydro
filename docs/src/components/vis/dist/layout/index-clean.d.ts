/**
 * @fileoverview Layout module exports - Bridge Architecture Implementation
 *
 * Clean layout module using our bridge architecture.
 * No dependencies on alpha.
 */
export { ELKLayoutEngine, DEFAULT_LAYOUT_CONFIG, createELKStateManager } from './ELKLayoutEngine';
export type { LayoutConfig, LayoutResult, LayoutEngine, LayoutStatistics, LayoutEventData, LayoutEventCallback, PositionedNode, PositionedEdge, PositionedContainer, PositionedHyperEdge, Position } from '../core/types';
import type { LayoutConfig } from '../core/types';
export declare function getLayoutConfig(name: string): LayoutConfig;
export declare function createLayoutConfig(overrides: Partial<LayoutConfig>): LayoutConfig;
export declare const LAYOUT_CONFIGS: {
    default: LayoutConfig;
    compact: {
        spacing: number;
        algorithm?: "mrtree" | "layered" | "force" | "stress" | "radial";
        direction?: "UP" | "DOWN" | "LEFT" | "RIGHT";
        nodeSize?: {
            width: number;
            height: number;
        };
    };
    spacious: {
        spacing: number;
        algorithm?: "mrtree" | "layered" | "force" | "stress" | "radial";
        direction?: "UP" | "DOWN" | "LEFT" | "RIGHT";
        nodeSize?: {
            width: number;
            height: number;
        };
    };
};
export interface ELKStateManager {
    updatePositions(): void;
    dispose(): void;
}
//# sourceMappingURL=index-clean.d.ts.map