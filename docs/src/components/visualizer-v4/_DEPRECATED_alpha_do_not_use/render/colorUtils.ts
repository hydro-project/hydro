/**
 * @fileoverview Color manipulation utilities for render components
 * 
 * Shared utilities to eliminate color-related code duplication.
 */

/**
 * Converts hex color to RGB values
 */
export function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const normalizedHex = hex.replace('#', '');
  return {
    r: parseInt(normalizedHex.substr(0, 2), 16),
    g: parseInt(normalizedHex.substr(2, 2), 16),
    b: parseInt(normalizedHex.substr(4, 2), 16)
  };
}

/**
 * Converts RGB values to RGB string
 */
export function rgbToString(r: number, g: number, b: number): string {
  return `rgb(${r}, ${g}, ${b})`;
}

/**
 * Creates a darker border color from a base color
 */
export function createDarkBorder(color: string, factor: number = 0.6): string {
  const { r, g, b } = hexToRgb(color);
  
  const darkR = Math.floor(r * factor);
  const darkG = Math.floor(g * factor);
  const darkB = Math.floor(b * factor);
  
  return rgbToString(darkR, darkG, darkB);
}

/**
 * Creates a vertical gradient from a base color
 */
export function createVerticalGradient(color: string, topFactor: number = 0.8, bottomFactor: number = 1.2): string {
  const { r, g, b } = hexToRgb(color);
  
  // Create darker top and lighter bottom
  const topR = Math.floor(r * topFactor);
  const topG = Math.floor(g * topFactor);
  const topB = Math.floor(b * topFactor);
  
  const bottomR = Math.min(Math.floor(r * bottomFactor), 255);
  const bottomG = Math.min(Math.floor(g * bottomFactor), 255);
  const bottomB = Math.min(Math.floor(b * bottomFactor), 255);
  
  const topColor = rgbToString(topR, topG, topB);
  const bottomColor = rgbToString(bottomR, bottomG, bottomB);
  
  return `linear-gradient(to bottom, ${topColor}, ${bottomColor})`;
}

/**
 * Gets node color based on type (shared logic)
 */
export function getNodeColorByType(nodeType: string): string {
  switch (nodeType) {
    case 'Source': return '#8dd3c7'; // Light teal
    case 'Transform': return '#ffffb3'; // Light yellow  
    case 'Tee': return '#bebada'; // Light purple
    case 'Network': return '#fb8072'; // Light red/salmon
    case 'Sink': return '#80b1d3'; // Light blue
    default: return '#b3de69'; // Light green
  }
}