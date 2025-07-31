/**
 * Collapsed Container Node Component
 * 
 * Displays a collapsed container as a compact node with expand functionality
 */

import React from 'react';
import { Handle } from '@xyflow/react';
import { COLORS, DEFAULT_STYLES } from '../utils/constants.js';

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
  
  // Reusable handle style function
  const getHandleStyle = (position) => ({
    background: textColor, 
    border: `${DEFAULT_STYLES.BORDER_WIDTH} solid ${backgroundColor}`, 
    width: 10, 
    height: 10,
    [position]: -5 
  });
  
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
      
      {/* Add connection handles for edges */}
      <Handle 
        type="source" 
        position="right" 
        id="source-right"
        style={getHandleStyle('right')}
      />
      <Handle 
        type="target" 
        position="left" 
        id="target-left"
        style={getHandleStyle('left')}
      />
      <Handle 
        type="source" 
        position="bottom" 
        id="source-bottom"
        style={getHandleStyle('bottom')}
      />
      <Handle 
        type="target" 
        position="top" 
        id="target-top"
        style={getHandleStyle('top')}
      />
    </div>
  );
}
