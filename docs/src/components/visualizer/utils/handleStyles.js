/**
 * Common handle styles and configurations for ReactFlow nodes
 * 
 * Centralizes handle positioning and styling to ensure consistency
 * across all node types (GroupNode, CollapsedContainerNode, etc.)
 */

import { Position } from '@xyflow/react';
import { REQUIRED_HANDLE_IDS } from './handleValidation.js';

/**
 * Create invisible handle style for seamless edge connections
 * Handles are positioned at the exact border with no visual appearance
 * Using zero offset for perfect edge-to-node connection
 */
export function createInvisibleHandleStyle(position) {
  const baseStyle = {
    background: 'red',      // Temporary: make visible for debugging
    border: '2px solid blue', // Temporary: make visible for debugging
    width: 12,              // Temporary: make larger for debugging
    height: 12,             // Temporary: make larger for debugging
    opacity: 1,             // Temporary: make visible for debugging
    zIndex: 1000,           // Ensure handles are on top for edge connections
    pointerEvents: 'all',   // Enable interactions for debugging
    borderRadius: '50%',    // Make them circular for visibility
  };

  // Position handles clearly inside the container bounds for debugging
  switch (position) {
    case 'right':
      return { 
        ...baseStyle, 
        right: 2,               // Inside the container
        top: '50%',
        transform: 'translateY(-50%)'
      };
    case 'left':
      return { 
        ...baseStyle, 
        left: 2,                // Inside the container
        top: '50%', 
        transform: 'translateY(-50%)'
      };
    case 'bottom':
      return { 
        ...baseStyle, 
        bottom: 2,              // Inside the container
        left: '50%',
        transform: 'translateX(-50%)'
      };
    case 'top':
      return { 
        ...baseStyle, 
        top: 2,                 // Inside the container
        left: '50%',
        transform: 'translateX(-50%)'
      };
    default:
      return baseStyle;
  }
}

/**
 * Standard handle configuration for container nodes
 * Returns array of handle props for consistent application
 */
export function getContainerHandles() {
  return [
    {
      type: "source",
      position: Position.Right,
      id: REQUIRED_HANDLE_IDS.source,
      style: createInvisibleHandleStyle('right')
    },
    {
      type: "target",
      position: Position.Left, 
      id: REQUIRED_HANDLE_IDS.target,
      style: createInvisibleHandleStyle('left')
    },
    {
      type: "source",
      position: Position.Bottom,
      id: REQUIRED_HANDLE_IDS.sourceBottom,
      style: createInvisibleHandleStyle('bottom')
    },
    {
      type: "target",
      position: Position.Top,
      id: REQUIRED_HANDLE_IDS.targetTop,
      style: createInvisibleHandleStyle('top')
    }
  ];
}

/**
 * Render handles using the standard configuration
 * Use this in both GroupNode and CollapsedContainerNode
 */
export function renderContainerHandles(HandleComponent) {
  return getContainerHandles().map(handleProps => 
    HandleComponent({
      key: handleProps.id,
      ...handleProps
    })
  );
}
