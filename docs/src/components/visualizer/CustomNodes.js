/**
 * Enhanced Custom Node Components for ReactFlow v12
 * 
 * Leverages v12's improved event handling, sub-flows, and measured dimensions
 */

import React, { useRef } from 'react';
import { Handle, Position } from '@xyflow/react';

// Enhanced ContainerNode leveraging ReactFlow v12's sub-flow improvements
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
        justifyContent: 'center'
      }}
    >
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
      
      {isCollapsed ? label : null}
    </div>
  );
};

// Enhanced LabelNode for ReactFlow v12
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
      pointerEvents: 'none', // v12: Better event isolation
      userSelect: 'none'
    }}>
      {data.label}
    </div>
  );
};
