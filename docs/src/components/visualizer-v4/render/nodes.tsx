/**
 * @fileoverview Bridge-Based Node Components
 * 
 * ReactFlow node components with configurable handle system for maximum layout flexibility
 */

import React from 'react';
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { getHandleConfig, CONTINUOUS_HANDLE_STYLE } from './handleConfig';
import { generateNodeColors } from '../shared/colorUtils';
import { truncateContainerName, truncateLabel } from '../shared/textUtils';
import { useStyleConfig } from './StyleConfigContext';

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
  const styleCfg = useStyleConfig();
  // Get dynamic colors based on node type (preferred) or style as fallback
  const nodeType = String(data.nodeType || data.style || 'default');
  const colorPalette = String(data.colorPalette || 'Set3');
  const colors = generateNodeColors([nodeType], colorPalette);
  
  // Determine which label to display
  // Priority: data.label (if set by toggle) > data.shortLabel > id
  const displayLabel = data.label || data.shortLabel || id;
  
  return (
    <div
      style={{
        padding: `${styleCfg.nodePadding ?? 12}px 16px`,
        background: colors.primary,
        border: `1px solid ${colors.border}`,
        borderRadius: `${styleCfg.nodeBorderRadius ?? 4}px`,
        fontSize: `${styleCfg.nodeFontSize ?? 12}px`,
        textAlign: 'center',
        minWidth: '120px',
        boxShadow: '0 2px 4px rgba(0,0,0,0.1)',
        position: 'relative',
        cursor: 'pointer', // Indicate that the node is clickable
        transition: 'all 0.2s ease' // Smooth transition for hover effects
      }}
      title={data.fullLabel ? `Click to toggle between:\n"${data.shortLabel || id}"\n"${data.fullLabel}"` : undefined} // Tooltip
    >
      {renderHandles()}
      {String(displayLabel)}
    </div>
  );
}

/**
 * Container node component with label positioned at bottom-right
 */
