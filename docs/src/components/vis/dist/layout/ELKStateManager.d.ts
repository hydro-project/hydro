/**
 * ELK State Manager (TypeScript port from working visualizer)
 *
 * This module provides wrapper functions that ensure all ELK layout interactions
 * are consistent with visualization state management as the single source of truth.
 *
 * Key principle: ELK should only ever calculate layouts based on the exact
 * visual state requirements, and return results that perfectly match those requirements.
 */
import { GraphNode, GraphEdge, Container, HyperEdge } from '../shared/types';
import { ELKAlgorithm } from '../shared/config';
export interface LayoutPosition {
    x: number;
    y: number;
}
export interface LayoutDimensions {
    width: number;
    height: number;
}
export interface ContainmentValidationResult {
    isValid: boolean;
    violations: ContainmentViolation[];
}
export interface ContainmentViolation {
    childId: string;
    containerId: string;
    issue: string;
    childBounds: LayoutBounds;
    containerBounds: LayoutBounds;
}
interface LayoutBounds {
    x: number;
    y: number;
    width: number;
    height: number;
    right: number;
    bottom: number;
}
export interface ELKStateManager {
    calculateFullLayout(nodes: GraphNode[], edges: GraphEdge[], containers: Container[], layoutType?: ELKAlgorithm): Promise<{
        nodes: any[];
        edges: GraphEdge[];
    }>;
    calculateVisualLayout(nodes: GraphNode[], edges: GraphEdge[], containers: Container[], hyperEdges: HyperEdge[], layoutType?: ELKAlgorithm, dimensionsCache?: Map<string, LayoutDimensions>): Promise<{
        nodes: any[];
        edges: GraphEdge[];
        elkResult: any;
    }>;
}
/**
 * Create an ELK state manager that wraps all ELK layout interactions
 * with proper state management as the single source of truth.
 */
export declare function createELKStateManager(): ELKStateManager;
export {};
//# sourceMappingURL=ELKStateManager.d.ts.map