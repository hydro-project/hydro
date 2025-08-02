/**
 * Color utilities for the vis system
 * Ported from the original visualizer to ensure consistent color mapping
 */

import { COLOR_PALETTES } from './constants.js';

/**
 * Generate node colors dynamically based on provided node type configuration
 * @param nodeType - The node type to get colors for
 * @param paletteKey - The color palette to use
 * @param nodeTypeConfig - Configuration object with node type mappings
 * @returns Color configuration for the node type
 */
export function generateNodeColors(nodeType: string, paletteKey: string = 'Set3', nodeTypeConfig: any = null) {
  const palette = COLOR_PALETTES[paletteKey] || COLOR_PALETTES.Set3;
  
  // Use provided configuration or fall back to defaults
  let colorIndex = 0; // Default color index
  if (nodeTypeConfig?.types) {
    const typeConfig = nodeTypeConfig.types.find((t: any) => t.id === nodeType);
    if (typeConfig && typeof typeConfig.colorIndex === 'number') {
      colorIndex = typeConfig.colorIndex;
    }
  } else {
    // Legacy fallback for backwards compatibility
    const defaultMapping: Record<string, number> = {
      'Source': 0,
      'Transform': 1,
      'Sink': 2,
      'Network': 3,
      'Operator': 4,
      'Join': 5,
      'Union': 6,
      'Filter': 7,
    };
    colorIndex = defaultMapping[nodeType] || 0;
  }
  
  const colors = palette[colorIndex % palette.length];
  
  // Create a subtle gradient using only the primary color with lighter/darker shades
  const lighterPrimary = lightenColor(colors.primary, 0.1);
  const darkerPrimary = darkenColor(colors.primary, 0.1);
  
  return {
    primary: colors.primary,
    secondary: colors.secondary,
    border: darkenColor(colors.primary, 0.3),
    gradient: `linear-gradient(145deg, ${lighterPrimary}, ${darkerPrimary})`,
  };
}

function darkenColor(hex: string, factor: number): string {
  // Remove # if present
  hex = hex.replace('#', '');
  
  // Parse RGB
  const r = parseInt(hex.substring(0, 2), 16);
  const g = parseInt(hex.substring(2, 4), 16);
  const b = parseInt(hex.substring(4, 6), 16);
  
  // Darken by factor
  const newR = Math.floor(r * (1 - factor));
  const newG = Math.floor(g * (1 - factor));
  const newB = Math.floor(b * (1 - factor));
  
  // Convert back to hex
  return `#${newR.toString(16).padStart(2, '0')}${newG.toString(16).padStart(2, '0')}${newB.toString(16).padStart(2, '0')}`;
}

function lightenColor(hex: string, factor: number): string {
  // Remove # if present
  hex = hex.replace('#', '');
  
  // Parse RGB
  const r = parseInt(hex.substring(0, 2), 16);
  const g = parseInt(hex.substring(2, 4), 16);
  const b = parseInt(hex.substring(4, 6), 16);
  
  // Lighten by factor
  const newR = Math.floor(r + (255 - r) * factor);
  const newG = Math.floor(g + (255 - g) * factor);
  const newB = Math.floor(b + (255 - b) * factor);
  
  // Convert back to hex
  return `#${newR.toString(16).padStart(2, '0')}${newG.toString(16).padStart(2, '0')}${newB.toString(16).padStart(2, '0')}`;
}
