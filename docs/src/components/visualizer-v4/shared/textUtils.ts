/**
 * @fileoverview Text utilities for the visualizer
 * 
 * Shared text processing functions like truncation, formatting, etc.
 */

/**
 * Configuration for smart label truncation
 */
export interface TruncationOptions {
  /** Maximum length of the truncated text */
  maxLength: number;
  /** Whether to prefer breaking on delimiters */
  preferDelimiters?: boolean;
  /** Custom delimiters to consider for truncation */
  delimiters?: string[];
  /** Whether to truncate from the left (keeping the end) */
  leftTruncate?: boolean;
}

/**
 * Default delimiters commonly found in container/node names
 */
export const DEFAULT_DELIMITERS = ['::', '.', '_', '-', '/', ' '];

/**
 * Smart truncation that preserves meaningful parts of labels
 * Based on the logic from HierarchyTree.tsx
 */
export function truncateLabel(
  label: string, 
  options: TruncationOptions
): string {
  const {
    maxLength,
    preferDelimiters = true,
    delimiters = DEFAULT_DELIMITERS,
    leftTruncate = false
  } = options;

  // Early return if no truncation needed
  if (!label || label.length <= maxLength) {
    return label;
  }

  // For left truncation (keeping the end), we want to preserve the rightmost meaningful part
  if (leftTruncate && preferDelimiters) {
    return leftTruncateWithDelimiters(label, maxLength, delimiters);
  }

  // For right truncation (keeping the beginning), try delimiter-based truncation
  if (preferDelimiters) {
    return rightTruncateWithDelimiters(label, maxLength, delimiters);
  }

  // Fallback to simple truncation
  return leftTruncate 
    ? `…${label.slice(-(maxLength - 1))}`
    : `${label.slice(0, maxLength - 1)}…`;
}

/**
 * Left-truncate keeping the meaningful ending (like Rust paths)
 * This is what's used for collapsed containers
 */
function leftTruncateWithDelimiters(
  label: string, 
  maxLength: number, 
  delimiters: string[]
): string {
  // Try each delimiter to find the best truncation
  for (const delimiter of delimiters) {
    if (label.includes(delimiter)) {
      const parts = label.split(delimiter);
      const lastPart = parts[parts.length - 1];
      
      // If the last part is meaningful and fits, use it with ellipsis
      if (lastPart.length > 2 && lastPart.length <= maxLength - 2) {
        return `…${delimiter}${lastPart}`;
      }
      
      // Try to keep multiple meaningful parts from the end
      if (parts.length > 1) {
        // Start with the last part and add more from the right
        let result = lastPart;
        for (let i = parts.length - 2; i >= 0; i--) {
          const candidate = parts[i] + delimiter + result;
          const withEllipsis = `…${delimiter}${result}`;
          
          if (candidate.length <= maxLength) {
            // Can fit without ellipsis
            result = candidate;
          } else if (withEllipsis.length <= maxLength) {
            // Need ellipsis but it fits
            return withEllipsis;
          } else {
            // Doesn't fit even with ellipsis, return previous result
            break;
          }
        }
        
        if (result.length <= maxLength) {
          return result;
        }
      }
    }
  }
  
  // Fallback: simple left truncation
  return `…${label.slice(-(maxLength - 1))}`;
}

/**
 * Right-truncate keeping the meaningful beginning
 */
function rightTruncateWithDelimiters(
  label: string, 
  maxLength: number, 
  delimiters: string[]
): string {
  // Try to split on common delimiters and keep meaningful parts
  for (const delimiter of delimiters) {
    if (label.includes(delimiter)) {
      const parts = label.split(delimiter);
      const firstPart = parts[0];
      
      // If the first part is meaningful and fits, use it
      if (firstPart.length > 2 && firstPart.length <= maxLength - 1) {
        return `${firstPart}…`;
      }
      
      // Try to keep multiple meaningful parts from the beginning
      if (parts.length > 1) {
        let result = firstPart;
        for (let i = 1; i < parts.length; i++) {
          const candidate = result + delimiter + parts[i];
          if (candidate.length <= maxLength - 1) {
            result = candidate;
          } else {
            break;
          }
        }
        
        if (result !== label && result.length <= maxLength - 1) {
          return `${result}…`;
        }
      }
    }
  }
  
  // Fallback: smart truncation from the end, keeping whole words when possible  
  if (label.length > maxLength) {
    const truncated = label.slice(0, maxLength - 1);
    const lastSpaceIndex = truncated.lastIndexOf(' ');
    if (lastSpaceIndex > maxLength * 0.7) { // Only break on word if it's not too short
      return truncated.slice(0, lastSpaceIndex) + '…';
    }
    return truncated + '…';
  }
  
  return label;
}

/**
 * Specialized truncation for container names in collapsed state
 * Optimized for Rust-style paths and hierarchical names
 */
export function truncateContainerName(
  name: string, 
  maxLength: number = 22
): string {
  return truncateLabel(name, {
    maxLength,
    preferDelimiters: true,
    delimiters: ['::', '_', '.', '-', '/'],
    leftTruncate: true
  });
}

/**
 * Calculate minimum width needed for a collapsed container based on label
 * Assumes ~8px per character + padding
 */
export function calculateCollapsedWidth(
  label: string, 
  minWidth: number = 200,
  charWidth: number = 8,
  padding: number = 16
): number {
  const truncated = truncateContainerName(label);
  const calculatedWidth = truncated.length * charWidth + padding;
  return Math.max(minWidth, calculatedWidth);
}
