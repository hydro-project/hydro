/**
 * @fileoverview Bridge-Based Node Components
 * 
 * ReactFlow node components for container and standard nodes
 */

import React from 'react';
import type { NodeProps } from '@xyflow/react';

/**
 * Standard graph node component
 */
export function StandardNode({ id, data }: NodeProps) {
  return (
    <div
      style={{
        padding: '12px 16px',
        background: '#e3f2fd',
        border: '1px solid #1976d2',
        borderRadius: '4px',
        fontSize: '12px',
        textAlign: 'center',
        minWidth: '120px',
        boxShadow: '0 2px 4px rgba(0,0,0,0.1)'
      }}
    >
      {String(data.label || id)}
    </div>
  );
}

/**
 * Container node component
 */
export function ContainerNode({ id, data }: NodeProps) {
  return (
    <div
      style={{
        padding: '16px',
        background: data.collapsed ? '#ffeb3b' : 'rgba(25, 118, 210, 0.1)',
        border: data.collapsed ? '2px solid #f57f17' : '2px solid #1976d2',
        borderRadius: '8px',
        fontSize: '12px',
        textAlign: 'center',
        minWidth: '180px',
        minHeight: data.collapsed ? '60px' : '120px'
      }}
    >
      <strong>{String(data.label || id)}</strong>
      {data.collapsed && (
        <div style={{ fontSize: '10px', color: '#666', marginTop: '4px' }}>
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