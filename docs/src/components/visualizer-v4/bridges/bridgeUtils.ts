/**
 * @fileoverview Utilities for bridges
 */

import type { VisualizationState } from '../core/VisualizationState';
import { LAYOUT_CONSTANTS } from '../shared/config';
import type { ExternalContainer as Container, GraphNode } from '../shared/types';

/** Build parent-child relationship map for visible elements */
export function buildParentMap(visState: VisualizationState): Map<string, string> {
  const parentMap = new Map<string, string>();

  const visibleContainerIds = new Set(Array.from(visState.visibleContainers).map(c => c.id));
  const visibleNodeIds = new Set(Array.from(visState.visibleNodes).map(n => n.id));

  visState.visibleContainers.forEach(container => {
    const containerChildren = visState.getContainerChildren(container.id);
    containerChildren.forEach(childId => {
      if (visibleContainerIds.has(childId) || visibleNodeIds.has(childId)) {
        parentMap.set(childId, container.id);
      }
    });
  });

  visState.visibleNodes.forEach(node => {
    const n = node as unknown as GraphNode & { containerId?: string };
    if (n.containerId && visibleContainerIds.has(n.containerId)) {
      parentMap.set(node.id, n.containerId);
    }
  });

  return parentMap;
}

/** Sort containers by hierarchy level so parents appear before children */
export function sortContainersByHierarchy<T extends { id: string }>(containers: T[], parentMap: Map<string, string>): T[] {
  const getHierarchyLevel = (containerId: string): number => {
    let level = 0;
    let currentId = containerId;
    while (parentMap.has(currentId)) {
      level++;
      currentId = parentMap.get(currentId)!;
    }
    return level;
  };

  return containers.sort((a, b) => getHierarchyLevel(a.id) - getHierarchyLevel(b.id));
}

/** Ensure a coordinate is a finite number, else 0 */
export function safeNum(n: unknown): number {
  return typeof n === 'number' && !isNaN(n) && isFinite(n) ? n : 0;
}

/** Check if ELK layout has meaningful non-zero coordinates */
export function hasMeaningfulELKPosition(layout: unknown | undefined): boolean {
  const l = layout as { position?: { x?: number; y?: number } } | undefined;
  const x = l?.position?.x;
  const y = l?.position?.y;
  if (x === undefined || y === undefined) return false;
  // Accept 0 if the other coordinate is non-zero; reject 0,0 default
  return !(x === 0 && y === 0);
}

/** Compute relative (child) position from absolute child and parent absolute */
export function toRelativePosition(childAbs: { x: unknown; y: unknown }, parentAbs: { x: unknown; y: unknown }) {
  const cx = safeNum(childAbs.x);
  const cy = safeNum(childAbs.y);
  const px = safeNum(parentAbs.x);
  const py = safeNum(parentAbs.y);
  return { x: cx - px, y: cy - py };
}

/** Compute absolute/root position safely */
export function toAbsolutePosition(abs: { x: unknown; y: unknown }) {
  return { x: safeNum(abs.x), y: safeNum(abs.y) };
}

/** Compute child-container position using ELK when available, else grid fallback */
export function computeChildContainerPosition(
  visState: VisualizationState,
  container: Container,
  parentId: string
) {
  const containerLayout = visState.getContainerLayout(container.id);
  const parentLayout = visState.getContainerLayout(parentId);

  if (hasMeaningfulELKPosition(containerLayout)) {
    const absoluteX = containerLayout?.position?.x ?? container.x ?? 0;
    const absoluteY = containerLayout?.position?.y ?? container.y ?? 0;
    const parentX = parentLayout?.position?.x ?? 0;
    const parentY = parentLayout?.position?.y ?? 0;
    return toRelativePosition({ x: absoluteX, y: absoluteY }, { x: parentX, y: parentY });
  }

  // Grid fallback: position among siblings under same parent
  const siblingContainers = Array.from(visState.visibleContainers).filter(c => {
    const map = buildParentMap(visState);
    return map.get(c.id) === parentId;
  });
  const containerIndex = siblingContainers.findIndex(c => c.id === container.id);

  const cols = LAYOUT_CONSTANTS.CONTAINER_GRID_COLUMNS || 2;
  const col = containerIndex % cols;
  const row = Math.floor(containerIndex / cols);
  const padding = LAYOUT_CONSTANTS.CONTAINER_GRID_PADDING || 20;
  const titleHeight = LAYOUT_CONSTANTS.CONTAINER_TITLE_HEIGHT || 30;

  return {
    x: padding + col * (LAYOUT_CONSTANTS.CHILD_CONTAINER_WIDTH + padding),
    y: titleHeight + row * (LAYOUT_CONSTANTS.CHILD_CONTAINER_HEIGHT + padding)
  };
}

/** Compute root container absolute position safely */
export function computeRootContainerPosition(visState: VisualizationState, container: Container) {
  const containerLayout = visState.getContainerLayout(container.id);
  const rootX = containerLayout?.position?.x ?? container.x ?? 0;
  const rootY = containerLayout?.position?.y ?? container.y ?? 0;
  return toAbsolutePosition({ x: rootX, y: rootY });
}

/** Compute node position from ELK; relative to parent if nested */
export function computeNodePosition(
  visState: VisualizationState,
  node: GraphNode,
  parentId: string | undefined
) {
  const nodeLayout = visState.getNodeLayout(node.id);
  if (parentId) {
    const parentLayout = visState.getContainerLayout(parentId);
    const absoluteX = nodeLayout?.position?.x ?? node.x ?? 0;
    const absoluteY = nodeLayout?.position?.y ?? node.y ?? 0;
    const parentX = parentLayout?.position?.x ?? 0;
    const parentY = parentLayout?.position?.y ?? 0;
    return toRelativePosition({ x: absoluteX, y: absoluteY }, { x: parentX, y: parentY });
  }
  const rootX = nodeLayout?.position?.x ?? node.x ?? 0;
  const rootY = nodeLayout?.position?.y ?? node.y ?? 0;
  return toAbsolutePosition({ x: rootX, y: rootY });
}

/** Get adjusted container dimensions, falling back to defaults when invalid */
export function getAdjustedContainerDimensionsSafe(visState: VisualizationState, containerId: string) {
  const adjusted = (visState.getContainerAdjustedDimensions(containerId) || {}) as { width?: number; height?: number };
  const width = safeNum(adjusted.width) > 0 ? adjusted.width : LAYOUT_CONSTANTS.DEFAULT_PARENT_CONTAINER_WIDTH;
  const height = safeNum(adjusted.height) > 0 ? adjusted.height : LAYOUT_CONSTANTS.DEFAULT_PARENT_CONTAINER_HEIGHT;
  return { width, height };
}
