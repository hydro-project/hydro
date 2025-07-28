/**
 * Custom Group Node Component for ReactFlow
 * 
 * Displays group nodes with labels for hierarchy containers
 */

import React from 'react';

export function GroupNode({ data, style }) {
  return (
    <div 
      style={{
        ...style,
        display: 'flex',
        alignItems: 'flex-start',
        justifyContent: 'flex-start',
        padding: '12px',
        position: 'relative',
        minWidth: '200px',
        minHeight: '100px',
      }}
    >
      <div 
        style={{
          position: 'absolute',
          top: '8px',
          left: '12px',
          fontSize: '14px',
          fontWeight: 'bold',
          color: style?.color || '#000',
          backgroundColor: 'rgba(255, 255, 255, 0.9)',
          padding: '4px 8px',
          borderRadius: '4px',
          border: `1px solid ${style?.color || '#000'}`,
          zIndex: 10,
        }}
      >
        {data?.label || 'Container'}
      </div>
    </div>
  );
}
