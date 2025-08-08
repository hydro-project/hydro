/**
 * @fileoverview Legend Component
 * 
 * Displays a color-coded legend for different node types.
 */

import React from 'react';
import { LegendProps } from './types';
import { generateNodeColors } from '../shared/colorUtils';
import { COLOR_PALETTES, COMPONENT_COLORS } from '../shared/config';

export function Legend({
  legendData,
  colorPalette = 'Set3',
  nodeTypeConfig,
  title,
  compact = false,
  className = '',
  style
}: LegendProps) {
  const displayTitle = title || legendData.title || 'Legend';
  const paletteKey = (colorPalette in COLOR_PALETTES) ? colorPalette as keyof typeof COLOR_PALETTES : 'Set3';

  const legendStyle: React.CSSProperties = {
    fontSize: compact ? '9px' : '10px',
    ...style
  };

  const itemStyle: React.CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    margin: compact ? '2px 0' : '3px 0',
    fontSize: compact ? '9px' : '10px'
  };

  const colorBoxStyle = (colors: any): React.CSSProperties => ({
    width: compact ? '10px' : '12px',
    height: compact ? '10px' : '12px',
    borderRadius: '2px',
    marginRight: compact ? '4px' : '6px',
    border: `1px solid ${COMPONENT_COLORS.BORDER_MEDIUM}`,
    flexShrink: 0,
    backgroundColor: colors.primary,
    borderColor: colors.border
  });

  return (
    <div className={`legend ${className}`} style={legendStyle}>
      {!compact && displayTitle && (
        <div style={{
          fontWeight: 'bold',
          marginBottom: '6px',
          color: COMPONENT_COLORS.TEXT_PRIMARY,
          fontSize: '11px'
        }}>
          {displayTitle}
        </div>
      )}
      
      {legendData.items.map(item => {
        const colors = generateNodeColors([item.type], paletteKey, nodeTypeConfig);
        return (
          <div 
            key={item.type} 
            style={itemStyle}
            title={item.description || `${item.label} nodes`}
          >
            <div style={colorBoxStyle(colors)} />
            <span style={{ color: COMPONENT_COLORS.TEXT_PRIMARY }}>
              {item.label}
            </span>
          </div>
        );
      })}
    </div>
  );
}
