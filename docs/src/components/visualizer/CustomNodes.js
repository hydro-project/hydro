/**
 * Simplified Custom Node Components for ReactFlow v12
 * 
 * Leverages v12's improved event handling and sub-flows
 * Reduced to only the essential custom behaviors that built-in types can't provide
 */

import React, { useRef } from 'react';
import { Handle, Position } from '@xyflow/react';

// Simplified ContainerNode - only the essential click-to-toggle behavior
// All other features now use ReactFlow v12's built-in capabilities
export const ContainerNode = ({ id, data }) => {
  const { onContainerToggle, label, isCollapsed, isDraggedRef } = data;
  const clickTimeRef = useRef(null);
  
  // ReactFlow v12: Much more reliable drag/click detection
  const handlePointerDown = (event) => {
    clickTimeRef.current = Date.now();
    // v12: Better drag state management
    if (isDraggedRef && isDraggedRef.current) {
      isDraggedRef.current[id] = false;
    }
  };

  const handlePointerUp = (event) => {
    const clickDuration = clickTimeRef.current ? Date.now() - clickTimeRef.current : 0;
    const wasReactFlowDragged = isDraggedRef && isDraggedRef.current && isDraggedRef.current[id];
    
    // ReactFlow v12: Simplified click detection due to better event handling
    if (clickDuration < 150 && !wasReactFlowDragged && onContainerToggle) {
      event.stopPropagation();
      onContainerToggle(id);
    }
    
    clickTimeRef.current = null;
  };

  const handleContextMenu = (event) => {
    event.preventDefault();
    if (onContainerToggle) {
      onContainerToggle(id);
    }
  };

  return (
    <div 
      onPointerDown={handlePointerDown}
      onPointerUp={handlePointerUp}
      onContextMenu={handleContextMenu}
      style={{ 
        width: '100%', 
        height: '100%', 
        cursor: 'pointer',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        position: 'relative' // Enable absolute positioning for label
      }}
    >
      {/* Container label positioned at top center */}
      {!isCollapsed && label && (
        <div style={{
          position: 'absolute',
          top: '4px',
          left: '50%',
          transform: 'translateX(-50%)',
          fontSize: '12px',
          fontWeight: 'bold',
          color: '#000',
          textShadow: '1px 1px 2px rgba(255, 255, 255, 0.9), -1px -1px 2px rgba(255, 255, 255, 0.9), 1px -1px 2px rgba(255, 255, 255, 0.9), -1px 1px 2px rgba(255, 255, 255, 0.9)',
          whiteSpace: 'nowrap',
          pointerEvents: 'none', // Don't interfere with container clicks
          zIndex: 10, // Ensure label appears above other content
          userSelect: 'none',
          letterSpacing: '0.5px'
        }}>
          {label}
        </div>
      )}

      {/* ReactFlow v12: Better handle positioning and connection */}
      <Handle
        type="target"
        position={Position.Top}
        id="top"
        style={{ background: '#555' }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        id="bottom"
        style={{ background: '#555' }}
      />
      <Handle
        type="target"
        position={Position.Left}
        id="left"
        style={{ background: '#555' }}
      />
      <Handle
        type="source"
        position={Position.Right}
        id="right"
        style={{ background: '#555' }}
      />
      
      {/* Show label in center only when collapsed */}
      {isCollapsed && (
        <div style={{
          fontSize: '13px',
          fontWeight: 'bold',
          color: '#000',
          textAlign: 'center',
          textShadow: '1px 1px 2px rgba(255, 255, 255, 0.9), -1px -1px 2px rgba(255, 255, 255, 0.9), 1px -1px 2px rgba(255, 255, 255, 0.9), -1px 1px 2px rgba(255, 255, 255, 0.9)',
          pointerEvents: 'none',
          userSelect: 'none',
          letterSpacing: '0.5px',
          maxWidth: '90%',
          overflow: 'hidden',
          textOverflow: 'ellipsis'
        }}>
          {label}
        </div>
      )}
    </div>
  );
};

// ✅ LabelNode completely removed - now using ReactFlow v12's built-in 'default' type
