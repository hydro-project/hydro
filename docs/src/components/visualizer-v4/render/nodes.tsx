/**
 * @fileoverview Bridge-Based Node Components
 * 
 * ReactFlow node components with configurable handle system for maximum layout flexibility
 */

import React from 'react';
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { getHandleConfig, CONTINUOUS_HANDLE_STYLE } from './handleConfig';
import { generateNodeColors } from '../shared/colorUtils';
import { COLLAPSED_CONTAINER_STYLES, EXPANDED_CONTAINER_STYLES } from '../shared/config';
import { truncateLabel, generateContainerSummary } from '../shared/labelUtils';

/**
 * Render handles based on current configuration
 */
function renderHandles() {
  const config = getHandleConfig();
  
  if (config.enableContinuousHandles) {
    // ReactFlow v12 continuous handles - connections anywhere on perimeter
    return (
      <>
        <Handle
          type="source"
          position={Position.Top}
          style={CONTINUOUS_HANDLE_STYLE}
          isConnectable={true}
        />
        <Handle
          type="target"
          position={Position.Top}
          style={CONTINUOUS_HANDLE_STYLE}
          isConnectable={true}
        />
      </>
    );
  }
  
  // Discrete handles if configured
  return (
    <>
      {config.sourceHandles.map(handle => (
        <Handle
          key={handle.id}
          id={handle.id}
          type="source"
          position={handle.position}
          style={handle.style}
          isConnectable={true}
        />
      ))}
      {config.targetHandles.map(handle => (
        <Handle
          key={handle.id}
          id={handle.id}
          type="target"
          position={handle.position}
          style={handle.style}
          isConnectable={true}
        />
      ))}
    </>
  );
}

/**
 * Standard graph node component
 */
export function StandardNode({ id, data }: NodeProps) {
  // Get dynamic colors based on node type (preferred) or style as fallback
  const nodeType = String(data.nodeType || data.style || 'default');
  const colorPalette = String(data.colorPalette || 'Set3');
  const colors = generateNodeColors([nodeType], colorPalette);
  
  return (
    <div
      style={{
        padding: '12px 16px',
        background: colors.primary,
        border: `1px solid ${colors.border}`,
        borderRadius: '4px',
        fontSize: '12px',
        textAlign: 'center',
        minWidth: '120px',
        boxShadow: '0 2px 4px rgba(0,0,0,0.1)',
        position: 'relative'
      }}
    >
      {renderHandles()}
      {String(data.label || id)}
    </div>
  );
}

/**
 * Container node component with professional styling and label truncation
 */
export function ContainerNode({ id, data }: NodeProps) {
  // Use dimensions from ELK layout via ReactFlowBridge data
  const width = data.width || 180; // fallback to default
  const height = data.height || (data.collapsed ? 60 : 120); // fallback to default
  
  const isCollapsed = data.collapsed || false;
  
  // Apply professional styling based on collapse state
  const containerStyles = isCollapsed ? COLLAPSED_CONTAINER_STYLES : EXPANDED_CONTAINER_STYLES;
  
  // Truncate label using intelligent algorithm
  const originalLabel = String(data.label || id);
  const truncatedLabel = truncateLabel(originalLabel, containerStyles.LABEL_MAX_LENGTH || 20);
  
  // Generate summary for collapsed containers
  const summary = isCollapsed && data.visState ? 
    generateContainerSummary(data, data.visState) : null;
  
  // Debug: Log container dimensions
  console.log(`[ContainerNode] 📏 Container ${id}: data.width=${data.width}, data.height=${data.height}, using ${width}x${height}, collapsed=${isCollapsed}`);
  
  return (
    <div
      style={{
        padding: '16px',
        background: containerStyles.BACKGROUND,
        border: containerStyles.BORDER,
        borderRadius: containerStyles.BORDER_RADIUS,
        width: `${width}px`,  // Use ELK-calculated width
        height: `${height}px`, // Use ELK-calculated height
        position: 'relative',
        boxSizing: 'border-box', // Ensure padding is included in dimensions
        boxShadow: isCollapsed ? containerStyles.BOX_SHADOW : undefined,
        transition: 'background-color 0.2s ease, border-color 0.2s ease', // Smooth hover
        display: 'flex',
        flexDirection: 'column',
        justifyContent: isCollapsed ? 'center' : 'flex-end', // Center content for collapsed
        alignItems: isCollapsed ? 'center' : 'flex-end', // Center for collapsed, bottom-right for expanded
      }}
    >
      {renderHandles()}
      
      {isCollapsed ? (
        // Collapsed container: centered layout with truncated label and summary
        <div style={{
          textAlign: 'center',
          width: '100%'
        }}>
          <div style={{
            fontSize: containerStyles.LABEL_FONT_SIZE,
            fontWeight: containerStyles.LABEL_FONT_WEIGHT,
            color: containerStyles.LABEL_COLOR,
            marginBottom: summary ? '4px' : '0',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            maxWidth: `${Number(width) - 32}px`, // Account for padding
          }}>
            {truncatedLabel}
          </div>
          
          {summary && (
            <div style={{
              fontSize: containerStyles.SUMMARY_FONT_SIZE,
              fontWeight: containerStyles.SUMMARY_FONT_WEIGHT,
              color: containerStyles.SUMMARY_COLOR,
              fontStyle: 'italic'
            }}>
              {summary}
            </div>
          )}
        </div>
      ) : (
        // Expanded container: label positioned at bottom-right (existing behavior)
        <div
          style={{
            position: 'absolute',
            bottom: '12px',
            right: '12px',
            fontSize: containerStyles.LABEL_FONT_SIZE,
            fontWeight: containerStyles.LABEL_FONT_WEIGHT,
            color: containerStyles.LABEL_COLOR,
            maxWidth: `${Number(width) - 36}px`, // Ensure label doesn't overflow container
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            // Text shadow for better legibility over container background
            textShadow: '1px 1px 2px rgba(255, 255, 255, 0.8), -1px -1px 2px rgba(255, 255, 255, 0.8)',
            filter: 'drop-shadow(0px 1px 2px rgba(0, 0, 0, 0.1))'
          }}
        >
          {truncatedLabel}
        </div>
      )}
    </div>
  );
}

// Export map for ReactFlow nodeTypes
export const nodeTypes = {
  standard: StandardNode,
  container: ContainerNode
};