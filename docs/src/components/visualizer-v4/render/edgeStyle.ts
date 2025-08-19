/**
 * @fileoverview Shared helpers for edge styling and flags.
 */

import type { EdgeProps } from '@xyflow/react';

type StyleCfg = {
  edgeColor?: string;
  edgeWidth?: number;
  edgeDashed?: boolean;
};

type StrokeDefaults = {
  color?: string;
  width?: number;
  dash?: string;
};

/**
 * Compute stroke/style values with sensible fallbacks.
 * Precedence: explicit style -> provided defaults -> config -> hardcoded fallback.
 */
export function getStroke(styleCfg: StyleCfg, style: any, defaults: StrokeDefaults = {}) {
  const stroke = (style?.stroke ?? styleCfg.edgeColor ?? defaults.color ?? '#1976d2') as string;
  const strokeWidth = (style?.strokeWidth ?? styleCfg.edgeWidth ?? defaults.width ?? 2) as number;
  // Priority: explicit style -> explicit default dash -> cfg-based dash -> undefined
  const strokeDasharray = (style?.strokeDasharray ?? defaults.dash ?? (styleCfg.edgeDashed ? '6,6' : undefined)) as string | undefined;
  return { stroke, strokeWidth, strokeDasharray } as const;
}

/** Extract optional halo color. */
export function getHaloColor(style: any): string | undefined {
  return style?.haloColor as string | undefined;
}

/** Return style object without haloColor property. */
export function stripHaloStyle(style: any): any {
  if (!style) return style;
  const { haloColor, ...rest } = style as Record<string, any>;
  return rest;
}

/** Whether this edge should render as a double line. */
export function isDoubleLineEdge(props: EdgeProps): boolean {
  return (
    (props as any).data?.processedStyle?.lineStyle === 'double' ||
    (props.style as any)?.lineStyle === 'double'
  );
}

/** Whether this edge should render with a wavy path. */
export function isWavyEdge(props: EdgeProps): boolean {
  const style: any = props.style as any;
  return Boolean(style?.filter?.includes('edge-wavy') || (props as any).data?.processedStyle?.waviness || style?.waviness);
}
