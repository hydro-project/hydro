/**
 * @fileoverview Custom ReactFlow Node Components
 * 
 * Custom node components for rendering graph elements.
 */

import React from 'react';
import { Handle, Position, NodeProps } from '@xyflow/react';
import { 
  getNodeBorderColor, 
  getNodeTextColor,
  COLORS,
  NODE_COLORS,
  SIZES,
  SHADOWS,
  TYPOGRAPHY,
  CONTAINER_COLORS,
  DEFAULT_NODE_STYLE,
  COMPONENT_COLORS
} from '../shared/config';
import { 
  StandardNodeProps, 
  ContainerNodeProps, 
  isContainerNodeData 
} from './types';

// Standard Node Component with Strong Typing
export const GraphStandardNode: React.FC<NodeProps> = (props) => {
  const { data, selected, id } = props;
  
  // Create display label from strongly typed data
  const displayLabel = (data.label || id) as string;
  
  // Get node type and generate colors directly in component
  const nodeType = (data as any)?.nodeType || 'Transform';
  
  // Simple color mapping based on node type - Set3 palette
  const getNodeColor = (type: string) => {
    switch (type) {
      case 'Source': return '#8dd3c7'; // Light teal
      case 'Transform': return '#ffffb3'; // Light yellow  
      case 'Tee': return '#bebada'; // Light purple
      case 'Network': return '#fb8072'; // Light red/salmon
      case 'Sink': return '#80b1d3'; // Light blue
      default: return '#b3de69'; // Light green
    }
  };
  
  const baseColor = getNodeColor(nodeType);
  
  // Create a much darker border color
  const createDarkBorder = (color: string) => {
    // Convert hex to RGB for manipulation
    const hex = color.replace('#', '');
    const r = parseInt(hex.substr(0, 2), 16);
    const g = parseInt(hex.substr(2, 2), 16);
    const b = parseInt(hex.substr(4, 2), 16);
    
    // Create darker border (multiply by 0.6 for distinguishable but still dark)
    const darkR = Math.floor(r * 0.6);
    const darkG = Math.floor(g * 0.6);
    const darkB = Math.floor(b * 0.6);
    
    return `rgb(${darkR}, ${darkG}, ${darkB})`;
  };
  
  const darkBorderColor = createDarkBorder(baseColor);
  
  // Create a vertical gradient: darker at top, lighter at bottom
  const createGradient = (color: string) => {
    // Convert hex to RGB for manipulation
    const hex = color.replace('#', '');
    const r = parseInt(hex.substr(0, 2), 16);
    const g = parseInt(hex.substr(2, 2), 16);
    const b = parseInt(hex.substr(4, 2), 16);
    
    // Create a darker top (multiply by 0.8) and lighter bottom (multiply by 1.2, capped at 255)
    const topR = Math.floor(r * 0.8);
    const topG = Math.floor(g * 0.8);
    const topB = Math.floor(b * 0.8);
    
    const bottomR = Math.min(Math.floor(r * 1.2), 255);
    const bottomG = Math.min(Math.floor(g * 1.2), 255);
    const bottomB = Math.min(Math.floor(b * 1.2), 255);
    
    const topColor = `rgb(${topR}, ${topG}, ${topB})`;
    const bottomColor = `rgb(${bottomR}, ${bottomG}, ${bottomB})`;
    
    return `linear-gradient(to bottom, ${topColor}, ${bottomColor})`;
  };
  
  const gradient = createGradient(baseColor);
  
  const handleClick = (event: React.MouseEvent) => {
    event.stopPropagation();
    // Node click handlers would go here
  };

  const handleDoubleClick = (event: React.MouseEvent) => {
    event.stopPropagation();
    // Node double-click handlers would go here
  };

  const handleContextMenu = (event: React.MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
    // Context menu handlers would go here
  };

  return (
    <div 
      className={`graph-standard-node ${data.style} ${selected ? 'selected' : ''}`}
      onClick={handleClick}
      onDoubleClick={handleDoubleClick}
      onContextMenu={handleContextMenu}
      style={{
        // Apply the gradient directly here
        background: gradient,
        border: `1px solid ${darkBorderColor}`, // Very thin border with much darker color
        fontSize: '12px',
        fontWeight: 600,
        color: '#333', // Darker text for better contrast on light Set3 colors
        textAlign: 'center',
        cursor: 'pointer',
        userSelect: 'none',
        padding: '8px 12px',
        borderRadius: '4px',
        width: '100%',
        height: '100%',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        boxSizing: 'border-box',
        textShadow: '0 1px 1px rgba(255,255,255,0.3)' // Light text shadow for legibility
      }}
    >
      {/* Connection handles */}
      <Handle
        type="target"
        position={Position.Top}
        style={{ background: NODE_COLORS.HANDLE }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        style={{ background: NODE_COLORS.HANDLE }}
      />
      
      {/* Node content */}
      <div style={{ textAlign: 'center', overflow: 'hidden', textOverflow: 'ellipsis' }}>
        {displayLabel}
      </div>
    </div>
  );
};

// Container Node Component with Strong Typing
export const GraphContainerNode: React.FC<ContainerNodeProps> = ({ 
  data, 
  selected, 
  id
}) => {
  // Type-safe access to container data
  if (!isContainerNodeData(data)) {
    console.error(`[GraphContainerNode] ❌ Invalid data for container ${id}: missing width/height`);
    return <div>Invalid container data</div>;
  }
  
  const isCollapsed = data.collapsed;
  
  // Use ELK-calculated dimensions from strongly typed data
  const width = data.width;
  const height = data.height;
  
  const handleClick = (event: React.MouseEvent) => {
    event.stopPropagation();
    // Container click handlers would go here
  };

  const handleToggleCollapse = (event: React.MouseEvent) => {
    event.stopPropagation();
    // Collapse toggle handlers would go here
  };

  return (
    <div 
      className={`graph-container-node ${selected ? 'selected' : ''}`}
      onClick={handleClick}
      style={{
        width: width,
        height: isCollapsed ? 40 : height,
        background: CONTAINER_COLORS.BACKGROUND,
        border: `${SIZES.BORDER_WIDTH_DEFAULT}px solid ${selected ? CONTAINER_COLORS.BORDER_SELECTED : CONTAINER_COLORS.BORDER}`,
        borderRadius: '12px',
        position: 'relative',
        cursor: 'pointer',
        transition: 'all 0.3s ease-in-out'
      }}
    >
      {/* Container content area */}
      {!isCollapsed && (
        <div 
          className="container-content"
          style={{
            position: 'absolute',
            top: '8px',
            left: '8px',
            right: '8px',
            bottom: '8px',
            pointerEvents: 'none' // Allow child nodes to be interactive
          }}
        />
      )}

      {/* Container title - bottom right with shadow for legibility */}
      <div 
        className="container-title"
        style={{
          position: 'absolute',
          bottom: '8px',
          right: '12px',
          fontSize: '11px',
          fontWeight: '500',
          color: '#374151',
          textShadow: '0 1px 3px rgba(255, 255, 255, 0.9), 0 0 8px rgba(255, 255, 255, 0.8)',
          pointerEvents: 'none', // Don't interfere with container interactions
          userSelect: 'none',
          zIndex: 10 // Ensure it appears above other content
        }}
      >
        {id}
      </div>
    </div>
  );
};
