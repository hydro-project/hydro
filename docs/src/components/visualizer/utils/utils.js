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

/**
 * Truncates a container name if it's longer than the specified max length
 * @param {string} name - The container name to truncate
 * @param {number} maxLength - Maximum length before truncation (default: 15)
 * @param {Object} options - Truncation options
 * @param {string} options.side - 'left' or 'right' truncation (default: 'left')
 * @param {boolean} options.splitOnDelimiter - Whether to favor splitting at delimiters (default: false)
 * @param {number} options.delimiterPenalty - Percentage penalty for delimiter split being longer (default: 0.2)
 * @returns {string} The truncated name with ellipsis if needed
 */
export function truncateContainerName(name, maxLength = 15, options = {}) {
  const {
    side = 'left',
    splitOnDelimiter = false,
    delimiterPenalty = 0.2
  } = options;

  if (!name || typeof name !== 'string') {
    return name;
  }
  
  if (name.length <= maxLength) {
    return name;
  }

  // Common delimiters used in hierarchical names
  const delimiters = ['::', '/', '.', '\\', '->', '<-', '|', '@', '#'];
  
  if (splitOnDelimiter) {
    // Find the best delimiter split position
    const bestSplit = findBestDelimiterSplit(name, maxLength, delimiters, side, delimiterPenalty);
    if (bestSplit) {
      return bestSplit;
    }
  }
  
  // Fallback to simple truncation
  if (side === 'left') {
    return '...' + name.slice(-(maxLength - 3));
  } else {
    return name.slice(0, maxLength - 3) + '...';
  }
}

/**
 * Finds the best position to split a string at delimiters
 * @param {string} name - The string to split
 * @param {number} maxLength - Maximum allowed length
 * @param {string[]} delimiters - Array of delimiters to consider
 * @param {string} side - Which side to truncate ('left' or 'right')
 * @param {number} penalty - Penalty factor for length overrun
 * @returns {string|null} The best split result or null if no good split found
 */
function findBestDelimiterSplit(name, maxLength, delimiters, side, penalty) {
  const maxPenaltyLength = Math.floor(maxLength * (1 + penalty));
  let bestSplit = null;
  let bestScore = Infinity;
  
  for (const delimiter of delimiters) {
    const positions = [];
    let pos = name.indexOf(delimiter);
    
    // Find all positions of this delimiter
    while (pos !== -1) {
      positions.push(pos);
      pos = name.indexOf(delimiter, pos + 1);
    }
    
    for (const delimiterPos of positions) {
      let candidate, score;
      
      if (side === 'left') {
        // Keep the right part after the delimiter
        const rightPart = name.slice(delimiterPos + delimiter.length);
        if (rightPart.length <= maxLength) {
          candidate = rightPart;
          score = maxLength - rightPart.length; // Prefer longer results
        } else if (rightPart.length <= maxPenaltyLength) {
          candidate = '...' + rightPart.slice(-(maxLength - 3));
          score = rightPart.length - maxLength + 1000; // Penalty for being too long
        }
      } else {
        // Keep the left part before the delimiter
        const leftPart = name.slice(0, delimiterPos);
        if (leftPart.length <= maxLength) {
          candidate = leftPart;
          score = maxLength - leftPart.length; // Prefer longer results
        } else if (leftPart.length <= maxPenaltyLength) {
          candidate = leftPart.slice(0, maxLength - 3) + '...';
          score = leftPart.length - maxLength + 1000; // Penalty for being too long
        }
      }
      
      if (candidate && score < bestScore) {
        bestSplit = candidate;
        bestScore = score;
      }
    }
  }
  
  return bestSplit;
}
