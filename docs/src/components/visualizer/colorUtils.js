/**
 * Color Utilities and Palettes
 * 
 * Contains color palettes and utility functions for generating
 * node colors, location colors, and color manipulations
 */

// Expanded color palettes from template.html
export const colorPalettes = {
  // Qualitative palettes
  'Set3': ['#8dd3c7', '#ffffb3', '#bebada', '#fb8072', '#80b1d3', '#fdb462', '#b3de69'],
  'Pastel1': ['#fbb4ae', '#b3cde3', '#ccebc5', '#decbe4', '#fed9a6', '#ffffcc', '#e5d8bd'],
  'Pastel2': ['#b3e2cd', '#fdcdac', '#cbd5e8', '#f4cae4', '#e6f5c9', '#fff2ae', '#f1e2cc'],
  'Set1': ['#e41a1c', '#377eb8', '#4daf4a', '#984ea3', '#ff7f00', '#ffff33', '#a65628'],
  'Set2': ['#66c2a5', '#fc8d62', '#8da0cb', '#e78ac3', '#a6d854', '#ffd92f', '#e5c494'],
  'Dark2': ['#1b9e77', '#d95f02', '#7570b3', '#e7298a', '#66a61e', '#e6ab02', '#a6761d'],
  'Accent': ['#7fc97f', '#beaed4', '#fdc086', '#ffff99', '#386cb0', '#f0027f', '#bf5b17'],
  'Paired': ['#a6cee3', '#1f78b4', '#b2df8a', '#33a02c', '#fb9a99', '#e31a1c', '#fdbf6f'],
  
  // Sequential palettes
  'Blues': ['#f7fbff', '#deebf7', '#c6dbef', '#9ecae1', '#6baed6', '#4292c6', '#2171b5'],
  'Greens': ['#f7fcf5', '#e5f5e0', '#c7e9c0', '#a1d99b', '#74c476', '#41ab5d', '#238b45'],
  'Oranges': ['#fff5eb', '#fee6ce', '#fdd0a2', '#fdae6b', '#fd8d3c', '#f16913', '#d94801'],
  'Purples': ['#fcfbfd', '#efedf5', '#dadaeb', '#bcbddc', '#9e9ac8', '#807dba', '#6a51a3'],
  'Reds': ['#fff5f0', '#fee0d2', '#fcbba1', '#fc9272', '#fb6a4a', '#ef3b2c', '#cb181d'],
  
  // Diverging palettes
  'Spectral': ['#9e0142', '#d53e4f', '#f46d43', '#fdae61', '#fee08b', '#e6f598', '#abdda4'],
  'RdYlBu': ['#d73027', '#f46d43', '#fdae61', '#fee090', '#e0f3f8', '#abd9e9', '#74add1'],
  'RdYlGn': ['#d73027', '#f46d43', '#fdae61', '#fee08b', '#d9ef8b', '#a6d96a', '#66bd63'],
  'PiYG': ['#d01c8b', '#f1b6da', '#fde0ef', '#f7f7f7', '#e6f5d0', '#b8e186', '#4d9221'],
  'BrBG': ['#8c510a', '#bf812d', '#dfc27d', '#f6e8c3', '#c7eae5', '#80cdc1', '#35978f'],
  
  // Modern/trendy palettes
  'Viridis': ['#440154', '#482777', '#3f4a8a', '#31678e', '#26838f', '#1f9d8a', '#6cce5a'],
  'Plasma': ['#0d0887', '#6a00a8', '#b12a90', '#e16462', '#fca636', '#f0f921', '#fcffa4'],
  'Warm': ['#375a7f', '#5bc0de', '#5cb85c', '#f0ad4e', '#d9534f', '#ad4e92', '#6f5499'],
  'Cool': ['#2c3e50', '#3498db', '#1abc9c', '#16a085', '#27ae60', '#2980b9', '#8e44ad'],
  'Earth': ['#8b4513', '#a0522d', '#cd853f', '#daa520', '#b8860b', '#228b22', '#006400']
};

// Color manipulation functions
export const lightenColor = (color, percent) => `color-mix(in srgb, ${color} ${100-percent}%, white)`;
export const darkenColor = (color, percent) => `color-mix(in srgb, ${color} ${100-percent}%, black)`;

// Color generation functions from template.html
export const generateNodeColors = (nodeType, palette = 'Set3') => {
  const colors = colorPalettes[palette];
  const typeMap = {
    'Source': 0,
    'Transform': 1,
    'Join': 2,
    'Aggregation': 3,
    'Network': 4,
    'Sink': 5,
    'Tee': 6
  };
  
  const baseColor = colors[typeMap[nodeType] || 0];
  
  // Create gradient colors
  const primary = baseColor;
  const secondary = lightenColor(baseColor, 10);
  const tertiary = lightenColor(baseColor, 25);
  const border = darkenColor(baseColor, 5);
  
  // Create a gentle linear gradient
  const gradient = `linear-gradient(0deg, ${tertiary} 0%, ${secondary} 80%, ${primary} 100%)`;
  
  return { primary, secondary, tertiary, border, gradient };
};

export const generateLocationColor = (locationId, totalLocations, palette = 'Set3') => {
  const colors = colorPalettes[palette];
  const color = colors[locationId % colors.length];
  return `${color}40`; // Add transparency
};

export const generateLocationBorderColor = (locationId, totalLocations, palette = 'Set3') => {
  const colors = colorPalettes[palette];
  return colors[locationId % colors.length];
};
