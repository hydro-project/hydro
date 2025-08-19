/**
 * @fileoverview Bridge-Based Edge Components
 * 
 * ReactFlow edge components for standard and hyper edges
 */

import React from 'react';
import { BaseEdge, EdgeProps, getStraightPath, getBezierPath, getSmoothStepPath } from '@xyflow/react';
import FloatingEdge from './FloatingEdge';
import { useStyleConfig } from './StyleConfigContext';

/**
 * Generates a wavy (sinusoidal) SVG path between two points
 */
function getWavyPath({
  sourceX,
  sourceY,
  targetX,
  targetY,
  amplitude = 10,
  frequency = 6
}: {
  sourceX: number;
  sourceY: number;
  targetX: number;
  targetY: number;
  amplitude?: number;
  frequency?: number;
}): string {
  const deltaX = targetX - sourceX;
  const deltaY = targetY - sourceY;
  const distance = Math.sqrt(deltaX * deltaX + deltaY * deltaY);
  
  if (distance === 0) {
    return `M ${sourceX} ${sourceY}`;
  }

  // Calculate the angle of the line
  const angle = Math.atan2(deltaY, deltaX);
  
  // Number of segments for smooth curve
  const segments = Math.max(20, Math.floor(distance / 5));
  
  let path = `M ${sourceX} ${sourceY}`;
  
  for (let i = 1; i <= segments; i++) {
    const t = i / segments;
    
    // Base position along the straight line
    const baseX = sourceX + deltaX * t;
    const baseY = sourceY + deltaY * t;
    
    // Calculate wave offset perpendicular to the line
    // Use actual distance traveled for consistent wave density
    const distanceTraveled = distance * t;
    const waveOffset = amplitude * Math.sin(frequency * Math.PI * distanceTraveled / 50);
    
    // Apply perpendicular offset
    const offsetX = -waveOffset * Math.sin(angle);
    const offsetY = waveOffset * Math.cos(angle);
    
    const x = baseX + offsetX;
    const y = baseY + offsetY;
    
    path += ` L ${x} ${y}`;
  }
  
  return path;
}

/**
 * Standard graph edge component - uses ReactFlow's automatic routing
 * Includes all styling properties from FloatingEdge for consistency
 */
export function StandardEdge(props: EdgeProps) {
  const styleCfg = useStyleConfig();

  // Check if this edge should be wavy (based on filter or direct style)
  const isWavy = (props.style as any)?.filter?.includes('edge-wavy') || 
                 (props as any).data?.processedStyle?.waviness ||
                 (props.style as any)?.waviness;

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

  const stroke = (props.style as any)?.stroke || styleCfg.edgeColor || '#1976d2';
  const strokeWidth = (props.style as any)?.strokeWidth ?? styleCfg.edgeWidth ?? 2;
  const strokeDasharray = (props.style as any)?.strokeDasharray || (styleCfg.edgeDashed ? '6,6' : undefined);
  const isDouble = (props as any).data?.processedStyle?.lineStyle === 'double' || (props.style as any)?.lineStyle === 'double';
  const haloColor = (props.style as any)?.haloColor;

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
        style={{ stroke, strokeWidth, strokeDasharray, ...(props.style && Object.fromEntries(Object.entries(props.style).filter(([key]) => key !== 'haloColor'))) }}
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

/**
 * HyperEdge component
 */
export function HyperEdge(props: EdgeProps) {
  const styleCfg = useStyleConfig();

  // Check if this edge should be wavy (based on filter or direct style)
  const isWavy = (props.style as any)?.filter?.includes('edge-wavy') || 
                 (props as any).data?.processedStyle?.waviness ||
                 (props.style as any)?.waviness;

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

  const stroke = styleCfg.edgeColor || '#ff5722';
  const strokeWidth = styleCfg.edgeWidth ?? 3;
  const strokeDasharray = styleCfg.edgeDashed ? '5,5' : '5,5';
  const haloColor = (props.style as any)?.haloColor;

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
          ...(props.style && Object.fromEntries(Object.entries(props.style).filter(([key]) => key !== 'haloColor')))
        }}
      />
    </g>
  );
}

// Export map for ReactFlow edgeTypes
export const edgeTypes = {
  standard: StandardEdge,
  hyper: HyperEdge,
  floating: FloatingEdge
};
