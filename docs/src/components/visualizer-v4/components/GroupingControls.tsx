/**
 * @fileoverview GroupingControls Component
 * 
 * Provides controls for selecting different hierarchical groupings.
 */

import React from 'react';
import { Select } from 'antd';
import { GroupingOption } from './types';
import { COMPONENT_COLORS, TYPOGRAPHY } from '../shared/config';

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
          fontSize: compact ? TYPOGRAPHY.UI_SMALL : TYPOGRAPHY.UI_MEDIUM,
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
          fontSize: compact ? TYPOGRAPHY.UI_SMALL : TYPOGRAPHY.UI_MEDIUM,
          fontWeight: 'bold'
        }}>
          Grouping: {hierarchyChoices[0].name}
        </span>
      </div>
    );
  }

  const handleChange = (value: string) => {
    if (!disabled && onGroupingChange && value) {
      onGroupingChange(value);
    }
  };

  const selectStyle: React.CSSProperties = {
    fontSize: compact ? TYPOGRAPHY.UI_SMALL : TYPOGRAPHY.UI_MEDIUM,
    width: '100%',
    maxWidth: compact ? '120px' : '180px',
  };

  const labelStyle: React.CSSProperties = {
    fontSize: compact ? TYPOGRAPHY.UI_SMALL : TYPOGRAPHY.UI_MEDIUM,
    fontWeight: 'bold',
    color: COMPONENT_COLORS.TEXT_PRIMARY,
    marginBottom: '4px',
    display: 'block',
  };

  // Create options for Ant Design Select
  const selectOptions = hierarchyChoices.map(choice => ({
    value: choice.id,
    label: choice.name,
  }));

  return (
    <div className={`grouping-controls ${className}`} style={style}>
      {!compact && (
        <label style={labelStyle}>
          Grouping:
        </label>
      )}
      
      <Select
        value={currentGrouping || undefined}
        onChange={handleChange}
        disabled={disabled}
        placeholder="Select grouping..."
        options={selectOptions}
        size="small"
        style={selectStyle}
        styles={{
          popup: {
            root: {
              fontSize: compact ? '9px' : '10px',
            }
          }
        }}
        popupMatchSelectWidth={true}
      />
    </div>
  );
}
