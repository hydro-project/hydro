/**
 * @fileoverview InfoPanel Component
 * 
 * Combined info panel that displays grouping controls, legend, and container hierarchy
 * with collapsible sections for organizing the interface.
 */

import React, { useState, useMemo } from 'react';
import { InfoPanelProps, HierarchyTreeNode, LegendData } from './types';
import { AntDockablePanel } from './AntDockablePanel';
import { CollapsibleSection } from './CollapsibleSection';
import { GroupingControls } from './GroupingControls';
import { HierarchyTree } from './HierarchyTree';
import { Legend } from './Legend';
import { EdgeStyleLegend } from './EdgeStyleLegend';
import { COMPONENT_COLORS, TYPOGRAPHY } from '../shared/config';

export function InfoPanel({
  visualizationState,
  legendData,
  edgeStyleConfig,
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
  const [legendCollapsed, setLegendCollapsed] = useState(false); // Start expanded so users can see it
  const [edgeStyleCollapsed, setEdgeStyleCollapsed] = useState(false);
  const [hierarchyCollapsed, setHierarchyCollapsed] = useState(false);
  const [groupingCollapsed, setGroupingCollapsed] = useState(false);

  // Get default legend data if none provided
  const defaultLegendData: LegendData = {
    title: "Node Types",
    items: []
  };

  // Generate legend data from actual node types in the visualization
  const generateLegendFromState = (): LegendData => {
    if (!visualizationState) return defaultLegendData;
    
    const nodeTypes = new Set<string>();
    
    // Collect all unique node types from visible nodes
    visualizationState.visibleNodes.forEach(node => {
      const nodeType = (node as any).nodeType || (node as any).style || 'default';
      nodeTypes.add(nodeType);
    });
    
    return {
      title: "Node Types",
      items: Array.from(nodeTypes).map(nodeType => ({
        type: nodeType,
        label: nodeType.charAt(0).toUpperCase() + nodeType.slice(1)
      }))
    };
  };

  // Ensure we always have valid legend data
  const effectiveLegendData = (legendData && legendData.items && Array.isArray(legendData.items)) 
    ? legendData 
    : generateLegendFromState();
  
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
    <AntDockablePanel
      title="Graph Info"
      defaultOpen={!defaultCollapsed}
      placement="left"
      width={300}
      className={className}
      style={style}
    >
      <div style={{ fontSize: TYPOGRAPHY.INFOPANEL_BASE }}> {/* Increased from 10px to 14px */}
        {/* Grouping & Hierarchy Section */}
        {(safeHierarchyChoices.length > 0 || hierarchyTree.length > 0) && (
          <CollapsibleSection
            title="Grouping"
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

        {/* Edge Style Legend Section */}
        {(edgeStyleConfig?.semanticMappings || edgeStyleConfig?.booleanPropertyPairs || edgeStyleConfig?.singlePropertyMappings || edgeStyleConfig?.propertyMappings) && (
          <CollapsibleSection
            title="Edge Styles"
            isCollapsed={edgeStyleCollapsed}
            onToggle={() => setEdgeStyleCollapsed(!edgeStyleCollapsed)}
          >
            <EdgeStyleLegend
              edgeStyleConfig={edgeStyleConfig}
              compact={true}
            />
          </CollapsibleSection>
        )}
      </div>
    </AntDockablePanel>
  );
}

// Re-export sub-components for individual use
export { Legend } from './Legend';
export { HierarchyTree } from './HierarchyTree';
export { GroupingControls } from './GroupingControls';
export { CollapsibleSection } from './CollapsibleSection';
export { AntDockablePanel } from './AntDockablePanel';
