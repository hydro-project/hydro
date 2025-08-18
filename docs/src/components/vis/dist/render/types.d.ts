/**
 * @fileoverview ReactFlow Integration Types
 *
 * Strong TypeScript types to enforce correct data flow from ELK layout to ReactFlow rendering.
 * These types ensure that ELK-calculated dimensions are properly passed through the pipeline.
 */
import type { Node, Edge, Connection } from '@xyflow/react';
/**
 * ELK-calculated position for any layout element
 */
export interface ELKPosition {
    x: number;
    y: number;
}
/**
 * ELK-calculated dimensions for any layout element
 */
export interface ELKDimensions {
    width: number;
    height: number;
}
/**
 * Combined ELK layout result for positioned elements
 */
export interface ELKLayoutResult extends ELKPosition, ELKDimensions {
}
/**
 * Standard node with ELK layout applied
 */
export interface ELKPositionedNode extends ELKLayoutResult {
    id: string;
    label: string;
    style: string;
}
/**
 * Container with ELK layout applied - MUST include computed dimensions
 */
export interface ELKPositionedContainer extends ELKLayoutResult {
    id: string;
    collapsed: boolean;
    children?: Set<string>;
}
/**
 * Edge with optional ELK-calculated routing points
 */
export interface ELKPositionedEdge {
    id: string;
    source: string;
    target: string;
    style: string;
    points?: ELKPosition[];
}
/**
 * Complete layout result from ELK with strongly typed dimensions
 */
export interface StrongLayoutResult {
    nodes: ELKPositionedNode[];
    containers: ELKPositionedContainer[];
    edges: ELKPositionedEdge[];
    hyperEdges: ELKPositionedEdge[];
}
/**
 * Base data that must be passed to all ReactFlow nodes
 */
export interface BaseNodeData extends Record<string, unknown> {
    label: string;
    style: string;
}
/**
 * Standard node data for ReactFlow
 */
export interface StandardNodeData extends BaseNodeData {
    nodeType?: string;
}
/**
 * Container node data - MUST include ELK-calculated dimensions
 */
export interface ContainerNodeData extends BaseNodeData {
    collapsed: boolean;
    width: number;
    height: number;
}
/**
 * Union type for all possible node data
 */
export type ReactFlowNodeData = StandardNodeData | ContainerNodeData;
/**
 * Standard ReactFlow node with proper typing
 */
export interface TypedStandardNode extends Node {
    type: 'standard';
    data: StandardNodeData;
}
/**
 * Container ReactFlow node with enforced dimension data
 */
export interface TypedContainerNode extends Node {
    type: 'container';
    data: ContainerNodeData;
    style: {
        width: number;
        height: number;
    };
}
/**
 * Union type for all ReactFlow nodes with proper typing
 */
export type TypedReactFlowNode = TypedStandardNode | TypedContainerNode;
/**
 * ReactFlow edge with proper typing
 */
export interface TypedReactFlowEdge extends Omit<Edge, 'data'> {
    data: {
        style: string;
        edge?: {
            style: string;
        };
        onEdgeClick?: (id: string) => void;
        onEdgeContextMenu?: (id: string, event: React.MouseEvent) => void;
        isHighlighted?: boolean;
        hyperEdge?: {
            aggregatedEdges: any[];
        };
    };
}
/**
 * Complete ReactFlow data with strong typing
 */
export interface TypedReactFlowData {
    nodes: TypedReactFlowNode[];
    edges: TypedReactFlowEdge[];
}
export interface RenderConfig {
    enableMiniMap?: boolean;
    enableControls?: boolean;
    fitView?: boolean;
    nodesDraggable?: boolean;
    snapToGrid?: boolean;
    gridSize?: number;
    nodesConnectable?: boolean;
    elementsSelectable?: boolean;
    enableZoom?: boolean;
    enablePan?: boolean;
    enableSelection?: boolean;
}
/**
 * Standard node props for ReactFlow v12
 */
export interface StandardNodeProps {
    id: string;
    data: StandardNodeData;
    width?: number;
    height?: number;
    selected?: boolean;
}
/**
 * Props for container node component with enforced dimensions
 */
export interface ContainerNodeProps {
    id: string;
    data: ContainerNodeData;
    width?: number;
    height?: number;
    selected?: boolean;
}
/**
 * Typed edge props for ReactFlow v12
 */
export interface TypedEdgeProps {
    id: string;
    sourceX: number;
    sourceY: number;
    targetX: number;
    targetY: number;
    sourcePosition: any;
    targetPosition: any;
    style?: React.CSSProperties;
    data?: {
        style?: string;
        edge?: {
            style: string;
        };
        onEdgeClick?: (id: string) => void;
        onEdgeContextMenu?: (id: string, event: React.MouseEvent) => void;
        isHighlighted?: boolean;
        hyperEdge?: {
            aggregatedEdges: any[];
        };
    };
    selected?: boolean;
}
/**
 * Type guard to check if node data is container data
 */
export declare function isContainerNodeData(data: ReactFlowNodeData): data is ContainerNodeData;
/**
 * Type guard to check if node is a container node
 */
export declare function isContainerNode(node: TypedReactFlowNode): node is TypedContainerNode;
/**
 * Type guard to check if ELK container has required dimensions
 */
export declare function isValidELKContainer(container: any): container is ELKPositionedContainer;
/**
 * Validates that ELK layout result has all required properties
 */
export declare function validateELKLayoutResult(result: any): result is StrongLayoutResult;
/**
 * Validates that ReactFlow data has proper container dimensions
 */
export declare function validateReactFlowData(data: any): data is TypedReactFlowData;
export interface GraphFlowEventHandlers {
    onNodeClick?: (event: React.MouseEvent, node: Node) => void;
    onNodeDoubleClick?: (event: React.MouseEvent, node: Node) => void;
    onNodeContextMenu?: (event: React.MouseEvent, node: Node) => void;
    onNodeDrag?: (event: React.MouseEvent, node: Node) => void;
    onNodeDragStop?: (event: React.MouseEvent, node: Node) => void;
    onEdgeClick?: (event: React.MouseEvent, edge: Edge) => void;
    onEdgeContextMenu?: (event: React.MouseEvent, edge: Edge) => void;
    onConnect?: (params: Connection) => void;
    onSelectionChange?: (selection: {
        nodes: Node[];
        edges: Edge[];
    }) => void;
    onPaneClick?: (event: React.MouseEvent) => void;
    onPaneContextMenu?: (event: React.MouseEvent) => void;
}
//# sourceMappingURL=types.d.ts.map