/**
 * @fileoverview Container graph node component
 */

import React from 'react';
import { type NodeProps } from '@xyflow/react';
import { truncateLabel } from '../shared/textUtils';
import { useStyleConfig } from './StyleConfigContext';
import { HandlesRenderer } from './handles';

export function ContainerNode({ id, data }: NodeProps) {
  const styleCfg = useStyleConfig();
  const width = data.width || 180;
  const height = data.height || (data.collapsed ? 100 : 120);

  const colorPalette = String(data.colorPalette || 'Set3');
  const nodeCount = Number(data.nodeCount || 0);
  const containerLabel = String(data.label || id);

  const generateContainerColors = (containerId: string, palette: string) => {
    const hash = containerId.split('').reduce((a, b) => a + b.charCodeAt(0), 0);
    const colorPalettes: Record<string, string[]> = {
      Set3: ['#8dd3c7', '#ffffb3', '#bebada', '#fb8072', '#80b1d3', '#fdb462', '#b3de69'],
      Pastel1: ['#fbb4ae', '#b3cde3', '#ccebc5', '#decbe4', '#fed9a6', '#ffffcc', '#e5d8bd'],
      Dark2: ['#1b9e77', '#d95f02', '#7570b3', '#e7298a', '#66a61e', '#e6ab02', '#a6761d'],
      Set1: ['#e41a1c', '#377eb8', '#4daf4a', '#984ea3', '#ff7f00', '#ffff33', '#a65628'],
      Set2: ['#66c2a5', '#fc8d62', '#8da0cb', '#e78ac3', '#a6d854', '#ffd92f', '#e5c494']
    };

    const colors = colorPalettes[palette] || colorPalettes['Set3'];
    const baseColor = colors[hash % colors.length];

    const lighten = (color: string, factor: number) => {
      const hex = color.replace('#', '');
      const r = parseInt(hex.substring(0, 2), 16);
      const g = parseInt(hex.substring(2, 4), 16);
      const b = parseInt(hex.substring(4, 6), 16);
      const newR = Math.floor(r + (255 - r) * factor);
      const newG = Math.floor(g + (255 - g) * factor);
      const newB = Math.floor(b + (255 - b) * factor);
      return `#${newR.toString(16).padStart(2, '0')}${newG.toString(16).padStart(2, '0')}${newB.toString(16).padStart(2, '0')}`;
    };

    const darken = (color: string, factor: number) => {
      const hex = color.replace('#', '');
      const r = parseInt(hex.substring(0, 2), 16);
      const g = parseInt(hex.substring(2, 4), 16);
      const b = parseInt(hex.substring(4, 6), 16);
      const newR = Math.floor(r * (1 - factor));
      const newG = Math.floor(g * (1 - factor));
      const newB = Math.floor(b * (1 - factor));
      return `#${newR.toString(16).padStart(2, '0')}${newG.toString(16).padStart(2, '0')}${newB.toString(16).padStart(2, '0')}`;
    };

    return { background: lighten(baseColor, 0.8), border: darken(baseColor, 0.2), text: darken(baseColor, 0.4) };
  };

  if (data.collapsed) {
    const containerColors = generateContainerColors(id, colorPalette);
    return (
      <div
        style={{
          width: `${width}px`,
          height: `${height}px`,
          background: containerColors.background,
          border: `${styleCfg.containerBorderWidth ?? 2}px solid ${containerColors.border}`,
          borderRadius: `${styleCfg.containerBorderRadius ?? 8}px`,
          position: 'relative',
          boxSizing: 'border-box',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          cursor: 'pointer',
          boxShadow:
            styleCfg.containerShadow === 'NONE'
              ? 'none'
              : styleCfg.containerShadow === 'LARGE'
              ? '0 10px 15px -3px rgba(0,0,0,0.2)'
              : styleCfg.containerShadow === 'MEDIUM'
              ? '0 4px 6px -1px rgba(0,0,0,0.15)'
              : '0 2px 8px rgba(0,0,0,0.15)',
          transition: 'all 0.2s ease'
        }}
      >
        <HandlesRenderer />
        <div
          style={{
            fontSize: '13px',
            fontWeight: '600',
            color: containerColors.text,
            textAlign: 'center',
            maxWidth: `${Number(width) - 16}px`,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            marginBottom: '4px',
            cursor: 'pointer'
          }}
        >
          {truncateLabel(containerLabel, { maxLength: Math.floor((Number(width) - 16) / 8), preferDelimiters: true, leftTruncate: true })}
        </div>
        <div style={{ fontSize: '11px', color: containerColors.text, opacity: 0.8, textAlign: 'center' }}>
          {nodeCount} node{nodeCount !== 1 ? 's' : ''}
        </div>
      </div>
    );
  }

  return (
    <div
      style={{
        padding: `${Math.max((styleCfg.nodePadding ?? 12) + 4, 8)}px`,
        background: 'rgba(25, 118, 210, 0.1)',
        border: `${styleCfg.containerBorderWidth ?? 2}px solid #1976d2`,
        borderRadius: `${styleCfg.containerBorderRadius ?? 8}px`,
        width: `${width}px`,
        height: `${height}px`,
        position: 'relative',
        boxSizing: 'border-box',
        cursor: 'pointer'
      }}
    >
      <HandlesRenderer />
      <div
        style={{
          position: 'absolute',
          bottom: '0px',
          right: '12px',
          fontSize: '12px',
          fontWeight: 'bold',
          color: '#1976d2',
          maxWidth: `${Number(width) - 36}px`,
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
          cursor: 'pointer',
          textShadow:
            '1px 1px 2px rgba(255, 255, 255, 0.8), -1px -1px 2px rgba(255, 255, 255, 0.8), 1px -1px 2px rgba(255, 255, 255, 0.8), -1px 1px 2px rgba(255, 255, 255, 0.8)',
          filter: 'drop-shadow(0px 1px 2px rgba(0, 0, 0, 0.1))'
        }}
      >
        {truncateLabel(containerLabel, { maxLength: Math.floor((Number(width) - 36) / 8), preferDelimiters: true, leftTruncate: false })}
      </div>
    </div>
  );
}

// Memoized variant to avoid unnecessary re-renders when props are unchanged
export const MemoContainerNode = React.memo(ContainerNode);

export default ContainerNode;
