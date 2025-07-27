import React from 'react';
import { getBezierPath } from '@xyflow/react';

// This is a custom edge component that correctly renders edges between child nodes
// inside a container. It works by calculating the absolute positions of the source
// and target nodes, including the position of their parent container.
export default function CustomEdge({
  id,
  source,
  target,
  markerEnd,
  style,
}) {
  // Use getBezierPath from @xyflow/react
  // Use ReactFlow store to get nodes - this is the v12 way
  const nodes = window.ReactFlow?.store?.getState()?.nodes || [];
  const sourceNode = nodes.find((n) => n.id === source);
  const targetNode = nodes.find((n) => n.id === target);

  if (!sourceNode || !targetNode) {
    console.warn('CustomEdge: source or target node not found', { id, source, target, nodes });
    return null;
  }

  // Find parent containers for both source and target nodes (v12: parentId instead of parentNode)
  const sourceParent = sourceNode.parentId ? nodes.find((n) => n.id === sourceNode.parentId) : null;
  const targetParent = targetNode.parentId ? nodes.find((n) => n.id === targetNode.parentId) : null;

  // Calculate absolute positions for source and target nodes
  // v12: Use measured.width/height instead of width/height
  const sourceAbsX = (sourceParent ? sourceParent.positionAbsolute?.x || 0 : 0) + sourceNode.position.x;
  const sourceAbsY = (sourceParent ? sourceParent.positionAbsolute?.y || 0 : 0) + sourceNode.position.y;
  const targetAbsX = (targetParent ? targetParent.positionAbsolute?.x || 0 : 0) + targetNode.position.x;
  const targetAbsY = (targetParent ? targetParent.positionAbsolute?.y || 0 : 0) + targetNode.position.y;

  // We also need to account for the node's dimensions to connect to the center of the sides.
  // v12: Use measured dimensions
  const sourceWidth = sourceNode.measured?.width || sourceNode.width || 100;
  const sourceHeight = sourceNode.measured?.height || sourceNode.height || 50;
  const targetWidth = targetNode.measured?.width || targetNode.width || 100;
  const targetHeight = targetNode.measured?.height || targetNode.height || 50;
  
  const sourceX = sourceAbsX + (sourceWidth / 2);
  const sourceY = sourceAbsY + (sourceHeight / 2);
  const targetX = targetAbsX + (targetWidth / 2);
  const targetY = targetAbsY + (targetHeight / 2);

  // Debug logging
  console.log('CustomEdge rendering', {
    id,
    source,
    target,
    sourceNode,
    targetNode,
    sourceParent,
    targetParent,
    sourceX,
    sourceY,
    targetX,
    targetY,
    markerEnd,
    style
  });

  // Get the edge path using ReactFlow's helper function
  const [edgePath] = getBezierPath({
    sourceX,
    sourceY,
    targetX,
    targetY,
  });

  return (
    <>
      {/* Debug marker: red dot at source, blue dot at target, and label */}
      <circle cx={sourceX} cy={sourceY} r={6} fill="red" />
      <circle cx={targetX} cy={targetY} r={6} fill="blue" />
      <text x={sourceX} y={sourceY - 10} fill="red" fontSize="12">{id}</text>
      <path
        id={id}
        className="react-flow__edge-path"
        d={edgePath}
        markerEnd={markerEnd}
        style={style}
      />
    </>
  );
}
