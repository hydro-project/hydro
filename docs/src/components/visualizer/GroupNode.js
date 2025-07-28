/**
 * Custom Group Node Component for ReactFlow
 * 
 * Displays group nodes with labels for hierarchy containers
 */

import React from 'react';

export function GroupNode(props) {
  const { data, style, width, height } = props;
  
  // Use ReactFlow's width/height props if style is missing
  const effectiveWidth = style?.width || width;
  const effectiveHeight = style?.height || height;
  
  if (!effectiveWidth || !effectiveHeight) {
    console.warn('[GroupNode] Missing dimensions:', { style, width, height, data });
    return null;
  }

  // Reconstruct style from ReactFlow props
  const effectiveStyle = style || {
    width: effectiveWidth,
    height: effectiveHeight,
    background: 'rgba(59, 130, 246, 0.25)',
    border: '3px solid rgb(59, 130, 246)',
    borderRadius: '8px',
    padding: '12px',
    fontSize: '14px',
    fontWeight: 'bold',
    color: 'rgb(59, 130, 246)',
    zIndex: 1,
  };
  
  return (
    <div 
      style={{
        ...effectiveStyle,
        width: effectiveWidth,
        height: effectiveHeight,
        display: 'flex',
        alignItems: 'flex-end',
        justifyContent: 'flex-end',
        position: 'relative',
        minWidth: '200px',
        minHeight: '100px',
      }}
    >
      <div 
        style={{
          position: 'absolute',
          bottom: '8px',
          right: '12px',
          fontSize: '14px',
          fontWeight: 'bold',
          color: effectiveStyle?.color || '#000',
          backgroundColor: 'rgba(255, 255, 255, 0.9)',
          padding: '4px 8px',
          borderRadius: '4px',
          border: `1px solid ${effectiveStyle?.color || '#000'}`,
          zIndex: 10,
        }}
      >
        {data?.label || 'Container'}
      </div>
    </div>
  );
}
