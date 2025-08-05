import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
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
export function InfoPanel({ colorPalette = 'Set3', graphData, nodes, collapsedContainers, onToggleContainer, childNodesByParent, onPositionChange, 
// New props for grouping
hierarchyChoices, currentGrouping, onGroupingChange }) {
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
        if (!children)
            return 0;
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
        return (_jsxs("div", { children: [_jsxs("div", { className: styles.treeNode, style: {
                        marginLeft: `${indent}px`,
                        cursor: isCollapsible ? 'pointer' : 'default'
                    }, onClick: () => isCollapsible && onToggleContainer?.(node.id), title: showTooltip ? fullLabel : `Container: ${fullLabel}${isCollapsible ? ' (click to toggle)' : ''}`, children: [_jsx("span", { className: styles.treeToggle, children: isCollapsible ? (isCurrentlyCollapsed ? '▶' : '▼') : '•' }), _jsx("span", { className: styles.treeLabel, children: displayLabel }), hasChildren && (_jsxs("span", { className: styles.treeNodeCount, children: ["(", node.children.length, ")"] })), !hasChildren && leafChildrenCount > 0 && (_jsxs("span", { className: styles.treeNodeCount, children: ["(", leafChildrenCount, ")"] }))] }), hasChildren && !isCurrentlyCollapsed && (_jsx("div", { children: node.children.map(child => (_jsx(TreeNode, { node: child, depth: depth + 1 }, child.id))) })), leafChildrenCount > 0 && !isCurrentlyCollapsed && (_jsxs("div", { className: styles.treeLeafIndicator, style: { marginLeft: `${indent + 16}px` }, title: `${leafChildrenCount} leaf node${leafChildrenCount !== 1 ? 's' : ''}`, children: [_jsx("span", { className: styles.treeToggle, children: "\u2022" }), _jsx("span", { className: styles.treeLabel, children: "<leaf>" }), _jsxs("span", { className: styles.treeNodeCount, children: ["(", leafChildrenCount, ")"] })] }))] }, node.id));
    };
    const CollapsibleSection = ({ title, isCollapsed, onToggle, children }) => (_jsxs("div", { style: { marginBottom: '12px' }, children: [_jsxs("div", { style: {
                    display: 'flex',
                    alignItems: 'center',
                    cursor: 'pointer',
                    fontSize: '11px',
                    fontWeight: 'bold',
                    marginBottom: '6px',
                    color: COMPONENT_COLORS.TEXT_PRIMARY
                }, onClick: onToggle, children: [_jsx("span", { style: { marginRight: '4px', fontSize: '10px' }, children: isCollapsed ? '▶' : '▼' }), title] }), !isCollapsed && (_jsx("div", { style: { paddingLeft: '12px' }, children: children }))] }));
    return (_jsx(DockablePanel, { id: "info", title: "Graph Info", defaultPosition: DOCK_POSITIONS.TOP_RIGHT, defaultDocked: true, defaultCollapsed: false, onPositionChange: onPositionChange, minWidth: 250, minHeight: 200, children: _jsxs("div", { style: { fontSize: '10px' }, children: [_jsxs(CollapsibleSection, { title: "Grouping & Hierarchy", isCollapsed: groupingCollapsed, onToggle: () => setGroupingCollapsed(!groupingCollapsed), children: [_jsx("div", { style: { marginBottom: '8px' }, children: _jsx(GroupingControls, { hierarchyChoices: hierarchyChoices, currentGrouping: currentGrouping, onGroupingChange: onGroupingChange, compact: true }) }), hierarchyTree.length > 0 && (_jsxs("div", { children: [_jsxs("div", { style: {
                                        fontSize: '10px',
                                        fontWeight: 'bold',
                                        color: COMPONENT_COLORS.TEXT_SECONDARY,
                                        marginBottom: '4px',
                                        paddingLeft: '4px'
                                    }, children: [currentGroupingName, " Hierarchy"] }), hierarchyTree.map(node => (_jsx(TreeNode, { node: node, depth: 0 }, node.id)))] }))] }), _jsx(CollapsibleSection, { title: legendData.title, isCollapsed: legendCollapsed, onToggle: () => setLegendCollapsed(!legendCollapsed), children: legendData.items.map(item => {
                        const colors = generateNodeColors(item.type, colorPalette, graphData?.nodeTypeConfig);
                        return (_jsxs("div", { style: {
                                display: 'flex',
                                alignItems: 'center',
                                margin: '3px 0',
                                fontSize: '10px'
                            }, children: [_jsx("div", { style: {
                                        width: '12px',
                                        height: '12px',
                                        borderRadius: '2px',
                                        marginRight: '6px',
                                        border: `1px solid ${COMPONENT_COLORS.BORDER_MEDIUM}`,
                                        flexShrink: 0,
                                        backgroundColor: colors.primary,
                                        borderColor: colors.border
                                    } }), _jsx("span", { children: item.label })] }, item.type));
                    }) })] }) }));
}
//# sourceMappingURL=InfoPanel.js.map