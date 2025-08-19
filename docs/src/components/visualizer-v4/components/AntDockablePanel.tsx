/**
 * @fileoverview Ant Design-based Dockable Panel
 * 
 * Uses Ant Design's Affix + Card for reliable dockable panel functionality.
 * This provides proper docked panel behavior without modal overlay issues.
 */

import React, { useState } from 'react';
import { Card, Button, Space, Affix } from 'antd';
import { 
  CloseOutlined, 
  PushpinOutlined, 
  PushpinFilled,
  ExpandOutlined,
  CompressOutlined,
  DragOutlined
} from '@ant-design/icons';

export interface AntDockablePanelProps {
  title: string;
  children: React.ReactNode;
  defaultOpen?: boolean;
  defaultCollapsed?: boolean;
  placement?: 'top' | 'right' | 'bottom' | 'left';
  width?: number | string;
  height?: number | string;
  className?: string;
  style?: React.CSSProperties;
  onOpenChange?: (open: boolean) => void;
  onCollapsedChange?: (collapsed: boolean) => void;
}

export function AntDockablePanel({
  title,
  children,
  defaultOpen = true,
  defaultCollapsed = false,
  placement = 'right',
  width = 320,
  height = 'auto',
  className = '',
  style,
  onOpenChange,
  onCollapsedChange
}: AntDockablePanelProps) {
  const [open, setOpen] = useState(defaultOpen);
  const [collapsed, setCollapsed] = useState(defaultCollapsed);
  const [pinned, setPinned] = useState(true);

  const handleOpenChange = (newOpen: boolean) => {
    setOpen(newOpen);
    onOpenChange?.(newOpen);
  };

  const handleToggleCollapse = () => {
    const newCollapsed = !collapsed;
    setCollapsed(newCollapsed);
    onCollapsedChange?.(newCollapsed);
  };

  const handleTogglePin = () => {
    setPinned(!pinned);
  };

  if (!open) {
    return null;
  }

  const cardStyle: React.CSSProperties = {
    position: pinned ? 'fixed' : 'absolute',
    zIndex: 1000,
    boxShadow: '0 4px 12px rgba(0, 0, 0, 0.15)',
    borderRadius: '6px',
    maxHeight: collapsed ? 'auto' : '80vh',
    overflow: 'hidden',
    ...style
  };

  // Position based on placement
  if (placement === 'right') {
    cardStyle.right = '16px';
    cardStyle.top = '80px';
    cardStyle.width = width;
  } else if (placement === 'left') {
    cardStyle.left = '16px';
    cardStyle.top = '80px';
    cardStyle.width = width;
  } else if (placement === 'top') {
    cardStyle.top = '16px';
    cardStyle.right = '16px';
    cardStyle.width = width;
  } else if (placement === 'bottom') {
    cardStyle.bottom = '16px';
    cardStyle.right = '16px';
    cardStyle.width = width;
  }

  const extra = (
    <Space size={4}>
      <Button
        type="text"
        size="small"
        icon={collapsed ? <ExpandOutlined /> : <CompressOutlined />}
        onClick={handleToggleCollapse}
        title={collapsed ? "Expand" : "Collapse"}
      />
      <Button
        type="text"
        size="small"
        icon={pinned ? <PushpinFilled /> : <PushpinOutlined />}
        onClick={handleTogglePin}
        title={pinned ? "Unpin" : "Pin"}
      />
      <Button
        type="text"
        size="small"
        icon={<CloseOutlined />}
        onClick={() => handleOpenChange(false)}
        title="Close"
      />
    </Space>
  );

  const cardContent = (
    <Card
      title={title}
      size="small"
      extra={extra}
      style={cardStyle}
      className={className}
      styles={{
        header: {
          padding: '8px 12px',
          minHeight: '40px',
          borderBottom: collapsed ? 'none' : '1px solid #f0f0f0'
        },
        body: {
          padding: collapsed ? 0 : '12px',
          height: collapsed ? 0 : 'auto',
          overflow: collapsed ? 'hidden' : 'auto',
          maxHeight: collapsed ? 0 : '60vh'
        }
      }}
    >
      {!collapsed && children}
    </Card>
  );

  // Use Affix for pinned panels to keep them in view when scrolling
  if (pinned) {
    return (
      <Affix offsetTop={placement === 'top' ? 16 : undefined} offsetBottom={placement === 'bottom' ? 16 : undefined}>
        {cardContent}
      </Affix>
    );
  }

  return cardContent;
}

// Compatibility wrapper for existing DockablePanel usage
export interface LegacyDockablePanelProps {
  title: string;
  children: React.ReactNode;
  defaultCollapsed?: boolean;
  defaultPosition?: { x: number; y: number };
  onPositionChange?: (position: { x: number; y: number }) => void;
  className?: string;
  style?: React.CSSProperties;
}

export function DockablePanel(props: LegacyDockablePanelProps) {
  // Map legacy props to new AntDockablePanel
  return (
    <AntDockablePanel
      title={props.title}
      defaultOpen={!props.defaultCollapsed}
      defaultCollapsed={props.defaultCollapsed}
      placement="right"
      className={props.className}
      style={props.style}
    >
      {props.children}
    </AntDockablePanel>
  );
}

// Re-export for backward compatibility
export { AntDockablePanel as DockablePanel2 };
