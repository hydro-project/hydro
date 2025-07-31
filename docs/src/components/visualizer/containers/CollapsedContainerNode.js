/**
 * Collapsed Container Node Component
 * 
 * Displays a collapsed container as a compact node with expand functionality
 */

import React from 'react';
import { Handle } from '@xyflow/react';
import { REQUIRED_HANDLE_IDS } from '../utils/handleValidation.js';
import { truncateContainerName } from '../utils/utils.js';
import { COLORS, COMPONENT_COLORS } from '../utils/constants.js';
import { getContainerHandles } from '../utils/handleStyles.js';

export function CollapsedContainerNode(props) {
  const { data, width, height, id } = props;
  
  // Use the width/height from props (ReactFlow passes these) or fall back to data
  const effectiveWidth = width || data?.originalDimensions?.width || 180;
  const effectiveHeight = height || 60;
  
  // Truncate the container label for display
  const fullLabel = data?.label || 'Container';
  const displayLabel = truncateContainerName(fullLabel, 15, {
    side: 'left',
    splitOnDelimiter: true,
    delimiterPenalty: 0.2
  });
  const showTooltip = fullLabel !== displayLabel;
  
  // Extract colors from the original style or use defaults
  const originalStyle = data?.nodeStyle || {};
  let backgroundColor = originalStyle.background || COLORS.CONTAINER_L0;
  let borderColor = COLORS.CONTAINER_BORDER_L0;
  let textColor = COLORS.CONTAINER_BORDER_L0;
  
  // Parse border to get border color if available
  if (originalStyle.border) {
    const borderMatch = originalStyle.border.match(/solid\s+(rgb\([^)]+\)|#[a-fA-F0-9]+|\w+)/);
    if (borderMatch) {
      borderColor = borderMatch[1];
      textColor = borderMatch[1];
    }
  }
  
  const containerStyle = {
    width: effectiveWidth,
    height: effectiveHeight,
    background: backgroundColor,
    border: `2px solid ${borderColor}`,
    borderRadius: '8px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    fontSize: '13px',
    fontWeight: '600',
    color: textColor,
    cursor: 'pointer',
    boxSizing: 'border-box',
    position: 'relative',
    transition: 'all 0.2s ease',
    boxShadow: '0 2px 4px rgba(0,0,0,0.1)',
  };
  
  const expandIconStyle = {
    position: 'absolute',
    top: '4px',
    right: '4px',
    width: '16px',
    height: '16px',
    background: textColor,
    color: COMPONENT_COLORS.TEXT_INVERSE,
    borderRadius: '50%',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    fontSize: '12px',
    fontWeight: 'bold',
  };
  
  const labelStyle = {
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
    paddingRight: '24px', // Make room for expand icon
    textAlign: 'center',
    width: '100%',
  };

  const nodeCountStyle = {
    position: 'absolute',
    bottom: '2px',
    left: '4px',
    background: 'rgba(0,0,0,0.7)',
    color: COMPONENT_COLORS.TEXT_INVERSE,
    fontSize: '10px',
    padding: '2px 4px',
    borderRadius: '10px',
    fontWeight: 'bold',
  };
  
  // Count how many nodes would be inside this container
  const nodeCount = data?.nodeCount || '?';
  
  return (
    <div style={containerStyle}>
      <div style={labelStyle} title={showTooltip ? fullLabel : undefined}>
        {displayLabel}
      </div>
      <div style={expandIconStyle} title="Click to expand">
        +
      </div>
      <div style={nodeCountStyle} title="Number of nodes inside">
        {nodeCount}
      </div>
      
      {/* 
        CRITICAL: Connection handles for ReactFlow edges
        Using centralized handle configuration for consistency
      */}
      {getContainerHandles().map(handleProps => (
        <Handle key={handleProps.id} {...handleProps} />
      ))}
    </div>
  );
}
