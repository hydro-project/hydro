/**
 * @fileoverview Custom ReactFlow Node Components
 * 
 * Custom node components for rendering graph elements.
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
import { createNodeEventHandlers } from './eventHandlers';
import { 
  getNodeColorByType, 
  createDarkBorder, 
  createVerticalGradient 
} from './colorUtils';

// Standard Node Component with Strong Typing
export const GraphStandardNode: React.FC<NodeProps> = (props) => {
  const { data, selected, id } = props;
  
  // Create display label from strongly typed data
  const displayLabel = (data.label || id) as string;
  
  // Get node type and generate colors
  const nodeType = (data as any)?.nodeType || 'Transform';
  const baseColor = getNodeColorByType(nodeType);
  const darkBorderColor = createDarkBorder(baseColor);
  const gradient = createVerticalGradient(baseColor);
  
  // Use shared event handlers
  const eventHandlers = createNodeEventHandlers(id, data);

  return (
    <div 
      className={`graph-standard-node ${data.style} ${selected ? 'selected' : ''}`}
      onClick={eventHandlers.handleClick}
      onDoubleClick={eventHandlers.handleDoubleClick}
      onContextMenu={eventHandlers.handleContextMenu}
      style={{
        // Apply the gradient directly here
        background: gradient,
        border: `1px solid ${darkBorderColor}`, // Very thin border with much darker color
        fontSize: '12px',
        fontWeight: 600,
        color: '#333', // Darker text for better contrast on light Set3 colors
        textAlign: 'center',
        cursor: 'pointer',
        userSelect: 'none',
        padding: '8px 12px',
        borderRadius: '4px',
        width: '100%',
        height: '100%',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        boxSizing: 'border-box',
        textShadow: '0 1px 1px rgba(255,255,255,0.3)' // Light text shadow for legibility
      }}
    >
      {/* Flexible connection handles - continuous positioning for ReactFlow v12 */}
      {/* Left side handles */}
      <Handle
        type="target"
        position={Position.Left}
        id="left-top"
        style={{ 
          background: NODE_COLORS.HANDLE, 
          left: -4, 
          top: '25%',
          transform: 'translateY(-50%)'
        }}
      />
      <Handle
        type="target"
        position={Position.Left}
        id="left-center"
        style={{ 
          background: NODE_COLORS.HANDLE, 
          left: -4, 
          top: '50%',
          transform: 'translateY(-50%)'
        }}
      />
      <Handle
        type="target"
        position={Position.Left}
        id="left-bottom"
        style={{ 
          background: NODE_COLORS.HANDLE, 
          left: -4, 
          top: '75%',
          transform: 'translateY(-50%)'
        }}
      />
      
      {/* Top side handles */}
      <Handle
        type="target" 
        position={Position.Top}
        id="top-left"
        style={{ 
          background: NODE_COLORS.HANDLE, 
          top: -4, 
          left: '25%',
          transform: 'translateX(-50%)'
        }}
      />
      <Handle
        type="target" 
        position={Position.Top}
        id="top-center"
        style={{ 
          background: NODE_COLORS.HANDLE, 
          top: -4, 
          left: '50%',
          transform: 'translateX(-50%)'
        }}
      />
      <Handle
        type="target" 
        position={Position.Top}
        id="top-right"
        style={{ 
          background: NODE_COLORS.HANDLE, 
          top: -4, 
          left: '75%',
          transform: 'translateX(-50%)'
        }}
      />
      
      {/* Right side handles */}
      <Handle
        type="source"
        position={Position.Right}
        id="right-top"
        style={{ 
          background: NODE_COLORS.HANDLE, 
          right: -4, 
          top: '25%',
          transform: 'translateY(-50%)'
        }}
      />
      <Handle
        type="source"
        position={Position.Right}
        id="right-center"
        style={{ 
          background: NODE_COLORS.HANDLE, 
          right: -4, 
          top: '50%',
          transform: 'translateY(-50%)'
        }}
      />
      <Handle
        type="source"
        position={Position.Right}
        id="right-bottom"
        style={{ 
          background: NODE_COLORS.HANDLE, 
          right: -4, 
          top: '75%',
          transform: 'translateY(-50%)'
        }}
      />
      
      {/* Bottom side handles */}
      <Handle
        type="source"
        position={Position.Bottom}
        id="bottom-left"
        style={{ 
          background: NODE_COLORS.HANDLE, 
          bottom: -4, 
          left: '25%',
          transform: 'translateX(-50%)'
        }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        id="bottom-center"
        style={{ 
          background: NODE_COLORS.HANDLE, 
          bottom: -4, 
          left: '50%',
          transform: 'translateX(-50%)'
        }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        id="bottom-right"
        style={{ 
          background: NODE_COLORS.HANDLE, 
          bottom: -4, 
          left: '75%',
          transform: 'translateX(-50%)'
        }}
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
  
  const handleClick = (event: React.MouseEvent) => {
    event.stopPropagation();
    
    // Check if this is a click on the container background (not on child nodes)
    const target = event.target as HTMLElement;
    const isBackgroundClick = target.classList.contains('graph-container-node') || 
                              target.classList.contains('container-content');
    
    if (isBackgroundClick && !isCollapsed) {
      // Request container collapse - pass to parent via data callback
      if (data.onContainerCollapse) {
        data.onContainerCollapse(id);
      }
    }
  };

  const handleToggleCollapse = (event: React.MouseEvent) => {
    event.stopPropagation();
    // Collapse toggle handlers would go here - for explicit toggle buttons
  };

  return (
    <div 
      className={`graph-container-node ${selected ? 'selected' : ''}`}
      onClick={handleClick}
      style={{
        width: width,
        height: isCollapsed ? SIZES.COLLAPSED_CONTAINER_HEIGHT : height,
        background: CONTAINER_COLORS.BACKGROUND,
        border: `${SIZES.BORDER_WIDTH_DEFAULT}px solid ${selected ? CONTAINER_COLORS.BORDER_SELECTED : CONTAINER_COLORS.BORDER}`,
        borderRadius: '12px',
        position: 'relative',
        cursor: isCollapsed ? 'pointer' : 'pointer',
        transition: 'all 0.3s ease-in-out'
      }}
    >
      {/* Container content area */}
      {!isCollapsed && (
        <div 
          className="container-content"
          style={{
            position: 'absolute',
            top: '8px',
            left: '8px',
            right: '8px',
            bottom: '8px',
            pointerEvents: 'none' // Allow child nodes to be interactive
          }}
        />
      )}

      {/* Container title - bottom right with shadow for legibility */}
      <div 
        className="container-title"
        style={{
          position: 'absolute',
          bottom: '8px',
          right: '12px',
          fontSize: '11px',
          fontWeight: '500',
          color: '#374151',
          textShadow: '0 1px 3px rgba(255, 255, 255, 0.9), 0 0 8px rgba(255, 255, 255, 0.8)',
          pointerEvents: 'none', // Don't interfere with container interactions
          userSelect: 'none',
          zIndex: 10 // Ensure it appears above other content
        }}
      >
        {id}
      </div>

      {/* Connection handles - only show on collapsed containers with flexible positioning */}
      {isCollapsed && (
        <>
          {/* Left side handles */}
          <Handle
            type="target"
            position={Position.Left}
            id="left-top"
            style={{ 
              background: CONTAINER_COLORS.BORDER, 
              left: -4, 
              top: '30%',
              transform: 'translateY(-50%)',
              width: 8, 
              height: 8 
            }}
          />
          <Handle
            type="target"
            position={Position.Left}
            id="left-bottom"
            style={{ 
              background: CONTAINER_COLORS.BORDER, 
              left: -4, 
              top: '70%',
              transform: 'translateY(-50%)',
              width: 8, 
              height: 8 
            }}
          />
          
          {/* Top side handles */}
          <Handle
            type="target" 
            position={Position.Top}
            id="top-left"
            style={{ 
              background: CONTAINER_COLORS.BORDER, 
              top: -4, 
              left: '30%',
              transform: 'translateX(-50%)',
              width: 8, 
              height: 8 
            }}
          />
          <Handle
            type="target" 
            position={Position.Top}
            id="top-right"
            style={{ 
              background: CONTAINER_COLORS.BORDER, 
              top: -4, 
              left: '70%',
              transform: 'translateX(-50%)',
              width: 8, 
              height: 8 
            }}
          />
          
          {/* Right side handles */}
          <Handle
            type="source"
            position={Position.Right}
            id="right-top"
            style={{ 
              background: CONTAINER_COLORS.BORDER, 
              right: -4, 
              top: '30%',
              transform: 'translateY(-50%)',
              width: 8, 
              height: 8 
            }}
          />
          <Handle
            type="source"
            position={Position.Right}
            id="right-bottom"
            style={{ 
              background: CONTAINER_COLORS.BORDER, 
              right: -4, 
              top: '70%',
              transform: 'translateY(-50%)',
              width: 8, 
              height: 8 
            }}
          />
          
          {/* Bottom side handles */}
          <Handle
            type="source"
            position={Position.Bottom}
            id="bottom-left"
            style={{ 
              background: CONTAINER_COLORS.BORDER, 
              bottom: -4, 
              left: '30%',
              transform: 'translateX(-50%)',
              width: 8, 
              height: 8 
            }}
          />
          <Handle
            type="source"
            position={Position.Bottom}
            id="bottom-right"
            style={{ 
              background: CONTAINER_COLORS.BORDER, 
              bottom: -4, 
              left: '70%',
              transform: 'translateX(-50%)',
              width: 8, 
              height: 8 
            }}
          />
        </>
      )}
    </div>
  );
};
