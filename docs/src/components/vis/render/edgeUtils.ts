/**
 * @fileoverview Shared edge utilities
 * 
 * Common utilities for edge rendering to reduce duplication.
 */

import { getBezierPath } from '@xyflow/react';
import { getEdgeColor, getEdgeStrokeWidth, getEdgeDashPattern } from '../shared/config';

/**
 * Shared bezier path calculation
 */
export function calculateBezierPath(params: {
  sourceX: number;
  sourceY: number;
  sourcePosition: any;
  targetX: number;
  targetY: number;
  targetPosition: any;
}) {
  return getBezierPath(params);
}

/**
 * Calculate common edge style
 */
export function calculateEdgeStyle(edge: any, selected: boolean, isHighlighted: boolean, customStyle: any = {}) {
  const strokeColor = getEdgeColor(edge?.style, selected, isHighlighted);
  const strokeWidth = customStyle?.strokeWidth || getEdgeStrokeWidth(edge?.style);
  
  return {
    strokeWidth,
    stroke: strokeColor,
    strokeDasharray: getEdgeDashPattern(edge?.style),
    ...customStyle
  };
}

/**
 * Get common edge path props
 */
export function getEdgePathProps(id: string, edgePath: string, edgeStyle: any, edge: any, selected: boolean) {
  return {
    id,
    style: edgeStyle,
    className: `react-flow__edge-path ${edge?.style || 'default'} ${selected ? 'selected' : ''}`,
    d: edgePath,
    fill: "none",
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const
  };
}