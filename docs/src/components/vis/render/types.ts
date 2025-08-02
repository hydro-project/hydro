/**
 * @fileoverview Minimal ReactFlow types
 */

import type { Node, Edge, Connection } from 'reactflow';

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

export interface GraphFlowEventHandlers {
  onNodeClick?: (event: React.MouseEvent, node: Node) => void;
  onNodeDoubleClick?: (event: React.MouseEvent, node: Node) => void;
  onNodeContextMenu?: (event: React.MouseEvent, node: Node) => void;
  onNodeDrag?: (event: React.MouseEvent, node: Node) => void;
  onNodeDragStop?: (event: React.MouseEvent, node: Node) => void;
  onEdgeClick?: (event: React.MouseEvent, edge: Edge) => void;
  onEdgeContextMenu?: (event: React.MouseEvent, edge: Edge) => void;
  onEdgeUpdate?: (oldEdge: Edge, newConnection: Connection) => void;
  onConnect?: (params: Connection) => void;
  onSelectionChange?: (selection: { nodes: Node[]; edges: Edge[] }) => void;
  onPaneClick?: (event: React.MouseEvent) => void;
  onPaneContextMenu?: (event: React.MouseEvent) => void;
}
