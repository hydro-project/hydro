/**
 * @fileoverview InfoPanel Component
 * 
 * Combined info panel that displays grouping controls, legend, and container hierarchy
 * with collapsible sections for organizing the interface.
 */

import React, { useState, useMemo } from 'react';
import { InfoPanelProps, HierarchyTreeNode, LegendData } from './types';
import { DockablePanel, PANEL_POSITIONS } from './DockablePanel';
import { CollapsibleSection } from './CollapsibleSection';
import { GroupingControls } from './GroupingControls';
import { HierarchyTree } from './HierarchyTree';
import { Legend } from './Legend';
import { COMPONENT_COLORS } from '../shared/config';

export function InfoPanel({
  visualizationState,
  legendData,
  hierarchyChoices = [],
  currentGrouping,
  onGroupingChange,
  collapsedContainers = new Set(),
  onToggleContainer,
  onPositionChange,
  colorPalette = 'Set3',
  defaultCollapsed = false,
  className = '',
  style
}: InfoPanelProps) {
  const [legendCollapsed, setLegendCollapsed] = useState(true);
  const [hierarchyCollapsed, setHierarchyCollapsed] = useState(false);
  const [groupingCollapsed, setGroupingCollapsed] = useState(false);

  // Get default legend data if none provided
  const defaultLegendData: LegendData = {
    title: "Node Types",
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

  const effectiveLegendData = legendData || defaultLegendData;

  // Get the current grouping name for the section title
  const currentGroupingName = hierarchyChoices?.find(choice => choice.id === currentGrouping)?.name || 'Container';

  // Build hierarchy tree structure from visualization state
  const hierarchyTree = useMemo((): HierarchyTreeNode[] => {
    if (!visualizationState) {
      return [];
    }

    const containers = visualizationState.visibleContainers;
    if (containers.length === 0) {
      return [];
    }

    // Create a map of container ID to container info
    const containerMap = new Map();
    containers.forEach(container => {
      const children = visualizationState.getContainerChildren(container.id);
      containerMap.set(container.id, {
        id: container.id,
        label: container.id, // Could extract label from container data if available
        children: [],
        nodeCount: children ? children.size : 0,
        parentId: null // This would need to be determined from container hierarchy
      });
    });

    // Build tree structure - for now, assume flat structure
    // In a real implementation, you'd need to determine parent-child relationships
    const buildTree = (parentId: string | null = null): HierarchyTreeNode[] => {
      return Array.from(containerMap.values())
        .filter((container: any) => container.parentId === parentId)
        .map((container: any) => ({
          id: container.id,
          label: container.label,
          children: buildTree(container.id),
          nodeCount: container.nodeCount
        }));
    };

    return buildTree();
  }, [visualizationState, collapsedContainers]);

  // Count immediate leaf (non-container) children of a container
  const countLeafChildren = (containerId: string): number => {
    const children = visualizationState?.getContainerChildren(containerId);
    if (!children) return 0;
    
    // Count children that are not containers themselves
    let leafCount = 0;
    children.forEach(childId => {
      const isContainer = visualizationState.getContainer(childId) !== undefined;
      if (!isContainer) {
        leafCount++;
      }
    });
    return leafCount;
  };

  return (
    <DockablePanel
      id="info"
      title="Graph Info"
      defaultPosition={PANEL_POSITIONS.TOP_LEFT}
      defaultDocked={true}
      defaultCollapsed={defaultCollapsed}
      onPositionChange={onPositionChange}
      minWidth={250}
      minHeight={200}
      className={className}
      style={style}
    >
      <div style={{ fontSize: '10px' }}>
        {/* Grouping & Hierarchy Section */}
        {(hierarchyChoices.length > 0 || hierarchyTree.length > 0) && (
          <CollapsibleSection
            title="Grouping & Hierarchy"
            isCollapsed={groupingCollapsed}
            onToggle={() => setGroupingCollapsed(!groupingCollapsed)}
          >
            {/* Grouping Controls */}
            {hierarchyChoices.length > 0 && (
              <div style={{ marginBottom: '8px' }}>
                <GroupingControls
                  hierarchyChoices={hierarchyChoices}
                  currentGrouping={currentGrouping}
                  onGroupingChange={onGroupingChange}
                  compact={true}
                />
              </div>
            )}
            
            {/* Hierarchy Tree */}
            {hierarchyTree.length > 0 && (
              <HierarchyTree
                hierarchyTree={hierarchyTree}
                collapsedContainers={collapsedContainers}
                onToggleContainer={onToggleContainer}
                title={`${currentGroupingName} Hierarchy`}
                showNodeCounts={true}
                truncateLabels={true}
                maxLabelLength={15}
              />
            )}
          </CollapsibleSection>
        )}

        {/* Legend Section */}
        <CollapsibleSection
          title={effectiveLegendData.title}
          isCollapsed={legendCollapsed}
          onToggle={() => setLegendCollapsed(!legendCollapsed)}
        >
          <Legend
            legendData={effectiveLegendData}
            colorPalette={colorPalette}
            compact={true}
          />
        </CollapsibleSection>

        {/* Graph Statistics */}
        {visualizationState && (
          <CollapsibleSection
            title="Statistics"
            isCollapsed={true}
            onToggle={() => {}} // Could add state if needed
          >
            <div style={{ fontSize: '9px', color: COMPONENT_COLORS.TEXT_SECONDARY }}>
              <div style={{ margin: '2px 0' }}>
                Nodes: {visualizationState.visibleNodes.length}
              </div>
              <div style={{ margin: '2px 0' }}>
                Edges: {visualizationState.visibleEdges.length}
              </div>
              <div style={{ margin: '2px 0' }}>
                Containers: {visualizationState.visibleContainers.length}
              </div>
              <div style={{ margin: '2px 0' }}>
                Collapsed: {collapsedContainers.size}
              </div>
            </div>
          </CollapsibleSection>
        )}
      </div>
    </DockablePanel>
  );
}

// Re-export sub-components for individual use
export { Legend } from './Legend';
export { HierarchyTree } from './HierarchyTree';
export { GroupingControls } from './GroupingControls';
export { CollapsibleSection } from './CollapsibleSection';
export { DockablePanel, PANEL_POSITIONS } from './DockablePanel';
