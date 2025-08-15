/**
 * @fileoverview HierarchyTree Component
 * 
 * Displays an interactive tree view of container hierarchy for navigation using Ant Design Tree.
 */

import React, { useMemo } from 'react';
import { Tree } from 'antd';
import type { TreeDataNode } from 'antd';
import { HierarchyTreeProps, HierarchyTreeNode } from './types';
import { TYPOGRAPHY } from '../shared/config';
import { COMPONENT_COLORS } from '../shared/config';
import { truncateLabel } from '../shared/textUtils';

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

  // Convert HierarchyTreeNode to Ant Design TreeDataNode format
  const convertToTreeData = (nodes: HierarchyTreeNode[]): TreeDataNode[] => {
    return nodes.map(node => {
      const truncatedLabel = truncateLabels 
        ? truncateLabel(node.label, { maxLength: maxLabelLength, leftTruncate: true })
        : node.label;
      const leafChildrenCount = Math.max(0, node.nodeCount - (node.children?.length || 0));
      const hasChildren = node.children && node.children.length > 0;
      const hasLeafChildren = leafChildrenCount > 0;
      
      // Create display title with better formatting
      let displayTitle: React.ReactNode = truncatedLabel;
      if (showNodeCounts && (hasChildren || hasLeafChildren)) {
        const count = hasChildren ? node.children.length : leafChildrenCount;
        const countText = hasChildren ? `${count} containers` : `${count} nodes`;
        displayTitle = (
          <span style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', width: '100%' }}>
            <span style={{ 
              fontWeight: hasChildren ? '500' : '400',
              color: COMPONENT_COLORS.TEXT_PRIMARY, // Same color for both container and leaf nodes
              ...(hasLeafChildren && !hasChildren ? {
                // Subtle gray background for leaf nodes instead of bullet
                backgroundColor: 'rgba(0, 0, 0, 0.04)',
                padding: '1px 4px',
                borderRadius: '3px',
                fontSize: TYPOGRAPHY.INFOPANEL_HIERARCHY_DETAILS
              } : {})
            }}>
              {truncatedLabel}
            </span>
            <span style={{ 
              fontSize: TYPOGRAPHY.INFOPANEL_HIERARCHY_DETAILS, 
              color: COMPONENT_COLORS.TEXT_TERTIARY,
              fontStyle: 'italic',
              marginLeft: '8px',
              opacity: 0.7
            }}>
              {countText}
            </span>
          </span>
        );
      } else {
        // For nodes without counts, still apply leaf styling if needed
        if (hasLeafChildren && !hasChildren) {
          displayTitle = (
            <span style={{
              color: COMPONENT_COLORS.TEXT_PRIMARY,
              backgroundColor: 'rgba(0, 0, 0, 0.04)',
              padding: '1px 4px',
              borderRadius: '3px',
              fontSize: TYPOGRAPHY.INFOPANEL_HIERARCHY_DETAILS
            }}>
              {truncatedLabel}
            </span>
          );
        }
      }

      return {
        key: node.id,
        title: displayTitle,
        children: hasChildren ? convertToTreeData(node.children) : undefined,
        isLeaf: !hasChildren, // Explicitly mark leaf nodes
        // Add custom properties for styling
        data: {
          originalLabel: node.label,
          truncatedLabel,
          nodeCount: node.nodeCount,
          leafChildrenCount,
          hasLeafChildren: hasLeafChildren && !hasChildren,
          isContainer: hasChildren
        }
      };
    });
  };

  // Convert collapsed containers Set to expanded keys array (Ant Design uses expanded, not collapsed)
  const expandedKeys = useMemo(() => {
    const allKeys: string[] = [];
    
    const collectKeys = (nodes: HierarchyTreeNode[]) => {
      nodes.forEach(node => {
        allKeys.push(node.id);
        if (node.children) {
          collectKeys(node.children);
        }
      });
    };
    
    collectKeys(hierarchyTree || []);
    
    // Return keys that are NOT in collapsedContainers (i.e., expanded keys)
    return allKeys.filter(key => !collapsedContainers.has(key));
  }, [hierarchyTree, collapsedContainers]);

  const treeData = useMemo(() => {
    return convertToTreeData(hierarchyTree || []);
  }, [hierarchyTree, maxLabelLength, showNodeCounts, truncateLabels]);

  const handleExpand = (expandedKeys: React.Key[], info: any) => {
    if (onToggleContainer && info.node) {
      // Ant Design expansion is the opposite of our collapse state
      const nodeKey = info.node.key as string;
      onToggleContainer(nodeKey);
    }
  };

  if (!hierarchyTree || hierarchyTree.length === 0) {
    return (
      <div className={`hierarchy-tree-empty ${className}`} style={style}>
        <span style={{ 
          color: COMPONENT_COLORS.TEXT_DISABLED,
          fontSize: TYPOGRAPHY.INFOPANEL_HIERARCHY_DETAILS,
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
          fontSize: TYPOGRAPHY.INFOPANEL_HIERARCHY_NODE,
          fontWeight: 'bold',
          color: COMPONENT_COLORS.TEXT_PRIMARY,
          marginBottom: '8px',
          paddingBottom: '4px',
          borderBottom: `1px solid ${COMPONENT_COLORS.BORDER_LIGHT}`,
        }}>
          {title}
        </div>
      )}
      
      <Tree
        treeData={treeData}
        expandedKeys={expandedKeys}
        onExpand={handleExpand}
        showLine={{ 
          showLeafIcon: false // No leaf icons, just clean lines
        }}
        showIcon={false}
        blockNode
        style={{
          fontSize: TYPOGRAPHY.INFOPANEL_HIERARCHY_NODE,
          color: COMPONENT_COLORS.TEXT_PRIMARY,
          backgroundColor: 'transparent'
        }}
        // Enhanced styling through CSS variables for better hierarchy
        rootStyle={{
          '--antd-tree-node-hover-bg': COMPONENT_COLORS.BUTTON_HOVER_BACKGROUND,
          '--antd-tree-node-selected-bg': COMPONENT_COLORS.BUTTON_HOVER_BACKGROUND,
          '--antd-tree-indent-size': '16px', // Reduce indent for better space usage
          '--antd-tree-node-padding': '2px 4px', // Better padding
        } as React.CSSProperties}
      />
    </div>
  );
}
