/**
 * @fileoverview Bridge-Based Edge Components
 *
 * ReactFlow edge components for standard and hyper edges
 */
import { EdgeProps } from '@xyflow/react';
/**
 * Standard graph edge component
 */
export declare function StandardEdge(props: EdgeProps): import("react/jsx-runtime").JSX.Element;
/**
 * HyperEdge component
 */
export declare function HyperEdge(props: EdgeProps): import("react/jsx-runtime").JSX.Element;
export declare const edgeTypes: {
    standard: typeof StandardEdge;
    hyper: typeof HyperEdge;
};
//# sourceMappingURL=edges.d.ts.map