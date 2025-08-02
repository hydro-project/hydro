/**
 * @fileoverview Node Styling Utilities
 * 
 * Applies consistent node styling with nodeTypeConfig support,
 * similar to the original visualizer approach.
 */

import { Node } from '@xyflow/react';
import { generateNodeColors } from '../shared/colorUtils';
import { DEFAULT_NODE_STYLE, COMPONENT_COLORS, TYPOGRAPHY, SHADOWS } from '../shared/config';

/**
 * Apply styling to ReactFlow nodes based on nodeType and nodeTypeConfig
 */
export function applyNodeStyling(
  nodes: Node[], 
  colorPalette: string = 'Set2', // Use Set2 for better contrast than Set3
  nodeTypeConfig: any = null
): Node[] {
  const styledNodes = nodes.map(node => {
    // Skip container nodes - they already have their styling
    if (node.type === 'container') {
      return node;
    }

    // Get the nodeType from the node data
    const nodeType = (node.data as any)?.nodeType || 'Transform';
    
    // Generate colors using the same logic as the original visualizer
    const nodeColors = generateNodeColors(nodeType, colorPalette, nodeTypeConfig);
    
    // Apply styling similar to the visualizer's createStyledNode with improved legibility
    const styledNode = {
      ...node,
      style: {
        // SIMPLE TEST: Just apply basic background color
        backgroundColor: nodeColors.primary,
        color: '#ffffff',
        padding: '8px 12px',
        borderRadius: '4px',
        border: `2px solid ${nodeColors.border}`,
        fontSize: '12px',
        fontWeight: '600'
      }
    };
    
    return styledNode;
  });
  
  return styledNodes;
}
