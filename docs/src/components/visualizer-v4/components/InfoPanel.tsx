/**
 * @fileoverview InfoPanel Component
 * 
 * Combined info panel that displays grouping controls, legend, and container hierarchy
 * with collapsible sections for organizing the interface.
 */

import React, { useState, useMemo } from 'react';
import { InfoPanelProps, HierarchyTreeNode, LegendData } from './types';
import { PANEL_POSITIONS } from './types';
import { DockablePanel } from './DockablePanel';
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
  
  // Ensure hierarchyChoices is always an array
  const safeHierarchyChoices = Array.isArray(hierarchyChoices) ? hierarchyChoices : [];

  // Get the current grouping name for the section title
  const currentGroupingName = safeHierarchyChoices.find(choice => choice.id === currentGrouping)?.name || 'Container';

    // Build hierarchy tree structure from visualization state
  const hierarchyTree = useMemo((): HierarchyTreeNode[] => {
    if (!visualizationState) {
      return [];
    }

    const containers = visualizationState.visibleContainers;
    if (containers.length === 0) {
      return [];
    }

    // Create a map of container ID to container info with proper parent detection
    const containerMap = new Map<string, HierarchyTreeNode & { parentId: string | null }>();
    
    containers.forEach(container => {
      // Find parent by checking which container has this container as a child
      let parentId: string | null = null;
      for (const potentialParent of containers) {
        if (potentialParent.id !== container.id && potentialParent.children && potentialParent.children.has && potentialParent.children.has(container.id)) {
          parentId = potentialParent.id;
          break;
        }
      }
      
      containerMap.set(container.id, {
        id: container.id,
        label: (container as any).data?.label || (container as any).label || container.id, // Try data.label, then label, then fallback to id
        children: [],
        nodeCount: container.children ? container.children.size : 0,
        parentId: parentId,
      });
    });

    // Recursively build tree structure 
    const buildTree = (parentId: string | null): HierarchyTreeNode[] => {
      const children: HierarchyTreeNode[] = [];
      for (const container of containerMap.values()) {
        if (container.parentId === parentId) {
          const grandchildren = buildTree(container.id);
          children.push({
            id: container.id,
            label: container.label,
            children: grandchildren,
            nodeCount: container.nodeCount
          });
        }
      }
      return children;
    };

    return buildTree(null); // Start with root containers (no parent)
  }, [visualizationState, collapsedContainers]); // Add collapsedContainers as dependency

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
        {(safeHierarchyChoices.length > 0 || hierarchyTree.length > 0) && (
          <CollapsibleSection
            title="Grouping & Hierarchy"
            isCollapsed={groupingCollapsed}
            onToggle={() => setGroupingCollapsed(!groupingCollapsed)}
          >
            {/* Grouping Controls */}
            {safeHierarchyChoices.length > 0 && (
              <div style={{ marginBottom: '8px' }}>
                <GroupingControls
                  hierarchyChoices={safeHierarchyChoices}
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
