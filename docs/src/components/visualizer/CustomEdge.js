import React from 'react';
import { ReactFlowComponents } from './externalLibraries.js';

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
  // Use getBezierPath from ReactFlowComponents
  const { getBezierPath } = ReactFlowComponents;
  // Use ReactFlowComponents to get nodes from the store
  // (ReactFlow v11 context: nodes are available via window.ReactFlow.store or via props)
  // We'll assume nodes are available via window.ReactFlow.store.getState().nodes
  const nodes = window.ReactFlow.store.getState().nodes;
  const sourceNode = nodes.find((n) => n.id === source);
  const targetNode = nodes.find((n) => n.id === target);

  if (!sourceNode || !targetNode) {
    console.warn('CustomEdge: source or target node not found', { id, source, target, nodes });
    return null;
  }

  // Find parent containers for both source and target nodes
  const sourceParent = sourceNode.parentNode ? nodes.find((n) => n.id === sourceNode.parentNode) : null;
  const targetParent = targetNode.parentNode ? nodes.find((n) => n.id === targetNode.parentNode) : null;

  // Calculate absolute positions for source and target nodes
  // If a node is in a container, its position is relative, so we add the parent's position.
  const sourceAbsX = (sourceParent ? sourceParent.positionAbsolute.x : 0) + sourceNode.position.x;
  const sourceAbsY = (sourceParent ? sourceParent.positionAbsolute.y : 0) + sourceNode.position.y;
  const targetAbsX = (targetParent ? targetParent.positionAbsolute.x : 0) + targetNode.position.x;
  const targetAbsY = (targetParent ? targetParent.positionAbsolute.y : 0) + targetNode.position.y;

  // We also need to account for the node's dimensions to connect to the center of the sides.
  const sourceX = sourceAbsX + (sourceNode.width / 2);
  const sourceY = sourceAbsY + (sourceNode.height / 2);
  const targetX = targetAbsX + (targetNode.width / 2);
  const targetY = targetAbsY + (targetNode.height / 2);

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
