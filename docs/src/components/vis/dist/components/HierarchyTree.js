import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { COMPONENT_COLORS } from '../shared/config.js';
export function HierarchyTree({ hierarchyTree, collapsedContainers = new Set(), onToggleContainer, title = 'Container Hierarchy', showNodeCounts = true, truncateLabels = true, maxLabelLength = 20, className = '', style }) {
    // Utility function to truncate labels
    const truncateLabel = (label, maxLength) => {
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
    const countLeafChildren = (node) => {
        // This would need to be passed from parent or calculated differently
        // For now, we'll use the nodeCount from the tree structure
        return Math.max(0, node.nodeCount - node.children.length);
    };
    // Recursive tree node component
    const TreeNode = ({ node, depth }) => {
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
        const nodeStyle = {
            marginLeft: `${indent}px`,
            cursor: isCollapsible ? 'pointer' : 'default',
            padding: '2px 4px',
            borderRadius: '2px',
            fontSize: '10px',
            display: 'flex',
            alignItems: 'center',
            transition: 'background-color 0.15s ease',
        };
        const toggleStyle = {
            marginRight: '6px',
            fontSize: '9px',
            color: isCollapsible ? COMPONENT_COLORS.TEXT_SECONDARY : COMPONENT_COLORS.TEXT_DISABLED,
            width: '10px',
            textAlign: 'center',
        };
        const labelStyle = {
            color: COMPONENT_COLORS.TEXT_PRIMARY,
            flex: 1,
        };
        const countStyle = {
            color: COMPONENT_COLORS.TEXT_SECONDARY,
            fontSize: '9px',
            marginLeft: '4px',
        };
        const leafIndicatorStyle = {
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
        return (_jsxs("div", { children: [_jsxs("div", { style: nodeStyle, onClick: handleClick, onMouseEnter: (e) => {
                        if (isCollapsible) {
                            e.currentTarget.style.backgroundColor = COMPONENT_COLORS.BUTTON_HOVER_BACKGROUND;
                        }
                    }, onMouseLeave: (e) => {
                        e.currentTarget.style.backgroundColor = 'transparent';
                    }, title: showTooltip ? fullLabel : `Container: ${fullLabel}${isCollapsible ? ' (click to toggle)' : ''}`, children: [_jsx("span", { style: toggleStyle, children: isCollapsible ? (isCurrentlyCollapsed ? '▶' : '▼') : '•' }), _jsx("span", { style: labelStyle, children: displayLabel }), showNodeCounts && (hasChildren || hasLeafChildren) && (_jsxs("span", { style: countStyle, children: ["(", hasChildren ? node.children.length : leafChildrenCount, ")"] }))] }), hasChildren && !isCurrentlyCollapsed && (_jsx("div", { children: node.children.map(child => (_jsx(TreeNode, { node: child, depth: depth + 1 }, child.id))) })), hasLeafChildren && !isCurrentlyCollapsed && !hasChildren && (_jsxs("div", { style: leafIndicatorStyle, title: `${leafChildrenCount} leaf node${leafChildrenCount !== 1 ? 's' : ''}`, children: [_jsx("span", { style: toggleStyle, children: "\u2022" }), _jsxs("span", { children: ["<", leafChildrenCount, " leaf node", leafChildrenCount !== 1 ? 's' : '', ">"] })] }))] }, node.id));
    };
    if (!hierarchyTree || hierarchyTree.length === 0) {
        return (_jsx("div", { className: `hierarchy-tree-empty ${className}`, style: style, children: _jsx("span", { style: {
                    color: COMPONENT_COLORS.TEXT_DISABLED,
                    fontSize: '10px',
                    fontStyle: 'italic'
                }, children: "No hierarchy available" }) }));
    }
    return (_jsxs("div", { className: `hierarchy-tree ${className}`, style: style, children: [title && (_jsx("div", { style: {
                    fontSize: '11px',
                    fontWeight: 'bold',
                    color: COMPONENT_COLORS.TEXT_PRIMARY,
                    marginBottom: '8px',
                    paddingBottom: '4px',
                    borderBottom: `1px solid ${COMPONENT_COLORS.BORDER_LIGHT}`,
                }, children: title })), _jsx("div", { children: hierarchyTree.map(node => (_jsx(TreeNode, { node: node, depth: 0 }, node.id))) })] }));
}
//# sourceMappingURL=HierarchyTree.js.map