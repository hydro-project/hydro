/**
 * Common handle styles and configurations for ReactFlow nodes
 * 
 * Centralizes handle positioning and styling to ensure consistency
 * across all node types (GroupNode, CollapsedContainerNode, etc.)
 */

import { REQUIRED_HANDLE_IDS } from './handleValidation.js';

/**
 * Create invisible handle style for seamless edge connections
 * Handles are positioned at the exact border with no visual appearance
 * Using zero offset for perfect edge-to-node connection
 */
export function createInvisibleHandleStyle(position) {
  const baseStyle = {
    background: 'transparent',
    border: 'none',
    width: 1,            // Minimal size for connection point
    height: 1,           // Minimal size for connection point
    opacity: 0,          // Completely invisible
    zIndex: 1000,        // Ensure handles are on top for edge connections
    pointerEvents: 'none', // Don't interfere with interactions
  };

  // Position handles exactly at the border with precise negative offset
  // This ensures edges connect directly to the border without gaps
  switch (position) {
    case 'right':
      return { 
        ...baseStyle, 
        right: -0.5, 
        top: '50%',
        transform: 'translateY(-50%)'
      };
    case 'left':
      return { 
        ...baseStyle, 
        left: -0.5,
        top: '50%', 
        transform: 'translateY(-50%)'
      };
    case 'bottom':
      return { 
        ...baseStyle, 
        bottom: -0.5,
        left: '50%',
        transform: 'translateX(-50%)'
      };
    case 'top':
      return { 
        ...baseStyle, 
        top: -0.5,
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
      position: "right",
      id: REQUIRED_HANDLE_IDS.source,
      style: createInvisibleHandleStyle('right')
    },
    {
      type: "target",
      position: "left", 
      id: REQUIRED_HANDLE_IDS.target,
      style: createInvisibleHandleStyle('left')
    },
    {
      type: "source",
      position: "bottom",
      id: REQUIRED_HANDLE_IDS.sourceBottom,
      style: createInvisibleHandleStyle('bottom')
    },
    {
      type: "target",
      position: "top",
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
