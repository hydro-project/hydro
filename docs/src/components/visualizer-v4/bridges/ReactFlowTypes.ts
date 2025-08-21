// Shared React Flow types used by ReactFlowBridge and its helpers
import type { Edge as ReactFlowEdge } from '@xyflow/react';

export interface ReactFlowNode {
  id: string;
  type: 'standard' | 'container';
  position: { x: number; y: number };
  data: {
    label: string;
    style: string;
    collapsed?: boolean;
    width?: number;
    height?: number;
    [key: string]: unknown;
  };
  style?: {
    width?: number;
    height?: number;
  };
  parentId?: string;
  connectable?: boolean;
  extent?: 'parent' | [[number, number], [number, number]];
}

export interface ReactFlowData {
  nodes: ReactFlowNode[];
  edges: ReactFlowEdge[];
}
