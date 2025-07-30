/**
 * Hook for managing collapsed container state
 * Handles toggling, storing state, and determining when to reroute edges
 */

import { useState, useCallback, useMemo } from 'react';

export function useCollapsedContainers(nodes) {
  const [collapsedContainers, setCollapsedContainers] = useState(new Set());

  // Toggle a container's collapsed state
  const toggleContainer = useCallback((containerId) => {
    console.log('toggleContainer called with:', containerId);
    setCollapsedContainers(prev => {
      const newSet = new Set(prev);
      if (newSet.has(containerId)) {
        console.log('Expanding container:', containerId);
        newSet.delete(containerId);
      } else {
        console.log('Collapsing container:', containerId);
        newSet.add(containerId);
      }
      console.log('New collapsed containers:', Array.from(newSet));
      return newSet;
    });
  }, []);

  // Check if a container is collapsed
  const isCollapsed = useCallback((containerId) => {
    return collapsedContainers.has(containerId);
  }, [collapsedContainers]);

  // Get all child node IDs for each parent container
  const childNodesByParent = useMemo(() => {
    const map = new Map();
    
    nodes.forEach(node => {
      if (node.parentId) {
        if (!map.has(node.parentId)) {
          map.set(node.parentId, new Set());
        }
        map.get(node.parentId).add(node.id);
      }
    });
    
    return map;
  }, [nodes]);

  // Collapse all containers
  const collapseAll = useCallback(() => {
    const groupNodes = nodes.filter(node => node.type === 'group');
    setCollapsedContainers(new Set(groupNodes.map(node => node.id)));
  }, [nodes]);

  // Expand all containers
  const expandAll = useCallback(() => {
    setCollapsedContainers(new Set());
  }, []);

  return {
    collapsedContainers,
    toggleContainer,
    isCollapsed,
    childNodesByParent,
    collapseAll,
    expandAll,
    hasCollapsedContainers: collapsedContainers.size > 0,
  };
}
