/**
 * @fileoverview Hierarchy Tree Utilities
 * 
 * Utility functions for building and managing hierarchy tree structures
 * from VisualizationState data.
 */

import { HierarchyTreeNode } from './types';
import type { VisualizationState } from '../core/VisualizationState';
import type { ExternalContainer as Container } from '../shared/types';

/**
 * Build hierarchy tree from VisualizationState
 */
export function buildHierarchyTree(visualizationState: VisualizationState | null | undefined, grouping: string = 'default'): HierarchyTreeNode[] {
  if (!visualizationState) {
    return [];
  }

  // Get all visible containers and build a map
  const containers: ReadonlyArray<Container> = visualizationState.visibleContainers || [];
  const containerMap = new Map<string, {
    id: string;
    label: string;
    children: HierarchyTreeNode[];
    nodeCount: number;
    parentId: string | null;
  }>();

  // Build initial container map with parent relationships
  containers.forEach((container: Container) => {
    // Find parent by checking which container has this container as a child
    let parentId: string | null = null;
  for (const potentialParent of containers) {
      if (potentialParent.id !== container.id && 
          potentialParent.children && 
          potentialParent.children.has && 
          potentialParent.children.has(container.id)) {
        parentId = potentialParent.id;
        break;
      }
    }
    
    containerMap.set(container.id, {
      id: container.id,
  label: (container as unknown as { data?: { label?: string }; label?: string }).data?.label || (container as unknown as { label?: string }).label || container.id,
      children: [],
      nodeCount: container.children ? container.children.size : 0,
      parentId: parentId,
    });
  });

  // Recursively build tree structure 
  const buildTree = (parentId: string | null): HierarchyTreeNode[] => {
    const children: HierarchyTreeNode[] = [];
    for (const container of containerMap.values()) {
      if (container.parentId === parentId) {
        const grandchildren = buildTree(container.id);
        children.push({
          id: container.id,
          label: container.label,
          children: grandchildren,
          nodeCount: container.nodeCount
        });
      }
    }
    return children;
  };

  return buildTree(null); // Start with root containers (no parent)
}

/**
 * Get collapsed containers set from VisualizationState
 */
export function getCollapsedContainersSet(visualizationState: VisualizationState | null | undefined): Set<string> {
  if (!visualizationState) {
    return new Set();
  }

  return new Set(
    (visualizationState.visibleContainers as ReadonlyArray<Container>)
      .filter((container: Container) => container.collapsed)
      .map((container: Container) => container.id)
  );
}

/**
 * Get expanded keys for HierarchyTree (inverse of collapsed containers)
 */
export function getExpandedKeysForHierarchyTree(
  hierarchyTree: HierarchyTreeNode[], 
  collapsedContainers: Set<string>
): string[] {
  const allKeys: string[] = [];
  
  const collectKeys = (nodes: HierarchyTreeNode[]) => {
    nodes.forEach(node => {
      allKeys.push(node.id);
      if (node.children) {
        collectKeys(node.children);
      }
    });
  };
  
  collectKeys(hierarchyTree);
  
  // Return keys that are NOT in collapsedContainers (i.e., expanded keys)
  return allKeys.filter(key => !collapsedContainers.has(key));
}

/**
 * Validate that HierarchyTree state is synchronized with VisualizationState
 */
export function validateHierarchyTreeSync(
  visualizationState: VisualizationState,
  hierarchyTree: HierarchyTreeNode[],
  collapsedContainers: Set<string>
): {
  isValid: boolean;
  errors: string[];
} {
  const errors: string[] = [];
  
  if (!visualizationState) {
    errors.push('VisualizationState is null or undefined');
    return { isValid: false, errors };
  }

  // Check that collapsed containers set matches actual collapsed state
  const actualCollapsedContainers = getCollapsedContainersSet(visualizationState);
  
  // Check for containers in collapsedContainers that are not actually collapsed
  for (const containerId of collapsedContainers) {
    if (!actualCollapsedContainers.has(containerId)) {
      errors.push(`Container ${containerId} is in collapsedContainers set but is not actually collapsed in VisualizationState`);
    }
  }
  
  // Check for containers that are collapsed but not in collapsedContainers set
  for (const containerId of actualCollapsedContainers) {
    if (!collapsedContainers.has(containerId)) {
      errors.push(`Container ${containerId} is collapsed in VisualizationState but not in collapsedContainers set`);
    }
  }
  
  // Check that all containers in hierarchyTree exist in VisualizationState
  const checkHierarchyExists = (nodes: HierarchyTreeNode[]) => {
    nodes.forEach(node => {
      const container = visualizationState.getContainer(node.id);
      if (!container) {
        errors.push(`Container ${node.id} exists in hierarchyTree but not in VisualizationState`);
      }
      if (node.children) {
        checkHierarchyExists(node.children);
      }
    });
  };
  
  checkHierarchyExists(hierarchyTree);
  
  return {
    isValid: errors.length === 0,
    errors
  };
}
