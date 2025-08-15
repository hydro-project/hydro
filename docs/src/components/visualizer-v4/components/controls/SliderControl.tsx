/**
 * @fileoverview Slider Control Component
 * 
 * Reusable slider component for numeric configuration values.
 */

import React from 'react';
import { COMPONENT_COLORS, TYPOGRAPHY } from '../../shared/config';

export interface SliderControlProps {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (value: number) => void;
  disabled?: boolean;
  formatValue?: (value: number) => string;
  className?: string;
  style?: React.CSSProperties;
}

export function SliderControl({
  label,
  value,
  min,
  max,
  step = 0.1,
  onChange,
  disabled = false,
  formatValue = (val) => val.toFixed(1),
  className = '',
  style = {}
}: SliderControlProps) {
  return (
    <div 
      className={`slider-control ${className}`}
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '4px',
        ...style
      }}
    >
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center'
        }}
      >
        <label
          style={{
            fontSize: TYPOGRAPHY.UI_SMALL,
            fontWeight: 'bold',
            color: COMPONENT_COLORS.TEXT_PRIMARY
          }}
        >
          {label}
        </label>
        
        <span
          style={{
            fontSize: TYPOGRAPHY.UI_SMALL,
            color: COMPONENT_COLORS.TEXT_SECONDARY,
            fontFamily: 'monospace',
            backgroundColor: COMPONENT_COLORS.BACKGROUND_SECONDARY,
            padding: '2px 6px',
            borderRadius: '3px',
            minWidth: '40px',
            textAlign: 'center'
          }}
        >
          {formatValue(value)}
        </span>
      </div>
      
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(parseFloat(e.target.value))}
        disabled={disabled}
        style={{
          width: '100%',
          height: '6px',
          borderRadius: '3px',
          background: disabled 
            ? COMPONENT_COLORS.BORDER_LIGHT 
            : `linear-gradient(to right, ${COMPONENT_COLORS.BORDER_MEDIUM} 0%, ${COMPONENT_COLORS.BORDER_MEDIUM} ${((value - min) / (max - min)) * 100}%, ${COMPONENT_COLORS.BORDER_LIGHT} ${((value - min) / (max - min)) * 100}%, ${COMPONENT_COLORS.BORDER_LIGHT} 100%)`,
          outline: 'none',
          cursor: disabled ? 'not-allowed' : 'pointer',
          appearance: 'none',
          WebkitAppearance: 'none'
        }}
      />
      
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          fontSize: TYPOGRAPHY.UI_SMALL,
          color: COMPONENT_COLORS.TEXT_TERTIARY
        }}
      >
        <span>{formatValue(min)}</span>
        <span>{formatValue(max)}</span>
      </div>
    </div>
  );
}