/**
 * @fileoverview Custom ReactFlow Node Components
 *
 * Custom node c      <Handle
        type="target"
        position={Position.Left}
        style={{ background: NODE_COLORS.HANDLE }}
      />
      <Handle
        type="source"
        position={Position.Right}
        style={{ background: NODE_COLORS.HANDLE }}
      />for rendering graph elements.
 */
import React from 'react';
import { StandardNodeProps, ContainerNodeProps } from './types';
export declare const GraphStandardNode: React.FC<StandardNodeProps>;
export declare const GraphContainerNode: React.FC<ContainerNodeProps>;
//# sourceMappingURL=nodes.d.ts.map