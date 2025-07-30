/**
 * Custom Group Node Component for ReactFlow
 * 
 * Displays group nodes with labels for hierarchy containers
 * Avoids ReactFlow's built-in group styling that causes shadows
 */

import React from 'react';
import { COLORS } from '../utils/constants.js';

export function GroupNode(props) {
  // In ReactFlow v12, custom components receive: id, data, width, height
  // No style prop! Get styling from data.nodeStyle
  const { data, width, height, id } = props;
  
  // Get style from data.nodeStyle where we stored it
  const nodeStyle = data?.nodeStyle || {};
  const effectiveWidth = width || nodeStyle.width || 300;
  const effectiveHeight = height || nodeStyle.height || 200;
  
  // Debug: Log what we're actually receiving (less frequently)
  if (Math.random() < 0.01) { // Back to 1% to reduce spam
    console.log(`[GroupNode] DEBUG ${id}:`, { 
      hasNodeStyle: !!data?.nodeStyle,
      nodeWidth: effectiveWidth, 
      nodeHeight: effectiveHeight,
      sequence: data?.sequence
    });
  }
  
  if (!effectiveWidth || !effectiveHeight || !data) {
    console.warn('[GroupNode] Missing required props:', { style: !!style, width, height, data, nodeId: id });
    // Return a simple fallback instead of null to see what's happening
    return (
      <div style={{ 
        width: 200, 
        height: 100, 
        background: 'red', 
        border: '2px solid black',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        color: 'white',
        fontWeight: 'bold'
      }}>
        ERROR: Missing Props
      </div>
    );
  }

  // Use the style object directly from our processed nodes, with fallbacks
  const containerStyle = {
    width: effectiveWidth,
    height: effectiveHeight,
    // Use hardcoded styles based on node ID since ReactFlow isn't passing them through
    background: getBackgroundColor(id),
    border: getBorderColor(id),
    borderRadius: '8px',
    // Remove padding to test if this is causing the inset
    fontSize: '14px',
    fontWeight: 'bold',
    color: getTextColor(id),
    zIndex: 1,
    boxSizing: 'border-box',
    position: 'relative',
    display: 'flex',
    alignItems: 'flex-end',
    justifyContent: 'flex-end',
    minWidth: '200px',
    minHeight: '100px',
  };

  // Helper functions to get colors based on node ID
  function getBackgroundColor(nodeId) {
    if (nodeId === 'cloud') return 'rgba(59, 130, 246, 0.25)';
    if (nodeId === 'region') return 'rgba(16, 185, 129, 0.25)';
    if (nodeId?.startsWith('az')) return 'rgba(245, 158, 11, 0.25)';
    return 'rgba(59, 130, 246, 0.25)'; // default
  }

  function getBorderColor(nodeId) {
    if (nodeId === 'cloud') return `3px solid ${COLORS.DEFAULT_BLUE}`;
    if (nodeId === 'region') return `3px solid ${COLORS.DEFAULT_GREEN}`;
    if (nodeId?.startsWith('az')) return `3px solid ${COLORS.DEFAULT_ORANGE}`;
    return `3px solid ${COLORS.DEFAULT_BLUE}`; // default
  }

  function getTextColor(nodeId) {
    if (nodeId === 'cloud') return COLORS.DEFAULT_BLUE;
    if (nodeId === 'region') return COLORS.DEFAULT_GREEN;
    if (nodeId?.startsWith('az')) return COLORS.DEFAULT_ORANGE;
    return COLORS.DEFAULT_BLUE; // default
  }
  
  return (
    <div style={containerStyle}>
      <div 
        style={{
          position: 'absolute',
          bottom: '4px',
          right: '4px',
          fontSize: '14px',
          fontWeight: 'bold',
          color: getTextColor(id),
          backgroundColor: COLORS.WHITE_ALPHA,
          padding: '4px 8px',
          borderRadius: '4px',
          border: `1px solid ${getTextColor(id)}`,
          zIndex: 10,
        }}
      >
        {data?.label || 'Container'}
      </div>
      <div 
        style={{
          position: 'absolute',
          top: '4px',
          right: '4px',
          width: '16px',
          height: '16px',
          background: getTextColor(id),
          color: 'white',
          borderRadius: '50%',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          fontSize: '12px',
          fontWeight: 'bold',
          cursor: 'pointer',
          zIndex: 10,
        }}
        title="Click to collapse"
      >
        −
      </div>
    </div>
  );
}
