/**
 * @fileoverview GroupingControls Component
 * 
 * Provides controls for selecting different hierarchical groupings.
 */

import React from 'react';
import { GroupingOption } from './types';
import { COMPONENT_COLORS } from '../shared/config';

export interface GroupingControlsProps {
  hierarchyChoices?: GroupingOption[];
  currentGrouping?: string | null;
  onGroupingChange?: (groupingId: string) => void;
  compact?: boolean;
  disabled?: boolean;
  className?: string;
  style?: React.CSSProperties;
}

export function GroupingControls({
  hierarchyChoices = [],
  currentGrouping,
  onGroupingChange,
  compact = false,
  disabled = false,
  className = '',
  style
}: GroupingControlsProps) {
  
  if (!hierarchyChoices || hierarchyChoices.length === 0) {
    return (
      <div className={`grouping-controls-empty ${className}`} style={style}>
        <span style={{ 
          color: COMPONENT_COLORS.TEXT_DISABLED,
          fontSize: compact ? '9px' : '10px',
          fontStyle: 'italic'
        }}>
          No grouping options available
        </span>
      </div>
    );
  }

  if (hierarchyChoices.length === 1) {
    return (
      <div className={`grouping-controls-single ${className}`} style={style}>
        <span style={{ 
          color: COMPONENT_COLORS.TEXT_PRIMARY,
          fontSize: compact ? '9px' : '10px',
          fontWeight: 'bold'
        }}>
          Grouping: {hierarchyChoices[0].name}
        </span>
      </div>
    );
  }

  const handleChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    if (!disabled && onGroupingChange) {
      onGroupingChange(event.target.value);
    }
  };

  const selectStyle: React.CSSProperties = {
    fontSize: compact ? '9px' : '10px',
    padding: compact ? '2px 4px' : '4px 6px',
    border: `1px solid ${COMPONENT_COLORS.BORDER_MEDIUM}`,
    borderRadius: '3px',
    backgroundColor: disabled ? COMPONENT_COLORS.BACKGROUND_SECONDARY : COMPONENT_COLORS.BACKGROUND_PRIMARY,
    color: disabled ? COMPONENT_COLORS.TEXT_DISABLED : COMPONENT_COLORS.TEXT_PRIMARY,
    cursor: disabled ? 'not-allowed' : 'pointer',
    width: '100%',
    maxWidth: compact ? '120px' : '180px',
  };

  const labelStyle: React.CSSProperties = {
    fontSize: compact ? '9px' : '10px',
    fontWeight: 'bold',
    color: COMPONENT_COLORS.TEXT_PRIMARY,
    marginBottom: '4px',
    display: 'block',
  };

  return (
    <div className={`grouping-controls ${className}`} style={style}>
      {!compact && (
        <label style={labelStyle}>
          Grouping:
        </label>
      )}
      
      <select
        value={currentGrouping || ''}
        onChange={handleChange}
        disabled={disabled}
        style={selectStyle}
        title={disabled ? 'Grouping controls are disabled' : 'Select a grouping method'}
      >
        {!currentGrouping && (
          <option value="" disabled>
            Select grouping...
          </option>
        )}
        {hierarchyChoices.map(choice => (
          <option key={choice.id} value={choice.id}>
            {choice.name}
          </option>
        ))}
      </select>
    </div>
  );
}
