import type { VisualizationState } from '../../core/VisualizationState';
import type { Edge as ReactFlowEdge } from '@xyflow/react';
import { convertEdgesToReactFlow, EdgeBridgeOptions } from '../EdgeBridge';
import type { EdgeStyleConfig } from '../../core/EdgeStyleProcessor';

export function convertEdges(
  visState: VisualizationState,
  edges: ReactFlowEdge[],
  edgeStyleConfig: EdgeStyleConfig | undefined
): void {
  const visibleEdges = visState.visibleEdges as unknown as Array<any>;

  const edgeBridgeOptions: EdgeBridgeOptions = {
    edgeStyleConfig,
    showPropertyLabels: true,
    enableAnimations: true
  };

  const convertedEdges = convertEdgesToReactFlow(visibleEdges, edgeBridgeOptions);
  edges.push(...convertedEdges);
}
