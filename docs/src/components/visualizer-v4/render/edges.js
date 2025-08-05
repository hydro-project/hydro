import { jsx as _jsx } from "react/jsx-runtime";
import { BaseEdge, getStraightPath } from '@xyflow/react';
/**
 * Standard graph edge component
 */
export function StandardEdge(props) {
    const [edgePath] = getStraightPath({
        sourceX: props.sourceX,
        sourceY: props.sourceY,
        targetX: props.targetX,
        targetY: props.targetY,
    });
    return (_jsx(BaseEdge, { path: edgePath, markerEnd: props.markerEnd, style: { stroke: '#1976d2', strokeWidth: 2 } }));
}
/**
 * HyperEdge component
 */
export function HyperEdge(props) {
    const [edgePath] = getStraightPath({
        sourceX: props.sourceX,
        sourceY: props.sourceY,
        targetX: props.targetX,
        targetY: props.targetY,
    });
    return (_jsx(BaseEdge, { path: edgePath, markerEnd: props.markerEnd, style: {
            stroke: '#ff5722',
            strokeWidth: 3,
            strokeDasharray: '5,5'
        } }));
}
// Export map for ReactFlow edgeTypes
export const edgeTypes = {
    standard: StandardEdge,
    hyper: HyperEdge
};
//# sourceMappingURL=edges.js.map