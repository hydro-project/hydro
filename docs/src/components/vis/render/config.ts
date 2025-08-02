/**
 * @fileoverview Render defaults
 */

import type { RenderConfig } from './types.js';

export const DEFAULT_RENDER_CONFIG: Required<RenderConfig> = {
  enableMiniMap: true,
  enableControls: true,
  fitView: true,
  nodesDraggable: true,
  snapToGrid: false,
  gridSize: 15,
  nodesConnectable: true,
  elementsSelectable: true,
  enableZoom: true,
  enablePan: true,
  enableSelection: true
};
