/**
 * @fileoverview Render Types
 * 
 * Type definitions for render components
 */

// Basic node/edge structure for render components
interface BaseNode {
  id: string;
  type?: string;
  data: Record<string, unknown>;
  position: { x: number; y: number };
}

interface BaseEdge {
  id: string;
  source: string;
  target: string;
  type?: string;
  data?: Record<string, unknown>;
}

// Typed ReactFlow data structures
export interface TypedReactFlowNode extends BaseNode {
  data: {
    label: string;
    style: string;
  [key: string]: unknown;
  };
  width?: number;
  height?: number;
}

export interface TypedReactFlowEdge extends BaseEdge {
  data?: {
    style?: string;
  [key: string]: unknown;
  };
}

export interface TypedReactFlowData {
  nodes: TypedReactFlowNode[];
  edges: TypedReactFlowEdge[];
}
