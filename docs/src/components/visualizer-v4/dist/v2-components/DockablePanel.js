import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
/**
 * Dockable Panel Component
 *
 * A draggable and dockable panel that can be positioned at various locations
 */
import React, { useState, useRef, useEffect, useCallback } from 'react';
import styles from '../../../pages/visualizer.module.css';
// Available dock positions
export const DOCK_POSITIONS = {
    TOP_LEFT: 'top-left',
    TOP_RIGHT: 'top-right',
    BOTTOM_LEFT: 'bottom-left',
    BOTTOM_RIGHT: 'bottom-right',
    FLOATING: 'floating'
};
export function DockablePanel({ id, title, children, defaultPosition = DOCK_POSITIONS.TOP_RIGHT, defaultDocked = true, defaultCollapsed = false, onPositionChange, className = '', minWidth = 200, minHeight = 100 }) {
    const [position, setPosition] = useState(defaultPosition);
    const [isDocked, setIsDocked] = useState(defaultDocked);
    const [isCollapsed, setIsCollapsed] = useState(defaultCollapsed);
    const [floatingPosition, setFloatingPosition] = useState({ x: 20, y: 20 });
    const [isDragging, setIsDragging] = useState(false);
    const [dragOffset, setDragOffset] = useState({ x: 0, y: 0 });
    const panelRef = useRef(null);
    const headerRef = useRef(null);
    // Handle drag start
    const handleDragStart = useCallback((e) => {
        if (!headerRef.current || !panelRef.current)
            return;
        const rect = panelRef.current.getBoundingClientRect();
        const clientX = e.touches ? e.touches[0].clientX : e.clientX;
        const clientY = e.touches ? e.touches[0].clientY : e.clientY;
        setDragOffset({
            x: clientX - rect.left,
            y: clientY - rect.top
        });
        setIsDragging(true);
        // If currently docked, convert to floating
        if (isDocked) {
            setIsDocked(false);
            setPosition(DOCK_POSITIONS.FLOATING);
            setFloatingPosition({
                x: rect.left,
                y: rect.top
            });
        }
        e.preventDefault();
    }, [isDocked]);
    // Handle drag move
    const handleDragMove = useCallback((e) => {
        if (!isDragging)
            return;
        const clientX = e.touches ? e.touches[0].clientX : e.clientX;
        const clientY = e.touches ? e.touches[0].clientY : e.clientY;
        setFloatingPosition({
            x: clientX - dragOffset.x,
            y: clientY - dragOffset.y
        });
        e.preventDefault();
    }, [isDragging, dragOffset]);
    // Handle drag end with docking logic
    const handleDragEnd = useCallback((e) => {
        if (!isDragging)
            return;
        setIsDragging(false);
        const clientX = e.changedTouches ? e.changedTouches[0].clientX : e.clientX;
        const clientY = e.changedTouches ? e.changedTouches[0].clientY : e.clientY;
        // Get viewport dimensions
        const viewportWidth = window.innerWidth;
        const viewportHeight = window.innerHeight;
        // Define dock zones (20% of viewport on each edge)
        const dockZoneSize = 0.2;
        const leftZone = viewportWidth * dockZoneSize;
        const rightZone = viewportWidth * (1 - dockZoneSize);
        const topZone = viewportHeight * dockZoneSize;
        const bottomZone = viewportHeight * (1 - dockZoneSize);
        // Determine dock position based on mouse position
        let newPosition = DOCK_POSITIONS.FLOATING;
        let shouldDock = false;
        if (clientX <= leftZone && clientY <= topZone) {
            newPosition = DOCK_POSITIONS.TOP_LEFT;
            shouldDock = true;
        }
        else if (clientX >= rightZone && clientY <= topZone) {
            newPosition = DOCK_POSITIONS.TOP_RIGHT;
            shouldDock = true;
        }
        else if (clientX <= leftZone && clientY >= bottomZone) {
            newPosition = DOCK_POSITIONS.BOTTOM_LEFT;
            shouldDock = true;
        }
        else if (clientX >= rightZone && clientY >= bottomZone) {
            newPosition = DOCK_POSITIONS.BOTTOM_RIGHT;
            shouldDock = true;
        }
        setPosition(newPosition);
        setIsDocked(shouldDock);
        // Notify parent of position change
        onPositionChange?.(newPosition, shouldDock);
        e.preventDefault();
    }, [isDragging, onPositionChange]);
    // Add global event listeners for drag
    useEffect(() => {
        if (isDragging) {
            const handleMouseMove = (e) => handleDragMove(e);
            const handleMouseUp = (e) => handleDragEnd(e);
            const handleTouchMove = (e) => handleDragMove(e);
            const handleTouchEnd = (e) => handleDragEnd(e);
            document.addEventListener('mousemove', handleMouseMove);
            document.addEventListener('mouseup', handleMouseUp);
            document.addEventListener('touchmove', handleTouchMove, { passive: false });
            document.addEventListener('touchend', handleTouchEnd);
            return () => {
                document.removeEventListener('mousemove', handleMouseMove);
                document.removeEventListener('mouseup', handleMouseUp);
                document.removeEventListener('touchmove', handleTouchMove);
                document.removeEventListener('touchend', handleTouchEnd);
            };
        }
    }, [isDragging, handleDragMove, handleDragEnd]);
    // Generate CSS classes based on dock position
    const getPositionClasses = () => {
        const classes = [styles.dockablePanel];
        if (className)
            classes.push(className);
        if (isDragging)
            classes.push(styles.dockablePanelDragging);
        if (isCollapsed)
            classes.push(styles.dockablePanelCollapsed);
        if (isDocked) {
            classes.push(styles.dockablePanelDocked);
            classes.push(styles[`dockablePanel${position.replace('-', '').replace(/\b\w/g, l => l.toUpperCase())}`]);
        }
        else {
            classes.push(styles.dockablePanelFloating);
        }
        return classes.join(' ');
    };
    // Generate inline styles for floating position
    const getInlineStyles = () => {
        if (!isDocked && position === DOCK_POSITIONS.FLOATING) {
            return {
                left: `${floatingPosition.x}px`,
                top: `${floatingPosition.y}px`,
                minWidth: `${minWidth}px`,
                minHeight: isCollapsed ? 'auto' : `${minHeight}px`
            };
        }
        return {};
    };
    return (_jsxs("div", { ref: panelRef, className: getPositionClasses(), style: getInlineStyles(), "data-panel-id": id, children: [_jsxs("div", { ref: headerRef, className: styles.dockablePanelHeader, onMouseDown: handleDragStart, onTouchStart: handleDragStart, children: [_jsx("h4", { className: styles.dockablePanelTitle, children: title }), _jsxs("div", { className: styles.dockablePanelControls, children: [_jsx("button", { className: styles.dockablePanelToggle, onClick: () => setIsCollapsed(!isCollapsed), title: isCollapsed ? `Expand ${title}` : `Collapse ${title}`, children: isCollapsed ? '⌄' : '⌃' }), _jsx("div", { className: styles.dockablePanelDragHandle, title: `Drag to move ${title}`, children: "\u22EE\u22EE" })] })] }), !isCollapsed && (_jsx("div", { className: styles.dockablePanelContent, children: children })), isDragging && (_jsxs("div", { className: styles.dockZoneOverlay, children: [_jsx("div", { className: `${styles.dockZone} ${styles.dockZoneTopLeft}` }), _jsx("div", { className: `${styles.dockZone} ${styles.dockZoneTopRight}` }), _jsx("div", { className: `${styles.dockZone} ${styles.dockZoneBottomLeft}` }), _jsx("div", { className: `${styles.dockZone} ${styles.dockZoneBottomRight}` })] }))] }));
}
//# sourceMappingURL=DockablePanel.js.map