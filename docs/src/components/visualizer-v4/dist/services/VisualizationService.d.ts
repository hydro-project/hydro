/**
 * @fileoverview Visualization Service - Encapsulation Layer
 *
 * This service enforces that ALL layout and rend    const layoutResult = await this.layoutEngine.layoutWithChangedContainer(
      visibleNodes,
      visibleEdges,
      visibleContainers, // Send all visible containers for hyperedge positioning
      hyperEdges,
      layoutConfig,
      null, // null = full layout
      visState // Pass VisState for centralized state management
    );rations go through VisState.
 * It provides statically-typed methods that prevent bypassing the single source of truth.
 *
 * CRITICAL: No external code should call ELK or ReactFlow directly - everything must go through this service.
 */
import type { VisualizationState } from '../shared/types';
import { TypedReactFlowData, TypedReactFlowNode, TypedReactFlowEdge } from '../render/types';
import { LayoutConfig } from '../layout/types';
/**
 * Encapsulated layout request - can only be created by VisualizationService
 */
export declare class EncapsulatedLayoutRequest {
    private readonly token;
    private readonly visState;
    private readonly layoutConfig;
    private readonly changedContainerId?;
    private constructor();
    /** Package-private access for the service */
    getVisState(): VisualizationState;
    getLayoutConfig(): LayoutConfig;
    getChangedContainerId(): string | null | undefined;
    /** Factory method - only callable by VisualizationService */
    static create(visState: VisualizationState, layoutConfig: LayoutConfig, changedContainerId?: string | null): EncapsulatedLayoutRequest;
}
/**
 * Encapsulated ReactFlow data - can only be created by VisualizationService
 */
export declare class EncapsulatedReactFlowData {
    private readonly token;
    private readonly data;
    private readonly sourceVisState;
    private constructor();
    /** Safe access to ReactFlow data - guaranteed to come from VisState */
    getReactFlowData(): TypedReactFlowData;
    getSourceVisState(): VisualizationState;
    /** Factory method - only callable by VisualizationService */
    static create(data: TypedReactFlowData, sourceVisState: VisualizationState): EncapsulatedReactFlowData;
}
/**
 * Visualization Service - The ONLY way to interact with layout and rendering
 *
 * This service enforces the architectural principle:
 * VisState → Layout Engine → VisState → ReactFlow Converter → ReactFlow
 */
export declare class VisualizationService {
    private layoutEngine;
    constructor();
    /**
     * Perform layout operation using ONLY VisState data
     *
     * @param visState - The single source of truth
     * @param layoutConfig - Layout configuration
     * @param changedContainerId - For selective layout (optional)
     * @returns Promise that resolves when layout is complete and applied back to VisState
     */
    performLayout(visState: VisualizationState, layoutConfig?: LayoutConfig): Promise<void>;
    /**
     * Generate ReactFlow data from ONLY VisState
     *
     * @param visState - The single source of truth
     * @returns Encapsulated ReactFlow data that can only come from VisState
     */
    generateReactFlowData(visState: VisualizationState): EncapsulatedReactFlowData;
    /**
     * Create a layout result structure from current VisState
     * This ensures we always use the CURRENT state, not stale data
     */
    private createLayoutResultFromVisState;
    /**
     * Full workflow: Layout + ReactFlow data generation
     * This is the main method components should use
     */
    layoutAndRender(visState: VisualizationState, layoutConfig: LayoutConfig): Promise<{
        nodes: TypedReactFlowNode[];
        edges: TypedReactFlowEdge[];
    }>;
    /**
     * Comprehensive VisState debugging - FOCUSED ON HYPEREDGES
     */
    private logVisStateDetailed;
    private logReactFlowData;
}
/**
 * Get the singleton VisualizationService instance
 * This ensures all components use the same service
 */
export declare function getVisualizationService(): VisualizationService;
/**
 * Type guards for runtime safety
 */
export declare function isEncapsulatedReactFlowData(obj: any): obj is EncapsulatedReactFlowData;
export declare function isEncapsulatedLayoutRequest(obj: any): obj is EncapsulatedLayoutRequest;
//# sourceMappingURL=VisualizationService.d.ts.map