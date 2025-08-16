/**
 * @fileoverview Dropdown Control Component
 * 
 * Reusable dropdown component for configuration panel selections.
 */

import React from 'react';
import { COMPONENT_COLORS, TYPOGRAPHY } from '../../shared/config';

export interface DropdownOption {
  value: string;
  label: string;
  description?: string;
}

export interface DropdownControlProps {
  label: string;
  value: string;
  options: DropdownOption[];
  onChange: (value: string) => void;
  disabled?: boolean;
  className?: string;
  style?: React.CSSProperties;
}

export function DropdownControl({
  label,
  value,
  options,
  onChange,
  disabled = false,
  className = '',
  style = {}
}: DropdownControlProps) {
  return (
    <div 
      className={`dropdown-control ${className}`}
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '4px',
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
      
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        style={{
          padding: '6px 8px',
          fontSize: TYPOGRAPHY.UI_SMALL,
          border: `1px solid ${COMPONENT_COLORS.BORDER_LIGHT}`,
          borderRadius: '4px',
          backgroundColor: disabled ? COMPONENT_COLORS.BACKGROUND_SECONDARY : COMPONENT_COLORS.BACKGROUND_PRIMARY,
          color: disabled ? COMPONENT_COLORS.TEXT_DISABLED : COMPONENT_COLORS.TEXT_PRIMARY,
          cursor: disabled ? 'not-allowed' : 'pointer'
        }}
        onMouseEnter={(e) => {
          if (!disabled) {
            e.currentTarget.style.borderColor = COMPONENT_COLORS.BORDER_MEDIUM;
          }
        }}
        onMouseLeave={(e) => {
          if (!disabled) {
            e.currentTarget.style.borderColor = COMPONENT_COLORS.BORDER_LIGHT;
          }
        }}
      >
        {options.map((option) => (
          <option 
            key={option.value} 
            value={option.value}
            title={option.description}
          >
            {option.label}
          </option>
        ))}
      </select>
    </div>
  );
}