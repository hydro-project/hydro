/**
 * @fileoverview HierarchyTree Component
 * 
 * Displays an interactive tree view of container hierarchy for navigation.
 */

import React, { useMemo } from 'react';
import { HierarchyTreeProps, HierarchyTreeNode } from './types.js';
import { COMPONENT_COLORS } from '../shared/config.js';

export function HierarchyTree({
  hierarchyTree,
  collapsedContainers = new Set(),
  onToggleContainer,
  title = 'Container Hierarchy',
  showNodeCounts = true,
  truncateLabels = true,
  maxLabelLength = 20,
  className = '',
  style
}: HierarchyTreeProps) {

  // Utility function to truncate labels
  const truncateLabel = (label: string, maxLength: number): string => {
    if (!truncateLabels || label.length <= maxLength) {
      return label;
    }
    
    // Try to split on common delimiters and keep the end
    const delimiters = ['::', '.', '_', '-'];
    for (const delimiter of delimiters) {
      if (label.includes(delimiter)) {
        const parts = label.split(delimiter);
        const lastPart = parts[parts.length - 1];
        if (lastPart.length <= maxLength) {
          return `...${delimiter}${lastPart}`;
        }
      }
    }
    
    // Fallback to simple truncation
    return `...${label.slice(-maxLength + 3)}`;
  };

  // Count immediate leaf (non-container) children of a container
  const countLeafChildren = (node: HierarchyTreeNode): number => {
    // This would need to be passed from parent or calculated differently
    // For now, we'll use the nodeCount from the tree structure
    return Math.max(0, node.nodeCount - node.children.length);
  };

  // Recursive tree node component
  const TreeNode: React.FC<{ node: HierarchyTreeNode; depth: number }> = ({ node, depth }) => {
    const hasChildren = node.children && node.children.length > 0;
    const leafChildrenCount = countLeafChildren(node);
    const hasLeafChildren = leafChildrenCount > 0;
    const indent = depth * 16;
    
    // Get the current collapsed state
    const isCurrentlyCollapsed = collapsedContainers.has(node.id);
    
    // A container is collapsible if it has either child containers OR leaf children
    const isCollapsible = hasChildren || hasLeafChildren;
    
    // Truncate the label for display but keep original for tooltip
    const fullLabel = node.label;
    const displayLabel = truncateLabel(fullLabel, maxLabelLength);
    const showTooltip = fullLabel !== displayLabel;

    const nodeStyle: React.CSSProperties = {
      marginLeft: `${indent}px`,
      cursor: isCollapsible ? 'pointer' : 'default',
      padding: '2px 4px',
      borderRadius: '2px',
      fontSize: '10px',
      display: 'flex',
      alignItems: 'center',
      transition: 'background-color 0.15s ease',
    };

    const toggleStyle: React.CSSProperties = {
      marginRight: '6px',
      fontSize: '9px',
      color: isCollapsible ? COMPONENT_COLORS.TEXT_SECONDARY : COMPONENT_COLORS.TEXT_DISABLED,
      width: '10px',
      textAlign: 'center',
    };

    const labelStyle: React.CSSProperties = {
      color: COMPONENT_COLORS.TEXT_PRIMARY,
      flex: 1,
    };

    const countStyle: React.CSSProperties = {
      color: COMPONENT_COLORS.TEXT_SECONDARY,
      fontSize: '9px',
      marginLeft: '4px',
    };

    const leafIndicatorStyle: React.CSSProperties = {
      marginLeft: `${indent + 16}px`,
      padding: '1px 4px',
      fontSize: '9px',
      color: COMPONENT_COLORS.TEXT_TERTIARY,
      fontStyle: 'italic',
      display: 'flex',
      alignItems: 'center',
    };

    const handleClick = () => {
      if (isCollapsible && onToggleContainer) {
        onToggleContainer(node.id);
      }
    };

    return (
      <div key={node.id}>
        <div 
          style={nodeStyle}
          onClick={handleClick}
          onMouseEnter={(e) => {
            if (isCollapsible) {
              e.currentTarget.style.backgroundColor = COMPONENT_COLORS.BUTTON_HOVER_BACKGROUND;
            }
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.backgroundColor = 'transparent';
          }}
          title={showTooltip ? fullLabel : `Container: ${fullLabel}${isCollapsible ? ' (click to toggle)' : ''}`}
        >
          <span style={toggleStyle}>
            {isCollapsible ? (isCurrentlyCollapsed ? '▶' : '▼') : '•'}
          </span>
          <span style={labelStyle}>{displayLabel}</span>
          {showNodeCounts && (hasChildren || hasLeafChildren) && (
            <span style={countStyle}>
              ({hasChildren ? node.children.length : leafChildrenCount})
            </span>
          )}
        </div>
        
        {/* Show nested container children when expanded */}
        {hasChildren && !isCurrentlyCollapsed && (
          <div>
            {node.children.map(child => (
              <TreeNode key={child.id} node={child} depth={depth + 1} />
            ))}
          </div>
        )}
        
        {/* Show leaf children indicator when expanded */}
        {hasLeafChildren && !isCurrentlyCollapsed && !hasChildren && (
          <div 
            style={leafIndicatorStyle}
            title={`${leafChildrenCount} leaf node${leafChildrenCount !== 1 ? 's' : ''}`}
          >
            <span style={toggleStyle}>•</span>
            <span>&lt;{leafChildrenCount} leaf node{leafChildrenCount !== 1 ? 's' : ''}&gt;</span>
          </div>
        )}
      </div>
    );
  };

  if (!hierarchyTree || hierarchyTree.length === 0) {
    return (
      <div className={`hierarchy-tree-empty ${className}`} style={style}>
        <span style={{ 
          color: COMPONENT_COLORS.TEXT_DISABLED,
          fontSize: '10px',
          fontStyle: 'italic'
        }}>
          No hierarchy available
        </span>
      </div>
    );
  }

  return (
    <div className={`hierarchy-tree ${className}`} style={style}>
      {title && (
        <div style={{
          fontSize: '11px',
          fontWeight: 'bold',
          color: COMPONENT_COLORS.TEXT_PRIMARY,
          marginBottom: '8px',
          paddingBottom: '4px',
          borderBottom: `1px solid ${COMPONENT_COLORS.BORDER_LIGHT}`,
        }}>
          {title}
        </div>
      )}
      
      <div>
        {hierarchyTree.map(node => (
          <TreeNode key={node.id} node={node} depth={0} />
        ))}
      </div>
    </div>
  );
}
