import type { VisualizationState } from '../../core/VisualizationState';
import type { ExternalContainer as Container } from '../../shared/types';
import type { ReactFlowNode } from '../ReactFlowTypes';
import { 
  computeChildContainerPosition,
  computeRootContainerPosition,
  getAdjustedContainerDimensionsSafe,
  buildParentMap as buildParentMapUtil,
  sortContainersByHierarchy as sortContainersByHierarchyUtil
} from '../bridgeUtils';

export function buildParentMap(state: VisualizationState): Map<string, string> {
  return buildParentMapUtil(state);
}

export function sortContainersByHierarchy<T extends { id: string }>(containers: T[], parentMap: Map<string, string>): T[] {
  return sortContainersByHierarchyUtil(containers, parentMap) as T[];
}

export function convertContainersFromELK(
  visState: VisualizationState,
  nodes: ReactFlowNode[],
  parentMap: Map<string, string>,
  colorPalette: string
): void {
  const containers = Array.from(visState.visibleContainers);
  const sorted = sortContainersByHierarchy(containers, parentMap);

  sorted.forEach(container => {
    const parentId = parentMap.get(container.id);
    const position = parentId
      ? computeChildContainerPosition(visState, container, parentId)
      : computeRootContainerPosition(visState, container);

    const { width, height } = getAdjustedContainerDimensionsSafe(visState, container.id);
    const nodeCount = container.collapsed ? visState.getContainerChildren(container.id)?.size || 0 : 0;

    const containerNode: ReactFlowNode = {
      id: container.id,
      type: 'container',
      position,
      data: {
        label: container.label || container.id,
        style: (container as unknown as { style?: string }).style || 'default',
        collapsed: container.collapsed,
        colorPalette,
        width,
        height,
        nodeCount
      },
  style: { width, height },
  parentId: parentId || undefined,
  extent: parentId ? 'parent' : undefined
    };

    nodes.push(containerNode);
  });
}
