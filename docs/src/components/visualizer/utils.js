/**
 * Utility functions for the visualizer
 */

// Color palettes for different node types
const colorPalettes = {
  Set3: [
    { primary: '#8dd3c7', secondary: '#ffffb3' },
    { primary: '#bebada', secondary: '#fb8072' },
    { primary: '#80b1d3', secondary: '#fdb462' },
    { primary: '#fccde5', secondary: '#b3de69' },
    { primary: '#d9d9d9', secondary: '#fccde5' },
    { primary: '#bc80bd', secondary: '#ccebc5' },
    { primary: '#ccebc5', secondary: '#ffed6f' },
    { primary: '#ffed6f', secondary: '#8dd3c7' },
  ],
  Pastel1: [
    { primary: '#fbb4ae', secondary: '#b3cde3' },
    { primary: '#ccebc5', secondary: '#decbe4' },
    { primary: '#fed9a6', secondary: '#fddaec' },
    { primary: '#f2f2f2', secondary: '#e5d8bd' },
    { primary: '#b3de69', secondary: '#fbb4ae' },
  ],
  Dark2: [
    { primary: '#1b9e77', secondary: '#d95f02' },
    { primary: '#7570b3', secondary: '#e7298a' },
    { primary: '#66a61e', secondary: '#e6ab02' },
    { primary: '#a6761d', secondary: '#666666' },
  ],
};

const nodeTypeColors = {
  'Source': 0,
  'Transform': 1,
  'Sink': 2,
  'Network': 3,
  'Operator': 4,
  'Join': 5,
  'Union': 6,
  'Filter': 7,
};

export function generateNodeColors(nodeType, paletteKey = 'Set3') {
  const palette = colorPalettes[paletteKey] || colorPalettes.Set3;
  const colorIndex = nodeTypeColors[nodeType] || 0;
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

// Location-specific color functions removed
// Location data is still tracked internally but not used for visualization

function darkenColor(hex, factor) {
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

function lightenColor(hex, factor) {
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
