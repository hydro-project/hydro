/**
 * @fileoverview Color utility functions
 *
 * Simple color utilities for the visualization system.
 */
export declare function hexToRgb(hex: string): {
    r: number;
    g: number;
    b: number;
} | null;
export declare function rgbToHex(r: number, g: number, b: number): string;
export declare function getContrastColor(backgroundColor: string): string;
export declare function generateNodeColors(nodeTypes: string[], palette?: string, nodeTypeConfig?: any): Record<string, string>;
//# sourceMappingURL=colorUtils.d.ts.map