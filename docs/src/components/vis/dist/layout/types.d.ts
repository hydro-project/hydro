/**
 * @fileoverview Layout types with proper TypeScript and centralized constants
 */
import type { GraphNode, GraphEdge, Container, HyperEdge } from '../shared/types';
import type { ELKAlgorithm, ELKDirection } from '../shared/config';
export interface LayoutConfig {
    algorithm?: ELKAlgorithm;
    direction?: ELKDirection;
    spacing?: number;
    nodeSize?: {
        width: number;
        height: number;
    };
}
export interface LayoutPosition {
    x: number;
    y: number;
}
export interface LayoutDimensions {
    width: number;
    height: number;
}
export interface PositionedNode extends GraphNode, LayoutPosition, LayoutDimensions {
}
export interface PositionedEdge extends GraphEdge {
    points?: LayoutPosition[];
}
export interface PositionedContainer extends Container, LayoutPosition, LayoutDimensions {
}
export interface PositionedHyperEdge extends HyperEdge {
    points?: LayoutPosition[];
}
export interface LayoutResult {
    nodes: PositionedNode[];
    edges: PositionedEdge[];
    containers: PositionedContainer[];
    hyperEdges: PositionedHyperEdge[];
}
export interface LayoutEngine {
    layout(nodes: GraphNode[], edges: GraphEdge[], containers: Container[], hyperEdges: HyperEdge[], config?: LayoutConfig): Promise<LayoutResult>;
}
export interface LayoutEngineOptions {
    enableCaching?: boolean;
    enableValidation?: boolean;
    logLevel?: 'none' | 'error' | 'warn' | 'info' | 'debug';
}
export interface LayoutValidationResult {
    isValid: boolean;
    errors: LayoutValidationError[];
    warnings: LayoutValidationWarning[];
}
export interface LayoutValidationError {
    type: 'containment' | 'overlap' | 'bounds';
    message: string;
    nodeId?: string;
    containerId?: string;
    details?: Record<string, any>;
}
export interface LayoutValidationWarning {
    type: 'performance' | 'suboptimal' | 'compatibility';
    message: string;
    suggestion?: string;
    details?: Record<string, any>;
}
export interface LayoutStatistics {
    totalNodes: number;
    totalEdges: number;
    totalContainers: number;
    layoutDuration: number;
    validationResult?: LayoutValidationResult;
    cacheStats?: {
        hits: number;
        misses: number;
        size: number;
    };
}
export interface LayoutEventData {
    type: 'start' | 'progress' | 'complete' | 'error';
    progress?: number;
    statistics?: LayoutStatistics;
    error?: Error;
}
export type LayoutEventCallback = (data: LayoutEventData) => void;
export interface AdvancedLayoutEngine extends LayoutEngine {
    setOptions(options: LayoutEngineOptions): void;
    getOptions(): LayoutEngineOptions;
    clearCache(): void;
    getCacheStatistics(): {
        size: number;
        hits?: number;
        misses?: number;
    };
    validateLayout(result: LayoutResult): LayoutValidationResult;
    on(event: 'layout', callback: LayoutEventCallback): void;
    off(event: 'layout', callback: LayoutEventCallback): void;
    getLastLayoutStatistics(): LayoutStatistics | null;
}
//# sourceMappingURL=types.d.ts.map