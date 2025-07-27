/**
 * Custom Node Components for ReactFlow
 * 
 * Contains custom node types including ContainerNode and LabelNode
 * with proper click/drag handling and edge connection support
 */

import React, { useRef } from 'react';
import { Handle, Position } from '@xyflow/react';

// Custom node for containers with simplified click handling for ReactFlow v12
export const ContainerNode = ({ id, data }) => {
  const { onContainerToggle, label, isCollapsed, isDraggedRef } = data;
  const clickTimeRef = useRef(null);
  
  // ReactFlow v12 has much better drag/click distinction
  const handlePointerDown = (event) => {
    clickTimeRef.current = Date.now();
    // Reset drag state
    if (isDraggedRef && isDraggedRef.current) {
      isDraggedRef.current[id] = false;
    }
  };

  const handlePointerUp = (event) => {
    const clickDuration = clickTimeRef.current ? Date.now() - clickTimeRef.current : 0;
    const wasReactFlowDragged = isDraggedRef && isDraggedRef.current && isDraggedRef.current[id];
    
    // ReactFlow v12 handles drag detection much better, so we can simplify this
    if (clickDuration < 200 && !wasReactFlowDragged && onContainerToggle) {
      event.stopPropagation();
      onContainerToggle(id);
    }
    
    clickTimeRef.current = null;
  };

  // Keep right-click as alternative
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
        justifyContent: 'center'
      }}
    >
      {/* ReactFlow v12 handles are more reliable */}
      {Handle && Position && (
        <>
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
        </>
      )}
      
      {isCollapsed ? label : null}
    </div>
  );
};

// Custom label node component - no connection handles
export const LabelNode = ({ data }) => {
  return (
    <div style={{
      background: 'rgba(255, 255, 255, 0.95)',
      border: '1px solid #ddd',
      borderRadius: '4px',
      fontSize: '11px',
      fontWeight: 'bold',
      color: '#333',
      padding: '4px 8px',
      boxShadow: '0 1px 3px rgba(0,0,0,0.1)',
      whiteSpace: 'nowrap',
      pointerEvents: 'none', // Ensure labels don't interfere with clicks
      userSelect: 'none' // Prevent text selection
    }}>
      {data.label}
    </div>
  );
};
