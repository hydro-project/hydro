import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
/**
 * @fileoverview InfoPanel Component
 *
 * Combined info panel that displays grouping controls, legend, and container hierarchy
 * with collapsible sections for organizing the interface.
 */
import { useState, useMemo } from 'react';
import { DockablePanel, PANEL_POSITIONS } from './DockablePanel';
import { CollapsibleSection } from './CollapsibleSection';
import { GroupingControls } from './GroupingControls';
import { HierarchyTree } from './HierarchyTree';
import { Legend } from './Legend';
import { COMPONENT_COLORS } from '../shared/config';
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
        // Create a map of container ID to container info with proper parent detection
        const containerMap = new Map();
        containers.forEach(container => {
            // Find parent by checking which container has this container as a child
            let parentId = null;
            for (const potentialParent of containers) {
                if (potentialParent.id !== container.id && potentialParent.children && potentialParent.children.has && potentialParent.children.has(container.id)) {
                    parentId = potentialParent.id;
                    break;
                }
            }
            containerMap.set(container.id, {
                id: container.id,
                label: container.data?.label || container.label || container.id, // Try data.label, then label, then fallback to id
                children: [],
                nodeCount: container.children ? container.children.size : 0,
                parentId: parentId,
            });
        });
        // Recursively build tree structure 
        const buildTree = (parentId) => {
            const children = [];
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
export { Legend } from './Legend';
export { HierarchyTree } from './HierarchyTree';
export { GroupingControls } from './GroupingControls';
export { CollapsibleSection } from './CollapsibleSection';
export { DockablePanel, PANEL_POSITIONS } from './DockablePanel';
//# sourceMappingURL=InfoPanel.js.map