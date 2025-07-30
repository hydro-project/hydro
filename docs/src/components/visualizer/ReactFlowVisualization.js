/**
 * ReactFlow Visualization Wrapper
 * 
 * Drop-in replacement for the original ReactFlowVisualization component
 */

import React from 'react';
import { Visualizer } from './Visualizer.js';

export function ReactFlowVisualization({ graphData, onControlsReady }) {
  return <Visualizer graphData={graphData} onControlsReady={onControlsReady} />;
}