export function ContainerNode({ id, data }: NodeProps) {
  const styleCfg = useStyleConfig();
  // Use dimensions from ELK layout via ReactFlowBridge data
  const width = data.width || 180; // fallback to default
  const height = data.height || (data.collapsed ? 100 : 120); // fallback to default, taller for collapsed
  
  // Get color palette and node count from data
  const colorPalette = String(data.colorPalette || 'Set3');
  const nodeCount = Number(data.nodeCount || 0);
  
  // Get the container label
  const containerLabel = String(data.label || id);
  
  // Debug: Log container dimensions
  // // console.log(((`[ContainerNode] 📏 Container ${id}: data.width=${data.width}, data.height=${data.height}, using ${width}x${height}`)));
  
  // Generate professional colors based on palette for collapsed containers
  const generateContainerColors = (containerId: string, palette: string) => {
    // Use a simple hash of the container ID to get consistent colors
    const hash = containerId.split('').reduce((a, b) => a + b.charCodeAt(0), 0);
    const colorPalettes: Record<string, string[]> = {
      'Set3': ['#8dd3c7', '#ffffb3', '#bebada', '#fb8072', '#80b1d3', '#fdb462', '#b3de69'],
      'Pastel1': ['#fbb4ae', '#b3cde3', '#ccebc5', '#decbe4', '#fed9a6', '#ffffcc', '#e5d8bd'],
      'Dark2': ['#1b9e77', '#d95f02', '#7570b3', '#e7298a', '#66a61e', '#e6ab02', '#a6761d'],
      'Set1': ['#e41a1c', '#377eb8', '#4daf4a', '#984ea3', '#ff7f00', '#ffff33', '#a65628'],
      'Set2': ['#66c2a5', '#fc8d62', '#8da0cb', '#e78ac3', '#a6d854', '#ffd92f', '#e5c494']
    };
    
    const colors = colorPalettes[palette] || colorPalettes['Set3'];
    const baseColor = colors[hash % colors.length];
    
    // Create lighter background and darker border
    const lighten = (color: string, factor: number) => {
      const hex = color.replace('#', '');
      const r = parseInt(hex.substring(0, 2), 16);
      const g = parseInt(hex.substring(2, 4), 16);
      const b = parseInt(hex.substring(4, 6), 16);
      
      const newR = Math.floor(r + (255 - r) * factor);
      const newG = Math.floor(g + (255 - g) * factor);
      const newB = Math.floor(b + (255 - b) * factor);
      
      return `#${newR.toString(16).padStart(2, '0')}${newG.toString(16).padStart(2, '0')}${newB.toString(16).padStart(2, '0')}`;
    };
    
    const darken = (color: string, factor: number) => {
      const hex = color.replace('#', '');
      const r = parseInt(hex.substring(0, 2), 16);
      const g = parseInt(hex.substring(2, 4), 16);
      const b = parseInt(hex.substring(4, 6), 16);
      
      const newR = Math.floor(r * (1 - factor));
      const newG = Math.floor(g * (1 - factor));
      const newB = Math.floor(b * (1 - factor));
      
      return `#${newR.toString(16).padStart(2, '0')}${newG.toString(16).padStart(2, '0')}${newB.toString(16).padStart(2, '0')}`;
    };
    
    return {
      background: lighten(baseColor, 0.8), // Very light background
      border: darken(baseColor, 0.2), // Darker border
      text: darken(baseColor, 0.4) // Readable text color
    };
  };
  
  if (data.collapsed) {
    // Professional collapsed container styling
    const containerColors = generateContainerColors(id, colorPalette);
    
    return (
      <div
        style={{
          width: `${width}px`,
          height: `${height}px`,
          background: containerColors.background,
          border: `${styleCfg.containerBorderWidth ?? 2}px solid ${containerColors.border}`,
          borderRadius: `${styleCfg.containerBorderRadius ?? 8}px`,
          position: 'relative',
          boxSizing: 'border-box',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          cursor: 'pointer',
          boxShadow: styleCfg.containerShadow === 'NONE' ? 'none' :
            styleCfg.containerShadow === 'LARGE' ? '0 10px 15px -3px rgba(0,0,0,0.2)' :
            styleCfg.containerShadow === 'MEDIUM' ? '0 4px 6px -1px rgba(0,0,0,0.15)' :
            '0 2px 8px rgba(0,0,0,0.15)',
          transition: 'all 0.2s ease'
        }}
      >
        {renderHandles()}
        
        {/* Container title - centered and truncated */}
        <div
          style={{
            fontSize: '13px',
            fontWeight: '600',
            color: containerColors.text,
            textAlign: 'center',
            maxWidth: `${Number(width) - 16}px`,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            marginBottom: '4px',
            cursor: 'pointer' // Indicate container is clickable for collapse/expand
          }}
        >
          {truncateLabel(containerLabel, {
            maxLength: Math.floor((Number(width) - 16) / 8), // ~8px per character
            preferDelimiters: true,
            leftTruncate: true // Keep end for collapsed containers (like Rust paths)
          })}
        </div>
        
        {/* Node count - smaller text below title */}
        <div
          style={{
            fontSize: '11px',
            color: containerColors.text,
            opacity: 0.8,
            textAlign: 'center'
          }}
        >
          {nodeCount} node{nodeCount !== 1 ? 's' : ''}
        </div>
      </div>
    );
  }
  
  // Expanded container styling (unchanged)
  return (
    <div
      style={{
  padding: `${Math.max((styleCfg.nodePadding ?? 12) + 4, 8)}px`,
        background: 'rgba(25, 118, 210, 0.1)',
  border: `${styleCfg.containerBorderWidth ?? 2}px solid #1976d2`,
  borderRadius: `${styleCfg.containerBorderRadius ?? 8}px`,
        width: `${width}px`,  // Use ELK-calculated width
        height: `${height}px`, // Use ELK-calculated height (now includes label space)
        position: 'relative',
        boxSizing: 'border-box', // Ensure padding is included in dimensions
        cursor: 'pointer' // Indicate container is clickable for collapse/expand
      }}
    >
      {renderHandles()}
      
      {/* Container label positioned at bottom-right */}
      <div
        style={{
          position: 'absolute',
          bottom: '0px',  // Decreased from 12px to give more space from internal nodes
          right: '12px',   // Keep horizontal spacing the same
          fontSize: '12px',
          fontWeight: 'bold',
          color: '#1976d2',
          maxWidth: `${Number(width) - 36}px`, // Ensure label doesn't overflow container (increased padding)
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
          cursor: 'pointer', // Indicate container is clickable for collapse/expand
          // Text shadow for better legibility over container background
          textShadow: '1px 1px 2px rgba(255, 255, 255, 0.8), -1px -1px 2px rgba(255, 255, 255, 0.8), 1px -1px 2px rgba(255, 255, 255, 0.8), -1px 1px 2px rgba(255, 255, 255, 0.8)',
          // Subtle drop shadow for the text element itself
          filter: 'drop-shadow(0px 1px 2px rgba(0, 0, 0, 0.1))'
        }}
      >
        {truncateLabel(containerLabel, {
          maxLength: Math.floor((Number(width) - 36) / 8), // ~8px per character
          preferDelimiters: true,
          leftTruncate: false // Keep beginning for expanded containers
        })}
      </div>
    </div>
  );
}

// Export map for ReactFlow nodeTypes
export const nodeTypes = {
  standard: StandardNode,
  container: ContainerNode
};