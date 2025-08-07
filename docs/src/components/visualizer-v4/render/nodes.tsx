/**
 * @fileoverview Bridge-Based Node Components
 * 
 * ReactFlow node components with configurable handle system for maximum layout flexibility
 */

import React from 'react';
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { getHandleConfig, CONTINUOUS_HANDLE_STYLE } from './handleConfig';
import { generateNodeColors } from '../shared/colorUtils';

/**
 * Render handles based on current configuration
 */
function renderHandles() {
  const config = getHandleConfig();
  
  if (config.enableContinuousHandles) {
    // ReactFlow v12 continuous handles - connections anywhere on perimeter
    return (
      <>
        <Handle
          type="source"
          position={Position.Top}
          style={CONTINUOUS_HANDLE_STYLE}
          isConnectable={true}
        />
        <Handle
          type="target"
          position={Position.Top}
          style={CONTINUOUS_HANDLE_STYLE}
          isConnectable={true}
        />
      </>
    );
  }
  
  // Discrete handles if configured
  return (
    <>
      {config.sourceHandles.map(handle => (
        <Handle
          key={handle.id}
          id={handle.id}
          type="source"
          position={handle.position}
          style={handle.style}
          isConnectable={true}
        />
      ))}
      {config.targetHandles.map(handle => (
        <Handle
          key={handle.id}
          id={handle.id}
          type="target"
          position={handle.position}
          style={handle.style}
          isConnectable={true}
        />
      ))}
    </>
  );
}

/**
 * Standard graph node component
 */
export function StandardNode({ id, data }: NodeProps) {
  // Get dynamic colors based on node type (preferred) or style as fallback
  const nodeType = String(data.nodeType || data.style || 'default');
  const colorPalette = String(data.colorPalette || 'Set3');
  const colors = generateNodeColors([nodeType], colorPalette);
  
  return (
    <div
      style={{
        padding: '12px 16px',
        background: colors.primary,
        border: `1px solid ${colors.border}`,
        borderRadius: '4px',
        fontSize: '12px',
        textAlign: 'center',
        minWidth: '120px',
        boxShadow: '0 2px 4px rgba(0,0,0,0.1)',
        position: 'relative'
      }}
    >
      {renderHandles()}
      {String(data.label || id)}
    </div>
  );
}

/**
 * Container node component with label positioned at bottom-right
 */
export function ContainerNode({ id, data }: NodeProps) {
  // Use dimensions from ELK layout via ReactFlowBridge data
  const width = data.width || 180; // fallback to default
  const height = data.height || (data.collapsed ? 60 : 120); // fallback to default
  
  // Debug: Log container dimensions
  console.log(`[ContainerNode] 📏 Container ${id}: data.width=${data.width}, data.height=${data.height}, using ${width}x${height}`);
  
  return (
    <div
      style={{
        padding: '16px',
        background: data.collapsed ? '#ffeb3b' : 'rgba(25, 118, 210, 0.1)',
        border: data.collapsed ? '2px solid #f57f17' : '2px solid #1976d2',
        borderRadius: '8px',
        width: `${width}px`,  // Use ELK-calculated width
        height: `${height}px`, // Use ELK-calculated height (now includes label space)
        position: 'relative',
        boxSizing: 'border-box' // Ensure padding is included in dimensions
      }}
    >
      {renderHandles()}
      
      {/* Container label positioned at bottom-right */}
      <div
        style={{
          position: 'absolute',
          bottom: '0px',  // Decreased from 12px to give more space from internal nodes
          right: '12px',   // Keep horizontal spacing the same
          fontSize: '12px',
          fontWeight: 'bold',
          color: data.collapsed ? '#f57f17' : '#1976d2',
          maxWidth: `${Number(width) - 36}px`, // Ensure label doesn't overflow container (increased padding)
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
          // Text shadow for better legibility over container background
          textShadow: '1px 1px 2px rgba(255, 255, 255, 0.8), -1px -1px 2px rgba(255, 255, 255, 0.8), 1px -1px 2px rgba(255, 255, 255, 0.8), -1px 1px 2px rgba(255, 255, 255, 0.8)',
          // Subtle drop shadow for the text element itself
          filter: 'drop-shadow(0px 1px 2px rgba(0, 0, 0, 0.1))'
        }}
      >
        {String(data.label || id)}
      </div>
      
      {/* Collapsed indicator (if needed) */}
      {data.collapsed && (
        <div style={{ 
          position: 'absolute',
          top: '8px',
          left: '8px',
          fontSize: '10px', 
          color: '#666',
          fontWeight: '500',
          // Text shadow for legibility
          textShadow: '1px 1px 1px rgba(255, 255, 255, 0.8), -1px -1px 1px rgba(255, 255, 255, 0.8)',
          filter: 'drop-shadow(0px 1px 1px rgba(0, 0, 0, 0.1))'
        }}>
          (collapsed)
        </div>
      )}
    </div>
  );
}

// Export map for ReactFlow nodeTypes
export const nodeTypes = {
  standard: StandardNode,
  container: ContainerNode
};