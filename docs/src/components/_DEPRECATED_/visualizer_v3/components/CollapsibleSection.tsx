/**
 * @fileoverview CollapsibleSection Component
 * 
 * A reusable collapsible section component for organizing panel content.
 */

import React, { useState } from 'react';
import { CollapsibleSectionProps } from './types';
import { COMPONENT_COLORS } from '../shared/config';

export function CollapsibleSection({
  title,
  isCollapsed,
  onToggle,
  children,
  level = 0,
  showIcon = true,
  disabled = false,
  className = '',
  style
}: CollapsibleSectionProps) {
  const handleClick = () => {
    if (!disabled) {
      onToggle();
    }
  };

  const sectionStyle: React.CSSProperties = {
    marginBottom: '12px',
    ...style
  };

  const headerStyle: React.CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    cursor: disabled ? 'default' : 'pointer',
    fontSize: '11px',
    fontWeight: 'bold',
    marginBottom: isCollapsed ? '0' : '6px',
    color: disabled ? COMPONENT_COLORS.TEXT_DISABLED : COMPONENT_COLORS.TEXT_PRIMARY,
    paddingLeft: `${level * 8}px`,
    padding: '4px 0',
    borderRadius: '2px',
    transition: 'background-color 0.15s ease',
  };

  const contentStyle: React.CSSProperties = {
    paddingLeft: '12px',
    paddingTop: '4px',
  };

  return (
    <div className={`collapsible-section ${className}`} style={sectionStyle}>
      <div 
        style={headerStyle}
        onClick={handleClick}
        onMouseEnter={(e) => {
          if (!disabled) {
            e.currentTarget.style.backgroundColor = COMPONENT_COLORS.BUTTON_HOVER_BACKGROUND;
          }
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.backgroundColor = 'transparent';
        }}
        title={disabled ? undefined : `${isCollapsed ? 'Expand' : 'Collapse'} ${title}`}
      >
        {showIcon && (
          <span style={{ 
            marginRight: '6px', 
            fontSize: '10px',
            color: disabled ? COMPONENT_COLORS.TEXT_DISABLED : COMPONENT_COLORS.TEXT_SECONDARY,
            transition: 'transform 0.15s ease',
            transform: isCollapsed ? 'rotate(0deg)' : 'rotate(90deg)'
          }}>
            ▶
          </span>
        )}
        <span>{title}</span>
      </div>
      
      {!isCollapsed && (
        <div style={contentStyle}>
          {children}
        </div>
      )}
    </div>
  );
}
