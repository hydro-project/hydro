/**
 * Create invisible handle style for seamless edge connections
 * Handles are positioned at the exact border with no visual appearance
 * Using zero offset for perfect edge-to-node connection
 */
export function createInvisibleHandleStyle(position: any): {
    background: string;
    border: string;
    width: number;
    height: number;
    opacity: number;
    zIndex: number;
    pointerEvents: string;
    borderRadius: string;
};
/**
 * Standard handle configuration for container nodes
 * Returns array of handle props for consistent application
 */
export function getContainerHandles(): {
    type: string;
    position: Position;
    id: string;
    style: {
        background: string;
        border: string;
        width: number;
        height: number;
        opacity: number;
        zIndex: number;
        pointerEvents: string;
        borderRadius: string;
    };
}[];
/**
 * Render handles using the standard configuration
 * Use this in both GroupNode and CollapsedContainerNode
 */
export function renderContainerHandles(HandleComponent: any): any[];
import { Position } from '@xyflow/react';
//# sourceMappingURL=handleStyles.d.ts.map