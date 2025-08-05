/**
 * @fileoverview Color utility functions
 * 
 * Simple color utilities for the visualization system.
 */

// Basic color utility functions
export function hexToRgb(hex: string): { r: number; g: number; b: number } | null {
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  return result ? {
    r: parseInt(result[1], 16),
    g: parseInt(result[2], 16),
    b: parseInt(result[3], 16)
  } : null;
}

export function rgbToHex(r: number, g: number, b: number): string {
  return "#" + ((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1);
}

export function getContrastColor(backgroundColor: string): string {
  const rgb = hexToRgb(backgroundColor);
  if (!rgb) return '#000000';
  
  // Calculate brightness using YIQ formula
  const brightness = (rgb.r * 299 + rgb.g * 587 + rgb.b * 114) / 1000;
  return brightness > 128 ? '#000000' : '#ffffff';
}

// Function expected by Legend component
export function generateNodeColors(nodeTypes: string[], palette: string = 'Set3', nodeTypeConfig?: any): Record<string, string> {
  const colors: Record<string, string> = {};
  const defaultColors = [
    '#8dd3c7', '#bebada', '#80b1d3', '#fccde5',
    '#d9d9d9', '#bc80bd', '#ccebc5', '#ffed6f'
  ];
  
  nodeTypes.forEach((nodeType, index) => {
    colors[nodeType] = defaultColors[index % defaultColors.length];
  });
  
  return colors;
}
