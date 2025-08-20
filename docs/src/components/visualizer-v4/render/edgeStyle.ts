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
export function getStroke(styleCfg: StyleCfg, style: unknown, defaults: StrokeDefaults = {}) {
  const s = (style ?? {}) as Record<string, unknown>;
  const stroke = (s.stroke ?? styleCfg.edgeColor ?? defaults.color ?? '#1976d2') as string;
  const strokeWidth = (s.strokeWidth ?? styleCfg.edgeWidth ?? defaults.width ?? 2) as number;
  // Priority: explicit style -> explicit default dash -> cfg-based dash -> undefined
  const strokeDasharray = (s.strokeDasharray ?? defaults.dash ?? (styleCfg.edgeDashed ? '6,6' : undefined)) as string | undefined;
  return { stroke, strokeWidth, strokeDasharray } as const;
}

/** Extract optional halo color. */
export function getHaloColor(style: unknown): string | undefined {
  const s = style as Record<string, unknown> | undefined;
  return (s?.haloColor as string | undefined);
}

/** Return style object without haloColor property. */
export function stripHaloStyle<T extends Record<string, unknown> | undefined>(style: T): T {
  if (!style) return style;
  const { haloColor: _haloColor, ...rest } = style as Record<string, unknown>;
  return rest as T;
}

/** Whether this edge should render as a double line. */
export function isDoubleLineEdge(props: EdgeProps): boolean {
  return (
  (props as unknown as { data?: { processedStyle?: { lineStyle?: string } } }).data?.processedStyle?.lineStyle === 'double' ||
  (props.style as Record<string, unknown> | undefined)?.lineStyle === 'double'
  );
}

/** Whether this edge should render with a wavy path. */
export function isWavyEdge(props: EdgeProps): boolean {
  const style = props.style as Record<string, unknown> | undefined;
  const data = props as unknown as { data?: { processedStyle?: { waviness?: unknown } } };
  const filter = style?.filter as string | undefined;
  return Boolean(filter?.includes('edge-wavy') || data.data?.processedStyle?.waviness || style?.waviness);
}
