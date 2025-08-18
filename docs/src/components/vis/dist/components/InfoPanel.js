import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
/**
 * @fileoverview InfoPanel Component
 *
 * Combined info panel that displays grouping controls, legend, and container hierarchy
 * with collapsible sections for organizing the interface.
 */
import { useState, useMemo } from 'react';
import { DockablePanel, PANEL_POSITIONS } from './DockablePanel.js';
import { CollapsibleSection } from './CollapsibleSection.js';
import { GroupingControls } from './GroupingControls.js';
import { HierarchyTree } from './HierarchyTree.js';
import { Legend } from './Legend.js';
import { COMPONENT_COLORS } from '../shared/config.js';
export function InfoPanel({ visualizationState, legendData, hierarchyChoices = [], currentGrouping, onGroupingChange, collapsedContainers = new Set(), onToggleContainer, onPositionChange, colorPalette = 'Set3', defaultCollapsed = false, className = '', style }) {
    const [legendCollapsed, setLegendCollapsed] = useState(true);
    const [hierarchyCollapsed, setHierarchyCollapsed] = useState(false);
    const [groupingCollapsed, setGroupingCollapsed] = useState(false);
    // Get default legend data if none provided
    const defaultLegendData = {
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
    const hierarchyTree = useMemo(() => {
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
        const buildTree = (parentId = null) => {
            return Array.from(containerMap.values())
                .filter((container) => container.parentId === parentId)
                .map((container) => ({
                id: container.id,
                label: container.label,
                children: buildTree(container.id),
                nodeCount: container.nodeCount
            }));
        };
        return buildTree();
    }, [visualizationState, collapsedContainers]);
    // Count immediate leaf (non-container) children of a container
    const countLeafChildren = (containerId) => {
        const children = visualizationState?.getContainerChildren(containerId);
        if (!children)
            return 0;
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
    return (_jsx(DockablePanel, { id: "info", title: "Graph Info", defaultPosition: PANEL_POSITIONS.TOP_LEFT, defaultDocked: true, defaultCollapsed: defaultCollapsed, onPositionChange: onPositionChange, minWidth: 250, minHeight: 200, className: className, style: style, children: _jsxs("div", { style: { fontSize: '10px' }, children: [(hierarchyChoices.length > 0 || hierarchyTree.length > 0) && (_jsxs(CollapsibleSection, { title: "Grouping & Hierarchy", isCollapsed: groupingCollapsed, onToggle: () => setGroupingCollapsed(!groupingCollapsed), children: [hierarchyChoices.length > 0 && (_jsx("div", { style: { marginBottom: '8px' }, children: _jsx(GroupingControls, { hierarchyChoices: hierarchyChoices, currentGrouping: currentGrouping, onGroupingChange: onGroupingChange, compact: true }) })), hierarchyTree.length > 0 && (_jsx(HierarchyTree, { hierarchyTree: hierarchyTree, collapsedContainers: collapsedContainers, onToggleContainer: onToggleContainer, title: `${currentGroupingName} Hierarchy`, showNodeCounts: true, truncateLabels: true, maxLabelLength: 15 }))] })), _jsx(CollapsibleSection, { title: effectiveLegendData.title, isCollapsed: legendCollapsed, onToggle: () => setLegendCollapsed(!legendCollapsed), children: _jsx(Legend, { legendData: effectiveLegendData, colorPalette: colorPalette, compact: true }) }), visualizationState && (_jsx(CollapsibleSection, { title: "Statistics", isCollapsed: true, onToggle: () => { }, children: _jsxs("div", { style: { fontSize: '9px', color: COMPONENT_COLORS.TEXT_SECONDARY }, children: [_jsxs("div", { style: { margin: '2px 0' }, children: ["Nodes: ", visualizationState.visibleNodes.length] }), _jsxs("div", { style: { margin: '2px 0' }, children: ["Edges: ", visualizationState.visibleEdges.length] }), _jsxs("div", { style: { margin: '2px 0' }, children: ["Containers: ", visualizationState.visibleContainers.length] }), _jsxs("div", { style: { margin: '2px 0' }, children: ["Collapsed: ", collapsedContainers.size] })] }) }))] }) }));
}
// Re-export sub-components for individual use
export { Legend } from './Legend.js';
export { HierarchyTree } from './HierarchyTree.js';
export { GroupingControls } from './GroupingControls.js';
export { CollapsibleSection } from './CollapsibleSection.js';
export { DockablePanel, PANEL_POSITIONS } from './DockablePanel.js';
//# sourceMappingURL=InfoPanel.js.map