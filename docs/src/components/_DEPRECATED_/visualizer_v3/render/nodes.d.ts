/**
 * @fileoverview Bridge-Based Node Components
 *
 * ReactFlow node components with configurable handle system for maximum layout flexibility
 */
import { type NodeProps } from '@xyflow/react';
/**
 * Standard graph node component
 */
export declare function StandardNode({ id, data }: NodeProps): import("react/jsx-runtime").JSX.Element;
/**
 * Container node component
 */
export declare function ContainerNode({ id, data }: NodeProps): import("react/jsx-runtime").JSX.Element;
export declare const nodeTypes: {
    standard: typeof StandardNode;
    container: typeof ContainerNode;
};
//# sourceMappingURL=nodes.d.ts.map