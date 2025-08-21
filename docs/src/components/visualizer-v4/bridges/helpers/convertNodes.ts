import type { VisualizationState } from '../../core/VisualizationState';
import type { GraphNode } from '../../shared/types';
import type { ReactFlowNode } from '../ReactFlowTypes';
import { computeNodePosition } from '../bridgeUtils';

export function convertNodesFromELK(
  visState: VisualizationState,
  nodes: ReactFlowNode[],
  parentMap: Map<string, string>,
  colorPalette: string,
  extractCustomProperties: (node: GraphNode) => Record<string, unknown>
): void {
  visState.visibleNodes.forEach(node => {
    const parentId = parentMap.get(node.id) || undefined;
    const position = computeNodePosition(visState, node, parentId);

    const rfNode: ReactFlowNode = {
      id: node.id,
      type: 'standard',
      position,
      data: {
        label: node.label || node.shortLabel || node.id,
        shortLabel: node.shortLabel || node.id,
        fullLabel: node.fullLabel || node.shortLabel || node.id,
        style: node.style || 'default',
        colorPalette,
        ...extractCustomProperties(node)
      },
  parentId,
  extent: parentId ? 'parent' : undefined
    };

    nodes.push(rfNode);
  });
}
