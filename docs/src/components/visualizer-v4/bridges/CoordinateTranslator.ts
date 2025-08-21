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

export interface ELKCoordinates {
  x: number;
  y: number;
}

export interface ReactFlowCoordinates {
  x: number;
  y: number;
}

export interface ContainerInfo {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

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
  static elkToReactFlow(
    elkCoords: ELKCoordinates, 
    parentContainer?: ContainerInfo
  ): ReactFlowCoordinates {
    if (!parentContainer) {
      // Top-level element: coordinates remain the same
      return {
        x: elkCoords.x,
        y: elkCoords.y
      };
    }
    
    // IMPORTANT: ELK already provides child coordinates relative to their container
    // So we don't need to subtract the parent container position - ELK has already done this!
    // The coordinates from ELK children are already relative to their parent.
    const relativeCoords = {
      x: elkCoords.x, // Already relative to parent container
      y: elkCoords.y  // Already relative to parent container
    };
    
    // // console.log(((`[CoordinateTranslator] ELK→ReactFlow: ELK child coordinates (${elkCoords.x}, ${elkCoords.y}) are already relative to container ${parentContainer.id} - using as-is`)));
    
    return relativeCoords;
  }
  
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
  static reactFlowToELK(
    reactFlowCoords: ReactFlowCoordinates,
    parentContainer?: ContainerInfo
  ): ELKCoordinates {
    if (!parentContainer) {
      // Top-level element: coordinates remain the same
      return {
        x: reactFlowCoords.x,
        y: reactFlowCoords.y
      };
    }
    
    // Since ELK expects child coordinates relative to their container,
    // and ReactFlow provides relative coordinates, we can use them directly.
    // We only add parent position if we need absolute coordinates in the VisState.
    const absoluteCoords = {
      x: reactFlowCoords.x + parentContainer.x,
      y: reactFlowCoords.y + parentContainer.y
    };
    
    // // console.log(((`[CoordinateTranslator] ReactFlow→ELK: relative(${reactFlowCoords.x}, ${reactFlowCoords.y}) → absolute(${absoluteCoords.x}, ${absoluteCoords.y}) within container ${parentContainer.id}`)));
    
    return absoluteCoords;
  }
  
  /**
   * Get container information for coordinate translation
   * 
   * @param containerId - Container ID
   * @param visState - VisState instance to extract container info from
   * @returns Container info for coordinate translation
   */
  static getContainerInfo(containerId: string, visState: any): ContainerInfo | undefined {
    const container = visState.getContainer(containerId);
    if (!container || !container.layout) {
      return undefined;
    }
    
    return {
      id: containerId,
      x: container.layout.position?.x || 0,
      y: container.layout.position?.y || 0,
      width: container.layout.dimensions?.width || 0,
      height: container.layout.dimensions?.height || 0
    };
  }
  
  /**
   * Validate coordinate conversion
   * 
   * Helper method to ensure coordinate translations are working correctly.
   * Useful for debugging coordinate system issues.
   */
  static validateConversion(
    originalELK: ELKCoordinates,
    reactFlow: ReactFlowCoordinates,
    backToELK: ELKCoordinates,
    parentContainer?: ContainerInfo
  ): boolean {
    const tolerance = 0.001; // Floating point tolerance
    
    const xMatch = Math.abs(originalELK.x - backToELK.x) < tolerance;
    const yMatch = Math.abs(originalELK.y - backToELK.y) < tolerance;
    
    if (!xMatch || !yMatch) {
      console.error('[CoordinateTranslator] ❌ Coordinate conversion validation failed:', {
        originalELK,
        reactFlow,
        backToELK,
        parentContainer: parentContainer?.id,
        xMatch,
        yMatch
      });
      return false;
    }
    
    // // console.log((('[CoordinateTranslator] ✅ Coordinate conversion validated successfully')));
    return true;
  }
  
  /**
   * Debug coordinate system state
   * 
   * Logs detailed information about coordinate systems for debugging
   */
  static debugCoordinates(
    elementId: string,
    elkCoords: ELKCoordinates,
    reactFlowCoords: ReactFlowCoordinates,
    parentContainer?: ContainerInfo
  ): void {
    // Debug logging removed for performance
  }
}
