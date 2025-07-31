/**
 * Dockable MiniMap Component
 * 
 * A movable and dockable wrapper for the ReactFlow MiniMap
 */

import React from 'react';
import { MiniMap } from '@xyflow/react';
import { DockablePanel, DOCK_POSITIONS } from './DockablePanel.js';

export function DockableMiniMap({ 
  onPositionChange,
  nodeColor,
  nodeStrokeColor,
  nodeClassName,
  nodeBorderRadius,
  nodeStrokeWidth,
  maskColor,
  maskStrokeColor,
  maskStrokeWidth,
  ...miniMapProps 
}) {
  return (
    <DockablePanel
      id="minimap"
      title="Mini Map"
      defaultPosition={DOCK_POSITIONS.BOTTOM_LEFT}
      defaultDocked={true}
      defaultCollapsed={false}
      onPositionChange={onPositionChange}
      minWidth={150}
      minHeight={100}
    >
      <div style={{ 
        width: '150px', 
        height: '100px',
        border: '1px solid #ddd',
        borderRadius: '4px',
        overflow: 'hidden'
      }}>
        <MiniMap
          nodeColor={nodeColor || '#e2e8f0'}
          nodeStrokeColor={nodeStrokeColor || '#94a3b8'}
          nodeClassName={nodeClassName}
          nodeBorderRadius={nodeBorderRadius || 2}
          nodeStrokeWidth={nodeStrokeWidth || 1}
          maskColor={maskColor || 'rgba(100, 116, 139, 0.1)'}
          maskStrokeColor={maskStrokeColor || '#64748b'}
          maskStrokeWidth={maskStrokeWidth || 1}
          style={{
            backgroundColor: '#f8fafc',
            border: 'none'
          }}
          {...miniMapProps}
        />
      </div>
    </DockablePanel>
  );
}
