/**
 * @fileoverview Standard Edge Component
 *
 * ReactFlow edge component for standard edges using bridge-based styling.
 */

import React from 'react';
import { BaseEdge, EdgeProps, getStraightPath, getBezierPath, getSmoothStepPath } from '@xyflow/react';
import { useStyleConfig } from './StyleConfigContext';
import { getWavyPath } from './edgePaths';
import { getStroke, getHaloColor, stripHaloStyle, isDoubleLineEdge, isWavyEdge } from './edgeStyle';

/**
 * Standard graph edge component - uses ReactFlow's automatic routing
 * Includes all styling properties from FloatingEdge for consistency
 */
export function StandardEdge(props: EdgeProps) {
  const styleCfg = useStyleConfig();

  // Check if this edge should be wavy (based on filter or direct style)
  const isWavy = isWavyEdge(props);

  let edgePath: string;
  
  if (isWavy) {
    // Use custom wavy path generation
    edgePath = getWavyPath({
      sourceX: props.sourceX,
      sourceY: props.sourceY,
      targetX: props.targetX,
      targetY: props.targetY,
      amplitude: 8, // Moderate wave amplitude
      frequency: 2  // 2 complete waves along the path
    });
  } else if (styleCfg.edgeStyle === 'straight') {
    [edgePath] = getStraightPath({
      sourceX: props.sourceX,
      sourceY: props.sourceY,
      targetX: props.targetX,
      targetY: props.targetY,
    });
  } else if (styleCfg.edgeStyle === 'smoothstep') {
    [edgePath] = getSmoothStepPath({
      sourceX: props.sourceX,
      sourceY: props.sourceY,
      targetX: props.targetX,
      targetY: props.targetY,
      sourcePosition: props.sourcePosition,
      targetPosition: props.targetPosition,
    });
  } else {
    [edgePath] = getBezierPath({
      sourceX: props.sourceX,
      sourceY: props.sourceY,
      targetX: props.targetX,
      targetY: props.targetY,
      sourcePosition: props.sourcePosition,
      targetPosition: props.targetPosition,
    });
  }

  const { stroke, strokeWidth, strokeDasharray } = getStroke(styleCfg, props.style);
  const isDouble = isDoubleLineEdge(props);
  const haloColor = getHaloColor(props.style as any);

  // Use simple rendering for regular edges (no halo, no double line)
  if (!isDouble && !haloColor) {
    return (
      <BaseEdge
        path={edgePath}
        markerEnd={props.markerEnd}
        style={{ stroke, strokeWidth, strokeDasharray, ...props.style }}
      />
    );
  }

  // Use complex rendering for edges with halos or double lines
  return (
    <g>
      {/* Render halo layer if haloColor is specified */}
      {haloColor && (
        <BaseEdge
          path={edgePath}
          markerEnd={undefined}
          style={{ 
            stroke: haloColor, 
            strokeWidth: strokeWidth + 4, 
            strokeDasharray, 
            strokeLinecap: 'round',
            opacity: 0.6
          }}
        />
      )}
      
      {/* Render main edge - always render this */}
      <BaseEdge
        path={edgePath}
        markerEnd={props.markerEnd}
  style={{ stroke, strokeWidth, strokeDasharray, ...(stripHaloStyle(props.style)) }}
      />
      
      {/* Render additional rails for double lines */}
      {isDouble && (
        <>
          <BaseEdge
            path={edgePath}
            markerEnd={undefined}
            style={{ stroke, strokeWidth, strokeDasharray, transform: `translate(0, 2px)` }}
          />
          <BaseEdge
            path={edgePath}
            markerEnd={undefined}
            style={{ stroke, strokeWidth, strokeDasharray, transform: `translate(0, -2px)` }}
          />
        </>
      )}
    </g>
  );
}

// Internal memoized variant for use in edgeTypes to reduce re-renders
export const MemoStandardEdge = React.memo(StandardEdge);
