/**
 * @fileoverview Coordinate System Translator
 *
 * Handles translation between different coordinate systems used by ELK and ReactFlow.
 *
 * CANONICAL COORDINATE SYSTEM: ELK
 * - VisState stores positions in ELK format (absolute coordinates)
 * - All layout calculations use ELK coordinates
 * - ReactFlow translation happens only when rendering
 *
 * KEY DIFFERENCES:
 * - ELK: Absolute coordinates for all elements
 * - ReactFlow: Relative coordinates for child nodes within parent containers
 */
/**
 * Coordinate system translator between ELK and ReactFlow
 */
export class CoordinateTranslator {
    /**
     * Convert ELK absolute coordinates to ReactFlow coordinates
     *
     * ELK uses absolute coordinates for all elements.
     * ReactFlow uses relative coordinates for child nodes within parent containers.
     *
     * @param elkCoords - Absolute coordinates from ELK
     * @param parentContainer - Parent container info (if node is inside a container)
     * @returns ReactFlow coordinates (relative to parent if applicable)
     */
    static elkToReactFlow(elkCoords: any, parentContainer: any): {
        x: any;
        y: any;
    };
    /**
     * Convert ReactFlow coordinates back to ELK absolute coordinates
     *
     * Used when ReactFlow reports position changes (e.g., user dragging nodes)
     * and we need to store them back in VisState using ELK coordinates.
     *
     * @param reactFlowCoords - ReactFlow coordinates (relative to parent if applicable)
     * @param parentContainer - Parent container info (if node is inside a container)
     * @returns Absolute coordinates in ELK format
     */
    static reactFlowToELK(reactFlowCoords: any, parentContainer: any): {
        x: any;
        y: any;
    };
    /**
     * Get container information for coordinate translation
     *
     * @param containerId - Container ID
     * @param visState - VisState instance to extract container info from
     * @returns Container info for coordinate translation
     */
    static getContainerInfo(containerId: any, visState: any): {
        id: any;
        x: any;
        y: any;
        width: any;
        height: any;
    };
    /**
     * Validate coordinate conversion
     *
     * Helper method to ensure coordinate translations are working correctly.
     * Useful for debugging coordinate system issues.
     */
    static validateConversion(originalELK: any, reactFlow: any, backToELK: any, parentContainer: any): boolean;
    /**
     * Debug coordinate system state
     *
     * Logs detailed information about coordinate systems for debugging
     */
    static debugCoordinates(elementId: any, elkCoords: any, reactFlowCoords: any, parentContainer: any): void;
}
//# sourceMappingURL=CoordinateTranslator.d.ts.map