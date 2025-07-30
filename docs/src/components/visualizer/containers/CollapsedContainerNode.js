/**
 * Collapsed Container Node Component
 * 
 * Displays a collapsed container as a compact node with expand functionality
 */

import React from 'react';
import { Handle } from '@xyflow/react';

export function CollapsedContainerNode(props) {
  const { data, width, height, id } = props;
  
  // Use the width/height from props (ReactFlow passes these) or fall back to data
  const effectiveWidth = width || data?.originalDimensions?.width || 180;
  const effectiveHeight = height || 60;
  
  // Extract colors from the original style or use defaults
  const originalStyle = data?.nodeStyle || {};
  let backgroundColor = originalStyle.background || 'rgba(59, 130, 246, 0.25)';
  let borderColor = 'rgb(59, 130, 246)';
  let textColor = 'rgb(59, 130, 246)';
  
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
    background: 'rgba(0,0,0,0.6)',
    color: 'white',
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
      
      {/* Add connection handles for edges */}
      <Handle 
        type="source" 
        position="right" 
        id="source-right"
        style={{ 
          background: textColor, 
          border: `2px solid ${backgroundColor}`, 
          width: 10, 
          height: 10,
          right: -5 
        }} 
      />
      <Handle 
        type="target" 
        position="left" 
        id="target-left"
        style={{ 
          background: textColor, 
          border: `2px solid ${backgroundColor}`, 
          width: 10, 
          height: 10,
          left: -5 
        }} 
      />
      <Handle 
        type="source" 
        position="bottom" 
        id="source-bottom"
        style={{ 
          background: textColor, 
          border: `2px solid ${backgroundColor}`, 
          width: 10, 
          height: 10,
          bottom: -5 
        }} 
      />
      <Handle 
        type="target" 
        position="top" 
        id="target-top"
        style={{ 
          background: textColor, 
          border: `2px solid ${backgroundColor}`, 
          width: 10, 
          height: 10,
          top: -5 
        }} 
      />
    </div>
  );
}
