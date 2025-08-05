import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
/**
 * Hierarchy Tree Component for Graph Visualizer
 *
 * Displays an interactive tree view of container hierarchy for navigation
 */
import React, { useMemo } from 'react';
import { DockablePanel, DOCK_POSITIONS } from './DockablePanel.js';
import styles from '../../../pages/visualizer.module.css';
export function HierarchyTree({ nodes, collapsedContainers, onToggleContainer, childNodesByParent, onPositionChange }) {
    // Build hierarchy tree structure from nodes
    const hierarchyTree = useMemo(() => {
        if (!nodes || nodes.length === 0)
            return [];
        // Get only container nodes (group type)
        const containerNodes = nodes.filter(node => node.type === 'group');
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
                isCollapsed: collapsedContainers.has(node.id),
                children: buildTree(node.id),
                nodeCount: childNodesByParent.get(node.id)?.size || 0
            }));
        };
        return buildTree();
    }, [nodes, collapsedContainers, childNodesByParent]);
    // Count immediate leaf (non-container) children of a container
    const countLeafChildren = (containerId) => {
        const children = childNodesByParent.get(containerId);
        if (!children)
            return 0;
        let leafCount = 0;
        children.forEach(childId => {
            const childNode = nodes.find(n => n.id === childId);
            if (childNode && childNode.type !== 'group') {
                leafCount++;
            }
        });
        return leafCount;
    };
    // Recursive tree node component
    const TreeNode = ({ node, depth = 0 }) => {
        const hasChildren = node.children && node.children.length > 0;
        const leafChildrenCount = countLeafChildren(node.id);
        const hasLeafChildren = leafChildrenCount > 0;
        const indent = depth * 16; // Increased indentation for better nesting visualization
        return (_jsxs("div", { children: [_jsxs("div", { className: `${styles.treeNode} ${node.isCollapsed ? styles.treeNodeCollapsed : styles.treeNodeExpanded}`, style: { marginLeft: `${indent}px` }, onClick: () => onToggleContainer(node.id), title: `${node.isCollapsed ? 'Expand' : 'Collapse'} container: ${node.label}`, children: [_jsx("span", { className: styles.treeToggle, children: hasChildren || hasLeafChildren ? (node.isCollapsed ? '▶' : '▼') : '•' }), _jsx("span", { className: styles.treeLabel, children: node.label }), _jsx("span", { className: styles.treeNodeCount, children: node.nodeCount > 0 ? `(${node.nodeCount})` : '' })] }), hasChildren && !node.isCollapsed && (_jsx("div", { className: styles.treeChildren, children: node.children.map(child => (_jsx(TreeNode, { node: child, depth: depth + 1 }, child.id))) })), hasLeafChildren && !node.isCollapsed && !hasChildren && (_jsxs("div", { className: styles.treeLeafIndicator, style: { marginLeft: `${indent + 16}px` }, title: `${leafChildrenCount} leaf node${leafChildrenCount !== 1 ? 's' : ''}`, children: [_jsx("span", { className: styles.treeToggle, children: "\u2022" }), _jsx("span", { className: styles.treeLabel, children: "<leaf>" }), _jsxs("span", { className: styles.treeNodeCount, children: ["(", leafChildrenCount, ")"] })] }))] }, node.id));
    };
    if (hierarchyTree.length === 0) {
        return null; // No containers to show
    }
    return (_jsx(DockablePanel, { id: "hierarchy", title: "Container Hierarchy", defaultPosition: DOCK_POSITIONS.BOTTOM_RIGHT, defaultDocked: true, defaultCollapsed: false, onPositionChange: onPositionChange, minWidth: 250, minHeight: 200, children: _jsx("div", { children: hierarchyTree.map(node => (_jsx(TreeNode, { node: node, depth: 0 }, node.id))) }) }));
}
//# sourceMappingURL=HierarchyTree.js.map