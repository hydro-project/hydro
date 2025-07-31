/**
 * Centralized color constants for the visualizer
 * Provides consistent color values across components
 */

export const COLORS = {
  // Default colors
  DEFAULT_GRAY: 'rgba(181, 182, 183, 1)',
  DEFAULT_GRAY_ALPHA: 'rgba(242, 242, 243, 0.25)',
  DEFAULT_GREEN: 'rgb(16, 185, 129)',
  DEFAULT_ORANGE: 'rgb(245, 158, 11)',
  
  // UI colors
  WHITE: '#fff',
  WHITE_ALPHA: 'rgba(255, 255, 255, 0.9)',
  GRAY_LIGHT: '#ccc',
  BLACK_ALPHA: 'rgba(0,0,0,0.1)',
  BLACK_SEMI_ALPHA: 'rgba(0,0,0,0.6)',
};

export const DEFAULT_STYLES = {
  BORDER_WIDTH: '2px',
  BORDER_RADIUS: '8px',
  BOX_SHADOW: '0 2px 4px rgba(0,0,0,0.1)',
};

// Common validation utilities
export const isValidGraphData = (graphData) => {
  return graphData && graphData.nodes && graphData.nodes.length > 0;
};

export const isValidNodesArray = (nodes) => {
  return nodes && nodes.length > 0;
};

// Common node filtering utilities
export const filterNodesByType = (nodes, type) => {
  return nodes.filter(node => node.type === type);
};

export const filterNodesByParent = (nodes, parentId) => {
  return nodes.filter(node => node.parentId === parentId);
};

export const filterNodesExcludingType = (nodes, type) => {
  return nodes.filter(node => node.type !== type);
};

export const getUniqueNodesById = (nodes) => {
  return nodes.filter((node, index, array) => 
    array.findIndex(n => n.id === node.id) === index
  );
};
