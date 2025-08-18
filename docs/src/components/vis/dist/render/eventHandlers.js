/**
 * @fileoverview Shared event handler utilities
 *
 * Centralized event handlers to avoid duplication across components.
 */
/**
 * Base event handler factory for nodes
 */
export function createNodeEventHandlers(id, data) {
    const handleClick = (event) => {
        event.stopPropagation();
        if (data?.onNodeClick) {
            data.onNodeClick(id);
        }
    };
    const handleDoubleClick = (event) => {
        event.stopPropagation();
        if (data?.onNodeDoubleClick) {
            data.onNodeDoubleClick(id);
        }
    };
    const handleContextMenu = (event) => {
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
export function createEdgeEventHandlers(id, data) {
    const handleClick = (event) => {
        event.stopPropagation();
        if (data?.onEdgeClick) {
            data.onEdgeClick(id);
        }
    };
    const handleContextMenu = (event) => {
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
export function createContainerEventHandlers(id, data) {
    const baseHandlers = createNodeEventHandlers(id, data);
    const handleToggleCollapse = (event) => {
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
//# sourceMappingURL=eventHandlers.js.map