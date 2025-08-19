/**
 * @fileoverview Hyper Edge Component
 *
 * ReactFlow edge component for hyper edges using bridge-based styling.
 */

import React from 'react';
import { BaseEdge, EdgeProps, getStraightPath } from '@xyflow/react';
import { useStyleConfig } from './StyleConfigContext';
import { getWavyPath } from './edgePaths';
import { getStroke, getHaloColor, stripHaloStyle, isWavyEdge } from './edgeStyle';

/**
 * HyperEdge component
 */
export function HyperEdge(props: EdgeProps) {
  const styleCfg = useStyleConfig();

  // Check if this edge should be wavy (based on filter or direct style)
  const isWavy = isWavyEdge(props);

  let edgePath: string;
  
  if (isWavy) {
    // Use custom wavy path generation for hyper edges
    edgePath = getWavyPath({
      sourceX: props.sourceX,
      sourceY: props.sourceY,
      targetX: props.targetX,
      targetY: props.targetY,
      amplitude: 6, // Slightly smaller amplitude for hyper edges
      frequency: 2.5  // More frequent waves for hyper edges
    });
  } else {
    [edgePath] = getStraightPath({
      sourceX: props.sourceX,
      sourceY: props.sourceY,
      targetX: props.targetX,
      targetY: props.targetY,
    });
  }

  // Use defaults suited for hyper edges
  const { stroke, strokeWidth, strokeDasharray } = getStroke(
    { ...styleCfg, edgeDashed: true },
    props.style,
    { color: '#ff5722', width: 3, dash: '5,5' }
  );
  const haloColor = getHaloColor(props.style as any);

  // Simple rendering for edges without halos
  if (!haloColor) {
    return (
      <BaseEdge
        path={edgePath}
        markerEnd={props.markerEnd}
        style={{ 
          stroke, 
          strokeWidth, 
          strokeDasharray,
          ...props.style
        }}
      />
    );
  }

  // Complex rendering for edges with halos
  return (
    <g>
      {/* Render halo layer */}
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
      
      {/* Render main edge */}
      <BaseEdge
        path={edgePath}
        markerEnd={props.markerEnd}
        style={{ 
          stroke, 
          strokeWidth, 
          strokeDasharray,
          ...(stripHaloStyle(props.style))
        }}
      />
    </g>
  );
}

// Internal memoized variant for use in edgeTypes to reduce re-renders
export const MemoHyperEdge = React.memo(HyperEdge);
