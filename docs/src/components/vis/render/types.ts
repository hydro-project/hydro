/**
 * @fileoverview ReactFlow Integration Types
 * 
 * Strong TypeScript types to enforce correct data flow from ELK layout to ReactFlow rendering.
 * These types ensure that ELK-calculated dimensions are properly passed through the pipeline.
 */

import type { Node, Edge, Connection } from '@xyflow/react';

// ============ ELK Layout Result Types ============

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
export interface ELKLayoutResult extends ELKPosition, ELKDimensions {}

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

// ============ ReactFlow Node Data Types ============

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
  // CRITICAL: ELK dimensions must be passed through data
  width: number;
  height: number;
  // Container interaction callbacks
  onContainerCollapse?: (containerId: string) => void;
  onContainerExpand?: (containerId: string) => void;
}

/**
 * Union type for all possible node data
 */
export type ReactFlowNodeData = StandardNodeData | ContainerNodeData;

// ============ ReactFlow Node Types ============

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
  // ReactFlow style should match data dimensions
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

// ============ Render Configuration ============

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

// ============ Component Props Types ============

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
  markerEnd?: string;
  markerStart?: string;
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

// ============ Type Guards ============

/**
 * Type guard to check if node data is container data
 */
export function isContainerNodeData(data: ReactFlowNodeData): data is ContainerNodeData {
  return 'width' in data && 'height' in data && 'collapsed' in data;
}

/**
 * Type guard to check if node is a container node
 */
export function isContainerNode(node: TypedReactFlowNode): node is TypedContainerNode {
  return node.type === 'container';
}

/**
 * Type guard to check if ELK container has required dimensions
 */
export function isValidELKContainer(container: any): container is ELKPositionedContainer {
  return (
    typeof container.id === 'string' &&
    typeof container.x === 'number' &&
    typeof container.y === 'number' &&
    typeof container.width === 'number' &&
    typeof container.height === 'number' &&
    typeof container.collapsed === 'boolean'
  );
}

// ============ Validation Functions ============

/**
 * Validates that ELK layout result has all required properties
 */
export function validateELKLayoutResult(result: any): result is StrongLayoutResult {
  if (!result || typeof result !== 'object') return false;
  
  // Check nodes
  if (!Array.isArray(result.nodes)) return false;
  if (!result.nodes.every((node: any) => 
    typeof node.id === 'string' &&
    typeof node.x === 'number' &&
    typeof node.y === 'number' &&
    typeof node.width === 'number' &&
    typeof node.height === 'number'
  )) return false;
  
  // Check containers
  if (!Array.isArray(result.containers)) return false;
  if (!result.containers.every(isValidELKContainer)) return false;
  
  return true;
}

/**
 * Validates that ReactFlow data has proper container dimensions
 */
export function validateReactFlowData(data: any): data is TypedReactFlowData {
  if (!data || typeof data !== 'object') return false;
  if (!Array.isArray(data.nodes) || !Array.isArray(data.edges)) return false;
  
  // Check that all container nodes have proper dimensions
  return data.nodes.every((node: any) => {
    if (node.type === 'container') {
      return (
        node.data &&
        typeof node.data.width === 'number' &&
        typeof node.data.height === 'number' &&
        node.style &&
        typeof node.style.width === 'number' &&
        typeof node.style.height === 'number' &&
        node.data.width === node.style.width &&
        node.data.height === node.style.height
      );
    }
    return true;
  });
}

export interface GraphFlowEventHandlers {
  onNodeClick?: (event: React.MouseEvent, node: Node) => void;
  onNodeDoubleClick?: (event: React.MouseEvent, node: Node) => void;
  onNodeContextMenu?: (event: React.MouseEvent, node: Node) => void;
  onNodeDrag?: (event: React.MouseEvent, node: Node) => void;
  onNodeDragStop?: (event: React.MouseEvent, node: Node) => void;
  onEdgeClick?: (event: React.MouseEvent, edge: Edge) => void;
  onEdgeContextMenu?: (event: React.MouseEvent, edge: Edge) => void;
  onConnect?: (params: Connection) => void;
  onSelectionChange?: (selection: { nodes: Node[]; edges: Edge[] }) => void;
  onPaneClick?: (event: React.MouseEvent) => void;
  onPaneContextMenu?: (event: React.MouseEvent) => void;
}
