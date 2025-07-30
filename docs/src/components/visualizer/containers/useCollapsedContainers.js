/**
 * Hook for managing collapsed container state
 * Handles toggling, storing state, and determining when to reroute edges
 */

import { useState, useCallback, useMemo } from 'react';
import { filterNodesByType } from '../utils/constants.js';

export function useCollapsedContainers(nodes) {
  const [collapsedContainers, setCollapsedContainers] = useState(new Set());

  // Toggle a container's collapsed state
  const toggleContainer = useCallback((containerId) => {
    setCollapsedContainers(prev => {
      const newSet = new Set(prev);
      if (newSet.has(containerId)) {
        newSet.delete(containerId);
      } else {
        newSet.add(containerId);
      }
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
    const groupNodes = filterNodesByType(nodes, 'group');
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
