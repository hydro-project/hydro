/**
 * @fileoverview Label utilities for text truncation and formatting
 * 
 * Shared utility functions for intelligent label truncation and formatting
 * across the visualizer components.
 */

/**
 * Intelligent left-truncation for labels that preserves meaningful parts
 * 
 * @param label - The original label text
 * @param maxLength - Maximum length of the truncated label
 * @returns Truncated label with meaningful parts preserved
 */
export function truncateLabel(label: string, maxLength: number): string {
  if (!label || label.length <= maxLength) {
    return label;
  }
  
  // Try to split on common delimiters and keep the meaningful part
  const delimiters = ['::', '.', '_', '-', '/'];
  for (const delimiter of delimiters) {
    if (label.includes(delimiter)) {
      const parts = label.split(delimiter);
      const lastPart = parts[parts.length - 1];
      
      // If the last part is meaningful and fits, use it
      if (lastPart.length > 2 && lastPart.length <= maxLength) {
        return `…${delimiter}${lastPart}`;
      }
      
      // If there are multiple parts, try to keep 2 meaningful parts
      if (parts.length > 1) {
        const lastTwoParts = parts.slice(-2).join(delimiter);
        if (lastTwoParts.length <= maxLength) {
          return `…${lastTwoParts}`;
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
 * Count leaf nodes in a container (nodes that are not containers themselves)
 * 
 * @param container - Container object with children set
 * @param visState - VisualizationState to query for child types
 * @returns Number of leaf nodes in the container
 */
export function countLeafNodes(container: any, visState: any): number {
  if (!container.children) {
    return 0;
  }
  
  let leafCount = 0;
  const children = Array.isArray(container.children) ? container.children : Array.from(container.children);
  
  for (const childId of children) {
    // Check if child is a container or a regular node
    const childContainer = visState.getContainer?.(childId);
    if (childContainer) {
      // Child is a container, recurse into it
      leafCount += countLeafNodes(childContainer, visState);
    } else {
      // Child is a leaf node
      leafCount++;
    }
  }
  
  return leafCount;
}

/**
 * Generate summary text for collapsed containers
 * 
 * @param container - Container object
 * @param visState - VisualizationState to query for children
 * @returns Summary text like "5 nodes" or "3 containers, 7 nodes"
 */
export function generateContainerSummary(container: any, visState: any): string {
  if (!container.children) {
    return '0 nodes';
  }
  
  const children = Array.isArray(container.children) ? container.children : Array.from(container.children);
  let containerCount = 0;
  let nodeCount = 0;
  
  for (const childId of children) {
    const childContainer = visState.getContainer?.(childId);
    if (childContainer) {
      containerCount++;
    } else {
      nodeCount++;
    }
  }
  
  if (containerCount === 0) {
    return `${nodeCount} node${nodeCount !== 1 ? 's' : ''}`;
  } else if (nodeCount === 0) {
    return `${containerCount} container${containerCount !== 1 ? 's' : ''}`;
  } else {
    return `${containerCount} container${containerCount !== 1 ? 's' : ''}, ${nodeCount} node${nodeCount !== 1 ? 's' : ''}`;
  }
}