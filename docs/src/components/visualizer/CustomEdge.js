import React from 'react';
import { getBezierPath, useStore } from '@xyflow/react';

// Simplified CustomEdge for ReactFlow v12 
// v12 has much better parent-child positioning, so we can rely more on built-in capabilities
export default function CustomEdge({
  id,
  source,
  target,
  markerEnd,
  style,
}) {
  // Use the proper ReactFlow v12 hook instead of accessing window.ReactFlow
  const { nodeInternals } = useStore();
  const sourceNode = nodeInternals.get(source);
  const targetNode = nodeInternals.get(target);

  if (!sourceNode || !targetNode) {
    // Fallback to built-in edge rendering if nodes not found
    return null;
  }

  // ReactFlow v12 provides positionAbsolute automatically for child nodes
  // This is much more reliable than manual calculations
  const sourceX = (sourceNode.positionAbsolute?.x || sourceNode.position.x) + (sourceNode.measured?.width || sourceNode.width || 100) / 2;
  const sourceY = (sourceNode.positionAbsolute?.y || sourceNode.position.y) + (sourceNode.measured?.height || sourceNode.height || 50) / 2;
  const targetX = (targetNode.positionAbsolute?.x || targetNode.position.x) + (targetNode.measured?.width || targetNode.width || 100) / 2;
  const targetY = (targetNode.positionAbsolute?.y || targetNode.position.y) + (targetNode.measured?.height || targetNode.height || 50) / 2;

  const [edgePath] = getBezierPath({
    sourceX,
    sourceY,
    targetX,
    targetY,
  });

  return (
    <path
      id={id}
      className="react-flow__edge-path"
      d={edgePath}
      markerEnd={markerEnd}
      style={style}
    />
  );
}
