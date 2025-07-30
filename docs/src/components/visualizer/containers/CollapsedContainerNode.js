/**
 * Collapsed Container Node Component
 * 
 * Displays a collapsed container as a compact node with expand functionality
 */

import React from 'react';
import { Handle } from '@xyflow/react';
import { REQUIRED_HANDLE_IDS } from '../utils/handleValidation.js';

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
      
      {/* 
        CRITICAL: Connection handles for ReactFlow edges
        
        These Handle IDs MUST match exactly with:
        1. GroupNode.js handles 
        2. Handle IDs used in containerLogic.js edge processing
        3. Any other node types that can be edge targets
        
        DO NOT CHANGE these IDs without updating all related components!
        This ensures seamless edge connections when expanding/collapsing containers.
      */}
      <Handle 
        type="source" 
        position="right" 
        id={REQUIRED_HANDLE_IDS.source} // CRITICAL: Must match GroupNode and edge processing
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
        id={REQUIRED_HANDLE_IDS.target} // CRITICAL: Must match GroupNode and edge processing
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
        id={REQUIRED_HANDLE_IDS.sourceBottom} // CRITICAL: Must match GroupNode
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
        id={REQUIRED_HANDLE_IDS.targetTop} // CRITICAL: Must match GroupNode
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
