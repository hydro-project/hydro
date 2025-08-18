/**
 * Color utilities for the vis system
 * Ported from the original visualizer to ensure consistent color mapping
 */
/**
 * Generate node colors dynamically based on provided node type configuration
 * @param nodeType - The node type to get colors for
 * @param paletteKey - The color palette to use
 * @param nodeTypeConfig - Configuration object with node type mappings
 * @returns Color configuration for the node type
 */
export declare function generateNodeColors(nodeType: string, paletteKey?: string, nodeTypeConfig?: any): {
    primary: any;
    secondary: any;
    border: string;
    gradient: string;
};
//# sourceMappingURL=colorUtils.d.ts.map