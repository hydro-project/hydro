/**
 * @fileoverview Node Styling Utilities
 *
 * Applies consistent node styling with nodeTypeConfig support,
 * similar to the original visualizer approach.
 */
import { generateNodeColors } from '../shared/colorUtils.js';
import { DEFAULT_NODE_STYLE, COMPONENT_COLORS } from '../shared/config.js';
/**
 * Apply styling to ReactFlow nodes based on nodeType and nodeTypeConfig
 */
export function applyNodeStyling(nodes, colorPalette = 'Set3', nodeTypeConfig = null) {
    return nodes.map(node => {
        // Skip container nodes - they already have their styling
        if (node.type === 'container') {
            return node;
        }
        // Get the nodeType from the node data
        const nodeType = node.data?.nodeType || 'Transform';
        // Generate colors using the same logic as the original visualizer
        const nodeColors = generateNodeColors(nodeType, colorPalette, nodeTypeConfig);
        // Apply styling similar to the visualizer's createStyledNode
        return {
            ...node,
            style: {
                ...DEFAULT_NODE_STYLE,
                ...node.style,
                background: nodeColors.gradient,
                color: COMPONENT_COLORS.TEXT_INVERSE,
                border: 'none',
                borderRadius: '6px',
                fontWeight: '500',
                boxShadow: 'none'
            }
        };
    });
}
//# sourceMappingURL=nodeStyler.js.map