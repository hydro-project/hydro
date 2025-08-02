/**
 * @fileoverview DockablePanel Component
 * 
 * A modern, TypeScript implementation of the dockable panel system.
 * Provides draggable and dockable panels with position management.
 */

import React, { useState, useRef, useEffect, useCallback } from 'react';
import { DockablePanelProps, PANEL_POSITIONS, PanelPosition } from './types.js';
import { COMPONENT_COLORS, SIZES, SHADOWS } from '../shared/config.js';

export function DockablePanel({
  id,
  title,
  children,
  defaultPosition = PANEL_POSITIONS.TOP_RIGHT,
  defaultDocked = true,
  defaultCollapsed = false,
  onPositionChange,
  onDockChange,
  onCollapseChange,
  className = '',
  minWidth = 200,
  minHeight = 100,
  maxWidth = 400,
  maxHeight = 600,
  style
}: DockablePanelProps) {
  const [position, setPosition] = useState<PanelPosition>(defaultPosition);
  const [isDocked, setIsDocked] = useState(defaultDocked);
  const [isCollapsed, setIsCollapsed] = useState(defaultCollapsed);
  const [floatingPosition, setFloatingPosition] = useState({ x: 20, y: 20 });
  const [isDragging, setIsDragging] = useState(false);
  const [dragOffset, setDragOffset] = useState({ x: 0, y: 0 });
  
  const panelRef = useRef<HTMLDivElement>(null);
  const headerRef = useRef<HTMLDivElement>(null);

  // Handle drag start
  const handleDragStart = useCallback((e: React.MouseEvent | React.TouchEvent) => {
    if (!headerRef.current || !panelRef.current) return;
    
    const rect = panelRef.current.getBoundingClientRect();
    const clientX = 'touches' in e ? e.touches[0].clientX : e.clientX;
    const clientY = 'touches' in e ? e.touches[0].clientY : e.clientY;
    
    setDragOffset({
      x: clientX - rect.left,
      y: clientY - rect.top,
    });
    setIsDragging(true);
    
    // Prevent text selection during drag
    e.preventDefault();
  }, []);

  // Handle drag move
  const handleDragMove = useCallback((e: MouseEvent | TouchEvent) => {
    if (!isDragging) return;
    
    const clientX = 'touches' in e ? e.touches[0].clientX : e.clientX;
    const clientY = 'touches' in e ? e.touches[0].clientY : e.clientY;
    
    setFloatingPosition({
      x: clientX - dragOffset.x,
      y: clientY - dragOffset.y,
    });
    
    // Check if we should dock to edges
    const windowWidth = window.innerWidth;
    const windowHeight = window.innerHeight;
    const dockThreshold = 50;
    
    let newPosition: PanelPosition = PANEL_POSITIONS.FLOATING;
    
    if (clientX < dockThreshold && clientY < windowHeight / 2) {
      newPosition = PANEL_POSITIONS.TOP_LEFT;
    } else if (clientX > windowWidth - dockThreshold && clientY < windowHeight / 2) {
      newPosition = PANEL_POSITIONS.TOP_RIGHT;
    } else if (clientX < dockThreshold && clientY >= windowHeight / 2) {
      newPosition = PANEL_POSITIONS.BOTTOM_LEFT;
    } else if (clientX > windowWidth - dockThreshold && clientY >= windowHeight / 2) {
      newPosition = PANEL_POSITIONS.BOTTOM_RIGHT;
    }
    
    if (newPosition !== PANEL_POSITIONS.FLOATING) {
      setPosition(newPosition);
      setIsDocked(true);
    } else {
      setPosition(PANEL_POSITIONS.FLOATING);
      setIsDocked(false);
    }
  }, [isDragging, dragOffset]);

  // Handle drag end
  const handleDragEnd = useCallback(() => {
    setIsDragging(false);
    
    // Notify parent of position change
    if (onPositionChange) {
      onPositionChange(id, position);
    }
    
    if (onDockChange) {
      onDockChange(id, isDocked);
    }
  }, [id, position, isDocked, onPositionChange, onDockChange]);

  // Attach global drag listeners
  useEffect(() => {
    if (isDragging) {
      document.addEventListener('mousemove', handleDragMove);
      document.addEventListener('mouseup', handleDragEnd);
      document.addEventListener('touchmove', handleDragMove);
      document.addEventListener('touchend', handleDragEnd);
      
      return () => {
        document.removeEventListener('mousemove', handleDragMove);
        document.removeEventListener('mouseup', handleDragEnd);
        document.removeEventListener('touchmove', handleDragMove);
        document.removeEventListener('touchend', handleDragEnd);
      };
    }
  }, [isDragging, handleDragMove, handleDragEnd]);

  // Handle collapse toggle
  const handleCollapseToggle = useCallback(() => {
    const newCollapsed = !isCollapsed;
    setIsCollapsed(newCollapsed);
    
    if (onCollapseChange) {
      onCollapseChange(id, newCollapsed);
    }
  }, [id, isCollapsed, onCollapseChange]);

  // Calculate panel styles based on position
  const getPanelStyles = (): React.CSSProperties => {
    const baseStyles: React.CSSProperties = {
      position: 'absolute',
      minWidth: `${minWidth}px`,
      minHeight: `${minHeight}px`,
      maxWidth: `${maxWidth}px`,
      maxHeight: `${maxHeight}px`,
      backgroundColor: COMPONENT_COLORS.PANEL_BACKGROUND,
      border: `1px solid ${COMPONENT_COLORS.BORDER_MEDIUM}`,
      borderRadius: SIZES.BORDER_RADIUS_DEFAULT,
      boxShadow: isDragging ? SHADOWS.PANEL_DRAGGING : SHADOWS.PANEL_DEFAULT,
      zIndex: isDragging ? 1000 : 100,
      overflow: 'hidden',
    };

    if (isDocked && position !== PANEL_POSITIONS.FLOATING) {
      // Docked positioning
      const dockOffset = 12;
      
      switch (position) {
        case PANEL_POSITIONS.TOP_LEFT:
          return { ...baseStyles, top: dockOffset, left: dockOffset };
        case PANEL_POSITIONS.TOP_RIGHT:
          return { ...baseStyles, top: dockOffset, right: dockOffset };
        case PANEL_POSITIONS.BOTTOM_LEFT:
          return { ...baseStyles, bottom: dockOffset, left: dockOffset };
        case PANEL_POSITIONS.BOTTOM_RIGHT:
          return { ...baseStyles, bottom: dockOffset, right: dockOffset };
      }
    }
    
    // Floating positioning
    return {
      ...baseStyles,
      left: `${floatingPosition.x}px`,
      top: `${floatingPosition.y}px`,
    };
  };

  return (
    <div
      ref={panelRef}
      className={`dockable-panel ${className}`}
      style={{ ...getPanelStyles(), ...style }}
    >
      {/* Panel Header */}
      <div
        ref={headerRef}
        onMouseDown={handleDragStart}
        onTouchStart={handleDragStart}
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '8px 12px',
          backgroundColor: COMPONENT_COLORS.PANEL_HEADER_BACKGROUND,
          borderBottom: `1px solid ${COMPONENT_COLORS.BORDER_LIGHT}`,
          cursor: 'move',
          userSelect: 'none',
          fontSize: '11px',
          fontWeight: 'bold',
          color: COMPONENT_COLORS.TEXT_PRIMARY,
        }}
      >
        <span>{title}</span>
        <div style={{ display: 'flex', gap: '4px' }}>
          {/* Collapse Toggle */}
          <button
            onClick={handleCollapseToggle}
            style={{
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              fontSize: '10px',
              color: COMPONENT_COLORS.TEXT_SECONDARY,
              padding: '2px 4px',
              borderRadius: '2px',
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.backgroundColor = COMPONENT_COLORS.BUTTON_HOVER_BACKGROUND;
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.backgroundColor = 'transparent';
            }}
            title={isCollapsed ? 'Expand panel' : 'Collapse panel'}
          >
            {isCollapsed ? '▲' : '▼'}
          </button>
          
          {/* Dock Indicator */}
          {isDocked && (
            <span
              style={{
                fontSize: '8px',
                color: COMPONENT_COLORS.TEXT_DISABLED,
                padding: '2px',
              }}
              title="Panel is docked"
            >
              📌
            </span>
          )}
        </div>
      </div>

      {/* Panel Content */}
      {!isCollapsed && (
        <div
          style={{
            padding: '12px',
            fontSize: '10px',
            maxHeight: `${maxHeight - 40}px`,
            overflowY: 'auto',
          }}
        >
          {children}
        </div>
      )}
    </div>
  );
}

export { PANEL_POSITIONS };
