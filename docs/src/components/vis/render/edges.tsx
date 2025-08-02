/**
 * @fileoverview Custom ReactFlow Edge Components
 * 
 * Custom edge components for rendering graph connections.
 */

import React from 'react';
import { getBezierPath } from '@xyflow/react';
import { 
  getEdgeColor, 
  getEdgeStrokeWidth, 
  getEdgeDashPattern,
  EDGE_COLORS,
  SIZES
} from '../shared/config';
import { TypedEdgeProps } from './types';

// Standard Edge Component
export const GraphStandardEdge: React.FC<TypedEdgeProps> = ({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  style = {},
  data,
  selected
}) => {
  const edge = data?.edge;
  
  const handleClick = (event: React.MouseEvent) => {
    event.stopPropagation();
    if (data?.onEdgeClick) {
      data.onEdgeClick(id);
    }
  };

  const handleContextMenu = (event: React.MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
    if (data?.onEdgeContextMenu) {
      data.onEdgeContextMenu(id, event);
    }
  };

  // Calculate bezier path
  const [edgePath] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition
  });

  // Get edge style based on type
  const strokeColor = getEdgeColor(edge?.style, selected, data?.isHighlighted || false);
  const strokeWidth = style?.strokeWidth || getEdgeStrokeWidth(edge?.style);
  
  const edgeStyle = {
    strokeWidth,
    stroke: strokeColor,
    strokeDasharray: getEdgeDashPattern(edge?.style),
    ...style
  };

  return (
    <path
      id={id}
      style={edgeStyle}
      className={`react-flow__edge-path ${edge?.style || 'default'} ${selected ? 'selected' : ''}`}
      d={edgePath}
      onClick={handleClick}
      onContextMenu={handleContextMenu}
      fill="none"
      strokeLinecap="round"
      strokeLinejoin="round"
    />
  );
};

// Hyper Edge Component (for aggregated edges)
export const GraphHyperEdge: React.FC<TypedEdgeProps> = ({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  style = {},
  data,
  selected
}) => {
  const hyperEdge = data?.hyperEdge;
  
  const handleClick = (event: React.MouseEvent) => {
    event.stopPropagation();
    if (data?.onEdgeClick) {
      data.onEdgeClick(id);
    }
  };

  const handleContextMenu = (event: React.MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
    if (data?.onEdgeContextMenu) {
      data.onEdgeContextMenu(id, event);
    }
  };

  // Calculate bezier path
  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition
  });

  // Hyper edge styling with gradient
  const edgeStyle = {
    strokeWidth: style?.strokeWidth || 2,
    stroke: 'url(#hyperEdgeGradient)',
    filter: 'drop-shadow(0 1px 2px rgba(147, 51, 234, 0.3))',
    ...style
  };

  const aggregatedCount = hyperEdge?.aggregatedEdges?.length || 1;

  return (
    <>
      {/* Define gradient for hyper edges */}
      <defs>
        <linearGradient id="hyperEdgeGradient" x1="0%" y1="0%" x2="100%" y2="0%">
          <stop offset="0%" stopColor="#9333ea" stopOpacity="0.8" />
          <stop offset="50%" stopColor="#c084fc" stopOpacity="0.9" />
          <stop offset="100%" stopColor="#9333ea" stopOpacity="0.8" />
        </linearGradient>
      </defs>
      
      <path
        id={id}
        style={edgeStyle}
        className={`react-flow__edge-path hyper-edge ${selected ? 'selected' : ''}`}
        d={edgePath}
        onClick={handleClick}
        onContextMenu={handleContextMenu}
        fill="none"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      
      {/* Edge label showing aggregated count */}
      {aggregatedCount > 1 && (
        <text
          x={labelX}
          y={labelY}
          className="hyper-edge-label"
          style={{
            fontSize: '10px',
            fontWeight: 'bold',
            fill: '#9333ea',
            textAnchor: 'middle',
            dominantBaseline: 'middle',
            background: 'white',
            padding: '2px 4px',
            borderRadius: '4px',
            pointerEvents: 'none'
          }}
        >
          {aggregatedCount}
        </text>
      )}
    </>
  );
};


