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
import { createEdgeEventHandlers } from './eventHandlers';
import { calculateBezierPath, calculateEdgeStyle, getEdgePathProps } from './edgeUtils';

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
  selected,
  markerEnd,
  markerStart
}) => {
  const edge = data?.edge;
  
  // Use shared event handlers
  const eventHandlers = createEdgeEventHandlers(id, data);

  // Use shared path calculation and styling
  const [edgePath] = calculateBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition
  });

  const edgeStyle = calculateEdgeStyle(edge, selected, data?.isHighlighted || false, style);
  const pathProps = getEdgePathProps(id, edgePath, edgeStyle, edge, selected);

  return (
    <path
      {...pathProps}
      onClick={eventHandlers.handleClick}
      onContextMenu={eventHandlers.handleContextMenu}
      markerEnd={markerEnd}
      markerStart={markerStart}
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
  selected,
  markerEnd,
  markerStart
}) => {
  const hyperEdge = data?.hyperEdge;
  
  // Use shared event handlers
  const eventHandlers = createEdgeEventHandlers(id, data);

  // Use shared path calculation (HyperEdge needs labelX/Y)
  const [edgePath, labelX, labelY] = calculateBezierPath({
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
        onClick={eventHandlers.handleClick}
        onContextMenu={eventHandlers.handleContextMenu}
        fill="none"
        strokeLinecap="round"
        strokeLinejoin="round"
        markerEnd={markerEnd}
        markerStart={markerStart}
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


