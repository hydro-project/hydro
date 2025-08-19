/**
 * @fileoverview CollapsibleSection Component
 *
 * Refactored to use native <details>/<summary> for accessibility and simplicity,
 * while keeping a controlled open state via props.
 */

import React from 'react';
import { CollapsibleSectionProps } from './types';
import { COMPONENT_COLORS, TYPOGRAPHY } from '../shared/config';

export function CollapsibleSection({
  title,
  isCollapsed,
  onToggle,
  children,
  level = 0,
  showIcon = true,
  disabled = false,
  className = '',
  style,
}: CollapsibleSectionProps) {
  const isOpen = !isCollapsed;
  const indent = level * 8;

  return (
    <details
      open={isOpen}
      className={`collapsible-section ${className}`}
      style={{ marginBottom: '12px', ...style }}
    >
      {/* Scoped style to hide default markers so we can render our own caret */}
      <style>{`
        details > summary.collapsible-summary { list-style: none; }
        details > summary.collapsible-summary::-webkit-details-marker { display: none; }
      `}</style>

      <summary
        className="collapsible-summary"
        aria-expanded={isOpen}
        aria-disabled={disabled}
        title={disabled ? undefined : `${isCollapsed ? 'Expand' : 'Collapse'} ${title}`}
        onClick={(e) => {
          // Prevent the browser from toggling <details>; we control via props
          e.preventDefault();
          if (!disabled) {
            onToggle();
          }
        }}
        style={{
          display: 'flex',
          alignItems: 'center',
          cursor: disabled ? 'default' : 'pointer',
          fontSize: TYPOGRAPHY.UI_MEDIUM,
          fontWeight: 'bold',
          color: disabled ? COMPONENT_COLORS.TEXT_DISABLED : COMPONENT_COLORS.TEXT_PRIMARY,
          paddingLeft: `${indent}px`,
          padding: '4px 0',
          userSelect: 'none',
        }}
      >
        {showIcon && (
          <span
            aria-hidden="true"
            style={{
              marginRight: '6px',
              fontSize: TYPOGRAPHY.UI_MEDIUM,
              color: disabled ? COMPONENT_COLORS.TEXT_DISABLED : COMPONENT_COLORS.TEXT_SECONDARY,
              transition: 'transform 0.15s ease',
              transform: isOpen ? 'rotate(90deg)' : 'rotate(0deg)',
              display: 'inline-block',
            }}
          >
            ▶
          </span>
        )}
        <span>{title}</span>
      </summary>

      <div style={{ paddingLeft: `${indent + 12}px`, paddingTop: '4px' }}>{children}</div>
    </details>
  );
}
