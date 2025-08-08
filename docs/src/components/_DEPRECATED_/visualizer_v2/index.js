/**
 * Hydro Graph Visualizer - Main Export
 * 
 * Simplified visualizer components with shared configuration and DRY principles
 */

export { Visualizer } from './Visualizer.js';
export { ReactFlowVisualization } from './ReactFlowVisualization.js';
export { FileDropZone } from './components/FileDropZone.js';
export { GroupingControls } from './components/GroupingControls.js';

// Container collapse/expand functionality
export { 
  useCollapsedContainers, 
  CollapsedContainerNode,
  processCollapsedContainers,
  rerouteEdgesForCollapsedContainers
} from './containers/index.js';
