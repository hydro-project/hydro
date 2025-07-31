/**
 * Combined Info Panel Component
 * 
 * Displays grouping controls, legend and container hierarchy with collapsible sections
 */

import React, { useState, useMemo } from 'react';
import { generateNodeColors, truncateContainerName } from '../utils/utils.js';
import { DockablePanel, DOCK_POSITIONS } from './DockablePanel.js';
import { GroupingControls } from './GroupingControls.js';
import { COMPONENT_COLORS } from '../utils/constants.js';
import styles from '../../../pages/visualizer.module.css';

export function InfoPanel({ 
  colorPalette = 'Set3', 
  graphData, 
  nodes, 
  collapsedContainers,
  onToggleContainer,
  childNodesByParent,
  onPositionChange,
  // New props for grouping
  hierarchyChoices,
  currentGrouping,
  onGroupingChange
}) {
  const [legendCollapsed, setLegendCollapsed] = useState(true);
  const [hierarchyCollapsed, setHierarchyCollapsed] = useState(false);
  const [groupingCollapsed, setGroupingCollapsed] = useState(false);

  // Get legend data from the graph JSON, fallback to default if not provided
  const legendData = graphData?.legend || {
    title: "Legend",
    items: [
      { type: "Source", label: "Source" },
      { type: "Transform", label: "Transform" },
      { type: "Sink", label: "Sink" },
      { type: "Network", label: "Network" },
      { type: "Aggregation", label: "Aggregation" },
      { type: "Join", label: "Join" },
      { type: "Tee", label: "Tee" }
    ]
  };

  // Get the current grouping name for the section title
  const currentGroupingName = hierarchyChoices?.find(choice => choice.id === currentGrouping)?.name || 'Container';

  // Build hierarchy tree structure from nodes
  const hierarchyTree = useMemo(() => {
    if (!nodes || nodes.length === 0) {
      return [];
    }

    // Get only container nodes (group type)
    const containerNodes = nodes.filter(node => node.type === 'group');
    
    if (containerNodes.length === 0) {
      return [];
    }

    // Build tree structure
    const buildTree = (parentId = null) => {
      return containerNodes
        .filter(node => {
          if (parentId === null) {
            // Root level containers (no parent or parent not in containerNodes)
            return !node.parentId || !containerNodes.some(c => c.id === node.parentId);
          }
          return node.parentId === parentId;
        })
        .map(node => ({
          id: node.id,
          label: node.data?.label || node.id,
          children: buildTree(node.id),
          nodeCount: childNodesByParent?.get(node.id)?.size || 0
        }));
    };

    const tree = buildTree();
    return tree;
  }, [nodes, childNodesByParent]);

  // Count immediate leaf (non-container) children of a container
  const countLeafChildren = (containerId) => {
    const children = childNodesByParent?.get(containerId);
    if (!children) return 0;
    
    return Array.from(children).filter(childId => {
      const childNode = nodes.find(n => n.id === childId);
      return childNode && childNode.type !== 'group';
    }).length;
  };

  const TreeNode = ({ node, depth }) => {
    const indent = depth * 16;
    const hasChildren = node.children && node.children.length > 0;
    const leafChildrenCount = countLeafChildren(node.id);
    
    // Get the current collapsed state directly from the collapsedContainers Set
    // instead of relying on the cached node.isCollapsed value
    const isCurrentlyCollapsed = collapsedContainers?.has(node.id) || false;
    
    // A container is collapsible if it has either child containers OR leaf children
    const isCollapsible = hasChildren || leafChildrenCount > 0;
    
    // Truncate the label for display but keep original for tooltip
    const fullLabel = node.label;
    const displayLabel = truncateContainerName(fullLabel, 15, {
      side: 'left',
      splitOnDelimiter: true,
      delimiterPenalty: 0.2
    });
    const showTooltip = fullLabel !== displayLabel;

    return (
      <div key={node.id}>
        <div 
          className={styles.treeNode}
          style={{ 
            marginLeft: `${indent}px`,
            cursor: isCollapsible ? 'pointer' : 'default'
          }}
          onClick={() => isCollapsible && onToggleContainer?.(node.id)}
          title={showTooltip ? fullLabel : `Container: ${fullLabel}${isCollapsible ? ' (click to toggle)' : ''}`}
        >
          <span className={styles.treeToggle}>
            {isCollapsible ? (isCurrentlyCollapsed ? '▶' : '▼') : '•'}
          </span>
          <span className={styles.treeLabel}>{displayLabel}</span>
          {hasChildren && (
            <span className={styles.treeNodeCount}>
              ({node.children.length})
            </span>
          )}
          {!hasChildren && leafChildrenCount > 0 && (
            <span className={styles.treeNodeCount}>
              ({leafChildrenCount})
            </span>
          )}
        </div>
        
        {hasChildren && !isCurrentlyCollapsed && (
          <div>
            {node.children.map(child => (
              <TreeNode key={child.id} node={child} depth={depth + 1} />
            ))}
          </div>
        )}
        
        {leafChildrenCount > 0 && !isCurrentlyCollapsed && (
          <div 
            className={styles.treeLeafIndicator}
            style={{ marginLeft: `${indent + 16}px` }}
            title={`${leafChildrenCount} leaf node${leafChildrenCount !== 1 ? 's' : ''}`}
          >
            <span className={styles.treeToggle}>•</span>
            <span className={styles.treeLabel}>&lt;leaf&gt;</span>
            <span className={styles.treeNodeCount}>({leafChildrenCount})</span>
          </div>
        )}
      </div>
    );
  };

  const CollapsibleSection = ({ title, isCollapsed, onToggle, children }) => (
    <div style={{ marginBottom: '12px' }}>
      <div 
        style={{
          display: 'flex',
          alignItems: 'center',
          cursor: 'pointer',
          fontSize: '11px',
          fontWeight: 'bold',
          marginBottom: '6px',
          color: COMPONENT_COLORS.TEXT_PRIMARY
        }}
        onClick={onToggle}
      >
        <span style={{ marginRight: '4px', fontSize: '10px' }}>
          {isCollapsed ? '▶' : '▼'}
        </span>
        {title}
      </div>
      {!isCollapsed && (
        <div style={{ paddingLeft: '12px' }}>
          {children}
        </div>
      )}
    </div>
  );

  return (
    <DockablePanel
      id="info"
      title="Graph Info"
      defaultPosition={DOCK_POSITIONS.TOP_RIGHT}
      defaultDocked={true}
      defaultCollapsed={false}
      onPositionChange={onPositionChange}
      minWidth={250}
      minHeight={200}
    >
      <div style={{ fontSize: '10px' }}>
        {/* Grouping & Hierarchy Section */}
        <CollapsibleSection
          title="Grouping & Hierarchy"
          isCollapsed={groupingCollapsed}
          onToggle={() => setGroupingCollapsed(!groupingCollapsed)}
        >
          <div style={{ marginBottom: '8px' }}>
            <GroupingControls
              hierarchyChoices={hierarchyChoices}
              currentGrouping={currentGrouping}
              onGroupingChange={onGroupingChange}
              compact={true}
            />
          </div>
          
          {hierarchyTree.length > 0 && (
            <div>
              <div style={{
                fontSize: '10px',
                fontWeight: 'bold',
                color: COMPONENT_COLORS.TEXT_SECONDARY,
                marginBottom: '4px',
                paddingLeft: '4px'
              }}>
                {currentGroupingName} Hierarchy
              </div>
              {hierarchyTree.map(node => (
                <TreeNode key={node.id} node={node} depth={0} />
              ))}
            </div>
          )}
        </CollapsibleSection>

        <CollapsibleSection
          title={legendData.title}
          isCollapsed={legendCollapsed}
          onToggle={() => setLegendCollapsed(!legendCollapsed)}
        >
          {legendData.items.map(item => {
            const colors = generateNodeColors(item.type, colorPalette, graphData?.nodeTypeConfig);
            return (
              <div key={item.type} style={{
                display: 'flex',
                alignItems: 'center',
                margin: '3px 0',
                fontSize: '10px'
              }}>
                <div 
                  style={{
                    width: '12px',
                    height: '12px',
                    borderRadius: '2px',
                    marginRight: '6px',
                    border: `1px solid ${COMPONENT_COLORS.BORDER_MEDIUM}`,
                    flexShrink: 0,
                    backgroundColor: colors.primary,
                    borderColor: colors.border
                  }}
                />
                <span>{item.label}</span>
              </div>
            );
          })}
        </CollapsibleSection>
      </div>
    </DockablePanel>
  );
}
