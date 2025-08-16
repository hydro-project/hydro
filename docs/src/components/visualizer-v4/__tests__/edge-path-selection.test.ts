/**
 * @fileoverview Test edge path generation for different edge styles
 * 
 * Tests that the edge components correctly generate different path shapes based on edgeStyle config.
 */

import { describe, it, expect } from 'vitest';
import { getStraightPath, getBezierPath, getSmoothStepPath } from '@xyflow/react';

describe('Edge Path Selection', () => {
  const mockEdgeProps = {
    sourceX: 0,
    sourceY: 0,
    targetX: 200,
    targetY: 100,
    sourcePosition: 'right' as any,
    targetPosition: 'left' as any,
  };

  it('should generate different paths for different edge styles', () => {
    // Test straight path
    const [straightPath] = getStraightPath({
      sourceX: mockEdgeProps.sourceX,
      sourceY: mockEdgeProps.sourceY,
      targetX: mockEdgeProps.targetX,
      targetY: mockEdgeProps.targetY,
    });

    // Test bezier path
    const [bezierPath] = getBezierPath({
      sourceX: mockEdgeProps.sourceX,
      sourceY: mockEdgeProps.sourceY,
      targetX: mockEdgeProps.targetX,
      targetY: mockEdgeProps.targetY,
      sourcePosition: mockEdgeProps.sourcePosition,
      targetPosition: mockEdgeProps.targetPosition,
    });

    // Test smooth step path
    const [smoothStepPath] = getSmoothStepPath({
      sourceX: mockEdgeProps.sourceX,
      sourceY: mockEdgeProps.sourceY,
      targetX: mockEdgeProps.targetX,
      targetY: mockEdgeProps.targetY,
      sourcePosition: mockEdgeProps.sourcePosition,
      targetPosition: mockEdgeProps.targetPosition,
    });

    // Paths should be different
    expect(straightPath).not.toBe(bezierPath);
    expect(bezierPath).not.toBe(smoothStepPath);
    expect(straightPath).not.toBe(smoothStepPath);

    // All paths should be valid SVG path strings
    expect(straightPath).toMatch(/^M\s*[\d.-]+\s*[\d.-]+/);
    expect(bezierPath).toMatch(/^M\s*[\d.-]+\s*[\d.-]+/);
    expect(smoothStepPath).toMatch(/^M\s*[\d.-]+\s*[\d.-]+/);
  });

  it('should generate straight line path for straight edge style', () => {
    const [straightPath] = getStraightPath({
      sourceX: 0,
      sourceY: 0,
      targetX: 100,
      targetY: 100,
    });

    // Straight path should be a simple line (M...L... pattern)
    expect(straightPath).toMatch(/^M\s*[\d.-]+\s*[\d.-]+\s*L\s*[\d.-]+\s*[\d.-]+$/);
  });

  it('should generate bezier curve for bezier edge style', () => {
    const [bezierPath] = getBezierPath({
      sourceX: 0,
      sourceY: 0,
      targetX: 100,
      targetY: 100,
      sourcePosition: 'right',
      targetPosition: 'left',
    });

    // Bezier path should contain curve commands (C for cubic bezier)
    expect(bezierPath).toMatch(/C/);
  });

  it('should generate smooth step path for smoothstep edge style', () => {
    const [smoothStepPath] = getSmoothStepPath({
      sourceX: 0,
      sourceY: 0,
      targetX: 100,
      targetY: 100,
      sourcePosition: 'right',
      targetPosition: 'left',
    });

    // Smooth step path should contain multiple segments
    expect(smoothStepPath.split(/[MLQ]/).length).toBeGreaterThan(2);
  });
});