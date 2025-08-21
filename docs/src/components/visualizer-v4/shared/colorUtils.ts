/**
 * @fileoverview Color utility functions
 * 
 * Simple color utilities for the visualization system.
 */

import { COLOR_PALETTES } from './config';

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
export function generateNodeColors(nodeTypes: string[], palette: string = 'Set3', nodeTypeConfig?: any): any {
  // Get the selected palette, fallback to Set3 if not found
  const selectedPalette = COLOR_PALETTES[palette] || COLOR_PALETTES['Set3'] || [];
  
  if (nodeTypes.length === 1) {
    // Single node type - return object with expected properties
    const nodeType = nodeTypes[0];
    // Use a hash of the node type to pick a consistent color
    const colorIndex = Math.abs(nodeType.split('').reduce((a, b) => a + b.charCodeAt(0), 0)) % selectedPalette.length;
    const paletteColor = selectedPalette[colorIndex] || { primary: '#8dd3c7' };
    
    return {
      primary: paletteColor.primary,
      border: paletteColor.primary,
      gradient: paletteColor.primary
    };
  }
  
  // Multiple node types - return a map
  const colors: Record<string, any> = {};
  nodeTypes.forEach((nodeType, index) => {
    const paletteColor = selectedPalette[index % selectedPalette.length] || { primary: '#8dd3c7' };
    colors[nodeType] = {
      primary: paletteColor.primary,
      border: paletteColor.primary,
      gradient: paletteColor.primary
    };
  });
  
  return colors;
}
