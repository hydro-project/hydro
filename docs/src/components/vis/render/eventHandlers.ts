/**
 * @fileoverview Shared event handler utilities
 * 
 * Centralized event handlers to avoid duplication across components.
 */

import React from 'react';

/**
 * Base event handler factory for nodes
 */
export function createNodeEventHandlers(id: string, data?: any) {
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

  return {
    handleClick,
    handleDoubleClick,
    handleContextMenu
  };
}

/**
 * Base event handler factory for edges
 */
export function createEdgeEventHandlers(id: string, data?: any) {
  const handleClick = (event: React.MouseEvent) => {
    event.stopPropagation();
    if (data?.onEdgeClick) {
      data.onEdgeClick(id);
    }
  };

  const handleContextMenu = (event: React.MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
    if (data?.onEdgeContextMenu) {
      data.onEdgeContextMenu(id, event);
    }
  };

  return {
    handleClick,
    handleContextMenu
  };
}

/**
 * Container-specific event handler
 */
export function createContainerEventHandlers(id: string, data?: any) {
  const baseHandlers = createNodeEventHandlers(id, data);
  
  const handleToggleCollapse = (event: React.MouseEvent) => {
    event.stopPropagation();
    if (data?.onToggleCollapse) {
      data.onToggleCollapse(id);
    }
  };

  return {
    ...baseHandlers,
    handleToggleCollapse
  };
}
