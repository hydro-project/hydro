/**
 * Hydro Graph Visualizer - Main Export
 * 
 * Simplified visualizer components with shared configuration and DRY principles
 */

export { Visualizer } from './Visualizer.js';
export { GraphCanvas } from './components/GraphCanvas.js';
export { ReactFlowVisualization } from './ReactFlowVisualization.js';
export { FileDropZone } from './components/FileDropZone.js';

// Container collapse/expand functionality
export { 
  useCollapsedContainers, 
  CollapsedContainerNode,
  CollapsedContainerWrapper,
  processCollapsedContainers,
  rerouteEdgesForCollapsedContainers
} from './containers/index.js';
