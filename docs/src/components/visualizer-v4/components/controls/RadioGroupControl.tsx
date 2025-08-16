/**
 * @fileoverview Radio Button Group Control Component
 * 
 * Reusable radio button group component for configuration panel selections.
 */

import React from 'react';
import { COMPONENT_COLORS, TYPOGRAPHY } from '../../shared/config';

export interface RadioOption {
  value: string;
  label: string;
  description?: string;
}

export interface RadioGroupControlProps {
  label: string;
  value: string;
  options: RadioOption[];
  onChange: (value: string) => void;
  disabled?: boolean;
  layout?: 'vertical' | 'horizontal';
  className?: string;
  style?: React.CSSProperties;
}

export function RadioGroupControl({
  label,
  value,
  options,
  onChange,
  disabled = false,
  layout = 'vertical',
  className = '',
  style = {}
}: RadioGroupControlProps) {
  const name = `radio-group-${label.toLowerCase().replace(/\s+/g, '-')}`;
  
  return (
    <div 
      className={`radio-group-control ${className}`}
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '8px',
        ...style
      }}
    >
      <label
        style={{
          fontSize: TYPOGRAPHY.UI_SMALL,
          fontWeight: 'bold',
          color: COMPONENT_COLORS.TEXT_PRIMARY,
          marginBottom: '2px'
        }}
      >
        {label}
      </label>
      
      <div
        style={{
          display: 'flex',
          flexDirection: layout === 'vertical' ? 'column' : 'row',
          gap: layout === 'vertical' ? '6px' : '12px',
          flexWrap: layout === 'horizontal' ? 'wrap' : undefined
        }}
      >
        {options.map((option) => (
          <label
            key={option.value}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              fontSize: TYPOGRAPHY.UI_SMALL,
              color: disabled ? COMPONENT_COLORS.TEXT_DISABLED : COMPONENT_COLORS.TEXT_PRIMARY,
              cursor: disabled ? 'not-allowed' : 'pointer',
              padding: '2px 0'
            }}
            title={option.description}
          >
            <input
              type="radio"
              name={name}
              value={option.value}
              checked={value === option.value}
              onChange={(e) => onChange(e.target.value)}
              disabled={disabled}
              style={{
                margin: 0,
                cursor: disabled ? 'not-allowed' : 'pointer'
              }}
            />
            
            <span
              style={{
                userSelect: 'none'
              }}
            >
              {option.label}
            </span>
          </label>
        ))}
      </div>
    </div>
  );
}