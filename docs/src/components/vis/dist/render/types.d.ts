/**
 * @fileoverview Render Types
 *
 * Type definitions for render components
 */
interface BaseNode {
    id: string;
    type?: string;
    data: Record<string, any>;
    position: {
        x: number;
        y: number;
    };
}
interface BaseEdge {
    id: string;
    source: string;
    target: string;
    type?: string;
    data?: Record<string, any>;
}
export interface TypedReactFlowNode extends BaseNode {
    data: {
        label: string;
        style: string;
        [key: string]: any;
    };
    width?: number;
    height?: number;
}
export interface TypedReactFlowEdge extends BaseEdge {
    data?: {
        style?: string;
        [key: string]: any;
    };
}
export interface TypedReactFlowData {
    nodes: TypedReactFlowNode[];
    edges: TypedReactFlowEdge[];
}
export {};
//# sourceMappingURL=types.d.ts.map