/**
 * @fileoverview Shared event handler utilities
 *
 * Centralized event handlers to avoid duplication across components.
 */
import React from 'react';
/**
 * Base event handler factory for nodes
 */
export declare function createNodeEventHandlers(id: string, data?: any): {
    handleClick: (event: React.MouseEvent) => void;
    handleDoubleClick: (event: React.MouseEvent) => void;
    handleContextMenu: (event: React.MouseEvent) => void;
};
/**
 * Base event handler factory for edges
 */
export declare function createEdgeEventHandlers(id: string, data?: any): {
    handleClick: (event: React.MouseEvent) => void;
    handleContextMenu: (event: React.MouseEvent) => void;
};
/**
 * Container-specific event handler
 */
export declare function createContainerEventHandlers(id: string, data?: any): {
    handleToggleCollapse: (event: React.MouseEvent) => void;
    handleClick: (event: React.MouseEvent) => void;
    handleDoubleClick: (event: React.MouseEvent) => void;
    handleContextMenu: (event: React.MouseEvent) => void;
};
//# sourceMappingURL=eventHandlers.d.ts.map