/**
 * @fileoverview Render Configuration
 * 
 * Configuration defaults for the bridge-based renderer.
 */

import type { RenderConfig } from '../core/types';

export const DEFAULT_RENDER_CONFIG: RenderConfig = {
  enableMiniMap: true,
  enableControls: true,
  fitView: true,
  nodesDraggable: true,
  snapToGrid: false,
  gridSize: 20,
  nodesConnectable: false,
  elementsSelectable: true,
  enableZoom: true,
  enablePan: true,
  enableSelection: true
};
