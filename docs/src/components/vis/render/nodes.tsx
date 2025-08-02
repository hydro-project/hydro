/**
 * @fileoverview Custom ReactFlow Node Components
 * 
 * Custom node components for rendering graph elements.
 */

import React from 'react';
import { Handle, Position, NodeProps } from 'reactflow';

// Standard Node Component
export const GraphStandardNode: React.FC<NodeProps> = ({ 
  data, 
  selected, 
  id 
}) => {
  const node = data?.node || { label: id, style: 'default' };
  
  const handleClick = (event: React.MouseEvent) => {
    event.stopPropagation();
    if (data?.onNodeClick) {
      data.onNodeClick(id);
    }
  };

  const handleDoubleClick = (event: React.MouseEvent) => {
    event.stopPropagation();
    if (data?.onNodeDoubleClick) {
      data.onNodeDoubleClick(id);
    }
  };

  const handleContextMenu = (event: React.MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
    if (data?.onNodeContextMenu) {
      data.onNodeContextMenu(id, event);
    }
  };

  return (
    <div 
      className={`graph-standard-node ${node.style} ${selected ? 'selected' : ''} ${data?.isHighlighted ? 'highlighted' : ''}`}
      onClick={handleClick}
      onDoubleClick={handleDoubleClick}
      onContextMenu={handleContextMenu}
      style={{
        width: node.width || 120,
        height: node.height || 60,
        padding: '8px',
        background: 'white',
        border: `2px solid ${getNodeBorderColor(node.style, selected, data?.isHighlighted)}`,
        borderRadius: '8px',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        fontSize: '12px',
        fontWeight: 'bold',
        color: getNodeTextColor(node.style),
        boxShadow: selected ? '0 0 10px rgba(0,123,255,0.5)' : '0 2px 4px rgba(0,0,0,0.1)',
        cursor: 'pointer',
        transition: 'all 0.2s ease-in-out'
      }}
    >
      {/* Connection handles */}
      <Handle
        type="target"
        position={Position.Top}
        style={{ background: '#555' }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        style={{ background: '#555' }}
      />
      
      {/* Node content */}
      <div style={{ textAlign: 'center', overflow: 'hidden', textOverflow: 'ellipsis' }}>
        {node.label}
      </div>
    </div>
  );
};

// Container Node Component
export const GraphContainerNode: React.FC<NodeProps> = ({ 
  data, 
  selected, 
  id 
}) => {
  const container = data?.container || { width: 200, height: 150, collapsed: false };
  const isCollapsed = data?.isCollapsed || container.collapsed;
  
  const handleClick = (event: React.MouseEvent) => {
    event.stopPropagation();
    if (data?.onContainerClick) {
      data.onContainerClick(id);
    }
  };

  const handleToggleCollapse = (event: React.MouseEvent) => {
    event.stopPropagation();
    if (data?.onToggleCollapse) {
      data.onToggleCollapse(id);
    }
  };

  return (
    <div 
      className={`graph-container-node ${selected ? 'selected' : ''}`}
      onClick={handleClick}
      style={{
        width: container.width,
        height: isCollapsed ? 40 : container.height,
        background: 'rgba(240, 242, 247, 0.8)',
        border: `2px solid ${selected ? '#007bff' : '#d0d7de'}`,
        borderRadius: '12px',
        position: 'relative',
        cursor: 'pointer',
        transition: 'all 0.3s ease-in-out'
      }}
    >
      {/* Container header */}
      <div 
        className="container-header"
        onClick={handleToggleCollapse}
        style={{
          position: 'absolute',
          top: 0,
          left: 0,
          right: 0,
          height: '32px',
          background: 'rgba(100, 116, 139, 0.1)',
          borderRadius: '10px 10px 0 0',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '0 12px',
          fontSize: '11px',
          fontWeight: 'bold',
          color: '#374151',
          borderBottom: isCollapsed ? 'none' : '1px solid #d0d7de'
        }}
      >
        <span>Container {id}</span>
        <span style={{ 
          transform: isCollapsed ? 'rotate(0deg)' : 'rotate(90deg)',
          transition: 'transform 0.2s ease'
        }}>
          ▶
        </span>
      </div>

      {/* Container content area */}
      {!isCollapsed && (
        <div 
          className="container-content"
          style={{
            position: 'absolute',
            top: '32px',
            left: '8px',
            right: '8px',
            bottom: '8px',
            pointerEvents: 'none' // Allow child nodes to be interactive
          }}
        />
      )}
    </div>
  );
};

// Helper functions for styling
function getNodeBorderColor(style: string, selected?: boolean, highlighted?: boolean): string {
  if (selected) return '#007bff';
  if (highlighted) return '#ffc107';
  
  switch (style) {
    case 'error': return '#dc3545';
    case 'warning': return '#fd7e14';
    case 'highlighted': return '#ffc107';
    case 'selected': return '#007bff';
    default: return '#6c757d';
  }
}

function getNodeTextColor(style: string): string {
  switch (style) {
    case 'error': return '#721c24';
    case 'warning': return '#856404';
    default: return '#212529';
  }
}
