/**
 * Custom Node Components for ReactFlow
 * 
 * Contains custom node types including ContainerNode and LabelNode
 * with proper click/drag handling and edge connection support
 */

import React, { useRef } from 'react';
import { ReactFlowComponents } from './externalLibraries.js';

// Custom node for containers to handle clicks directly
export const ContainerNode = ({ id, data }) => {
  // The toggle function is passed through the node's data
  const { onContainerToggle, label, isCollapsed, isDraggedRef } = data;
  const dragStartTimeRef = useRef(null);
  const dragStartPosRef = useRef(null);
  const dragThresholdRef = useRef(false);
  
  // Get ReactFlow components
  const { Handle, Position } = ReactFlowComponents || {};

  // SOLUTION FOR REACTFLOW CLICK VS DRAG HANDLING:
  // We need to distinguish between clicks (for toggling) and drags (for moving).
  // ReactFlow intercepts pointer events during drag, so we track drag state via position changes.
  const handlePointerDown = (event) => {
    dragStartTimeRef.current = Date.now();
    dragStartPosRef.current = { x: event.clientX, y: event.clientY };
    dragThresholdRef.current = false;
    
    // Reset the drag state flag when starting a new interaction
    if (isDraggedRef && isDraggedRef.current) {
      isDraggedRef.current[id] = false;
    }
    
    // Don't stop propagation here - let ReactFlow handle drag initiation
  };

  const handlePointerMove = (event) => {
    // If pointer moves significantly, consider this a drag operation
    if (dragStartTimeRef.current && dragStartPosRef.current) {
      const deltaX = Math.abs(event.clientX - dragStartPosRef.current.x);
      const deltaY = Math.abs(event.clientY - dragStartPosRef.current.y);
      const distance = Math.sqrt(deltaX * deltaX + deltaY * deltaY);
      
      // If moved more than 5 pixels, consider it a drag
      if (distance > 5) {
        dragThresholdRef.current = true;
      }
    }
  };

  const handlePointerUp = (event) => {
    const now = Date.now();
    const timeDiff = dragStartTimeRef.current ? now - dragStartTimeRef.current : 0;
    
    // Calculate final distance moved
    let finalDistance = 0;
    if (dragStartPosRef.current) {
      const deltaX = Math.abs(event.clientX - dragStartPosRef.current.x);
      const deltaY = Math.abs(event.clientY - dragStartPosRef.current.y);
      finalDistance = Math.sqrt(deltaX * deltaX + deltaY * deltaY);
    }
    
    // Check if ReactFlow detected this container as being dragged
    const wasReactFlowDragged = isDraggedRef && isDraggedRef.current && isDraggedRef.current[id];
    
    // Only toggle if this was a quick click AND ReactFlow didn't detect a drag
    if (timeDiff < 300 && finalDistance < 5 && !dragThresholdRef.current && !wasReactFlowDragged && onContainerToggle) {
      event.stopPropagation(); // Only stop propagation for actual clicks
      onContainerToggle(id);
    }
    
    // Reset tracking
    dragStartTimeRef.current = null;
    dragStartPosRef.current = null;
    dragThresholdRef.current = false;
  };

  // Keep right-click as an alternative interaction method
  const handleContextMenu = (event) => {
    event.preventDefault();
    if (onContainerToggle) {
      onContainerToggle(id);
    }
  };

  // The outer div is sized and positioned by ReactFlow.
  // This inner div fills the node, captures clicks, and displays content.
  return (
    <div 
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
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
      {/* ReactFlow handles for edge connections */}
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
      
      {/* Only show the label if the container is collapsed. */}
      {/* Expanded containers get their label from a separate LabelNode. */}
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
