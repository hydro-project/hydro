/**
 * Collapsed Container Node Component
 * 
 * Displays a collapsed container as a compact node with expand functionality
 */

import React from 'react';
import { Handle } from '@xyflow/react';
import { COLORS, DEFAULT_STYLES } from '../utils/constants.js';
import { getContainerHandles } from '../utils/handleStyles.js';

export function CollapsedContainerNode(props) {
  const { data, width, height, id } = props;
  
  // Use the width/height from props (ReactFlow passes these) or fall back to data
  const effectiveWidth = width || data?.originalDimensions?.width || 180;
  const effectiveHeight = height || 60;
  
  // Extract colors from the original style or use defaults
  const originalStyle = data?.nodeStyle || {};
  let backgroundColor = originalStyle.background || COLORS.DEFAULT_GRAY_ALPHA;
  let borderColor = COLORS.DEFAULT_GRAY;
  let textColor = COLORS.DEFAULT_GRAY;
  
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
    border: `${DEFAULT_STYLES.BORDER_WIDTH} solid ${borderColor}`,
    borderRadius: DEFAULT_STYLES.BORDER_RADIUS,
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
    boxShadow: DEFAULT_STYLES.BOX_SHADOW,
  };
  
  const expandIconStyle = {
    position: 'absolute',
    top: '4px',
    right: '4px',
    width: '16px',
    height: '16px',
    background: textColor,
    color: 'white',
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
    background: COLORS.BLACK_SEMI_ALPHA,
    color: COLORS.WHITE,
    fontSize: '10px',
    padding: '2px 4px',
    borderRadius: '10px',
    fontWeight: 'bold',
  };
  
  // Count how many nodes would be inside this container
  const nodeCount = data?.nodeCount || '?';
  
  return (
    <div style={containerStyle}>
      <div style={labelStyle}>
        {data?.label || 'Container'}
      </div>
      <div style={expandIconStyle} title="Click to expand">
        +
      </div>
      <div style={nodeCountStyle} title="Number of nodes inside">
        {nodeCount}
      </div>
      
      {/* 
        CRITICAL: Connection handles for ReactFlow edges
        Using centralized handle configuration for consistency with GroupNode
      */}
      {getContainerHandles().map(handleProps => (
        <Handle key={handleProps.id} {...handleProps} />
      ))}
    </div>
  );
}
