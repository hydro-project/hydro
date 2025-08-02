/**
 * @fileoverview Custom ReactFlow Node Components
 * 
 * Custom node c      <Handle
        type="target"
        position={Position.Left}
        style={{ background: NODE_COLORS.HANDLE }}
      />
      <Handle
        type="source"
        position={Position.Right}
        style={{ background: NODE_COLORS.HANDLE }}
      />for rendering graph elements.
 */

import React from 'react';
import { Handle, Position, NodeProps } from '@xyflow/react';
import { 
  getNodeBorderColor, 
  getNodeTextColor,
  COLORS,
  NODE_COLORS,
  SIZES,
  SHADOWS,
  TYPOGRAPHY,
  CONTAINER_COLORS,
  DEFAULT_NODE_STYLE,
  COMPONENT_COLORS
} from '../shared/config';
import { 
  StandardNodeProps, 
  ContainerNodeProps, 
  isContainerNodeData 
} from './types';

// Standard Node Component with Strong Typing
export const GraphStandardNode: React.FC<StandardNodeProps> = ({ 
  data, 
  selected, 
  id 
}) => {
  // Create display label from strongly typed data
  const displayLabel = data.label || id;
  
  const handleClick = (event: React.MouseEvent) => {
    event.stopPropagation();
    // Node click handlers would go here
  };

  const handleDoubleClick = (event: React.MouseEvent) => {
    event.stopPropagation();
    // Node double-click handlers would go here
  };

  const handleContextMenu = (event: React.MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
    // Context menu handlers would go here
  };

  return (
    <div 
      className={`graph-standard-node ${data.style} ${selected ? 'selected' : ''}`}
      onClick={handleClick}
      onDoubleClick={handleDoubleClick}
      onContextMenu={handleContextMenu}
      style={{
        // Style is now applied in the nodeStyler, so we just need basic styling here
        border: `${SIZES.BORDER_WIDTH_DEFAULT}px solid ${getNodeBorderColor(data.style, selected, false)}`,
        boxShadow: selected ? SHADOWS.NODE_SELECTED : SHADOWS.NODE_DEFAULT,
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
        {displayLabel}
      </div>
    </div>
  );
};

// Container Node Component with Strong Typing
export const GraphContainerNode: React.FC<ContainerNodeProps> = ({ 
  data, 
  selected, 
  id
}) => {
  // Type-safe access to container data
  if (!isContainerNodeData(data)) {
    console.error(`[GraphContainerNode] ❌ Invalid data for container ${id}: missing width/height`);
    return <div>Invalid container data</div>;
  }
  
  const isCollapsed = data.collapsed;
  
  // Use ELK-calculated dimensions from strongly typed data
  const width = data.width;
  const height = data.height;
  
  console.log(`[GraphContainerNode] 📦 Rendering container ${id}: ${width}x${height} (ELK dimensions: ✅)`);
  
  const handleClick = (event: React.MouseEvent) => {
    event.stopPropagation();
    // Container click handlers would go here
  };

  const handleToggleCollapse = (event: React.MouseEvent) => {
    event.stopPropagation();
    // Collapse toggle handlers would go here
  };

  return (
    <div 
      className={`graph-container-node ${selected ? 'selected' : ''}`}
      onClick={handleClick}
      style={{
        width: width,
        height: isCollapsed ? 40 : height,
        background: CONTAINER_COLORS.BACKGROUND,
        border: `${SIZES.BORDER_WIDTH_DEFAULT}px solid ${selected ? CONTAINER_COLORS.BORDER_SELECTED : CONTAINER_COLORS.BORDER}`,
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
          background: CONTAINER_COLORS.HEADER_BACKGROUND,
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
