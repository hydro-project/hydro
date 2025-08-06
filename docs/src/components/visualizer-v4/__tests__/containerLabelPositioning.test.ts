/**
 * Test container label positioning and dimension adjustments
 */

import { createVisualizationState } from '../core/VisState';
import { LAYOUT_CONSTANTS } from '../core/constants';

describe('Container Label Positioning & Dimensions', () => {
  let visState: any;

  beforeEach(() => {
    visState = createVisualizationState();
  });

  describe('getContainerAdjustedDimensions', () => {
    test('should add label space to expanded containers', () => {
      // Create a container with base dimensions
      const baseWidth = 300;
      const baseHeight = 200;
      
      visState.setContainer('container1', {
        expandedDimensions: { width: baseWidth, height: baseHeight },
        collapsed: false
      });

      const adjustedDims = visState.getContainerAdjustedDimensions('container1');

      // Should maintain width but add label space to height
      expect(adjustedDims.width).toBe(baseWidth);
      expect(adjustedDims.height).toBe(
        baseHeight + LAYOUT_CONSTANTS.CONTAINER_LABEL_HEIGHT + LAYOUT_CONSTANTS.CONTAINER_LABEL_PADDING
      );
    });

    test('should ensure minimum dimensions for collapsed containers', () => {
      visState.setContainer('container1', {
        expandedDimensions: { width: 50, height: 20 }, // Very small dimensions
        collapsed: true
      });

      const adjustedDims = visState.getContainerAdjustedDimensions('container1');

      // Should enforce minimum width and include label space in height
      expect(adjustedDims.width).toBeGreaterThanOrEqual(LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH);
      expect(adjustedDims.height).toBeGreaterThanOrEqual(
        LAYOUT_CONSTANTS.CONTAINER_LABEL_HEIGHT + LAYOUT_CONSTANTS.CONTAINER_LABEL_PADDING * 2
      );
    });

    test('should handle containers without explicit dimensions', () => {
      visState.setContainer('container1', {
        collapsed: false
      });

      const adjustedDims = visState.getContainerAdjustedDimensions('container1');

      // Should use minimum dimensions plus label space
      expect(adjustedDims.width).toBe(LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH);
      expect(adjustedDims.height).toBeGreaterThanOrEqual(
        LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT + LAYOUT_CONSTANTS.CONTAINER_LABEL_HEIGHT
      );
    });

    test('should throw error for non-existent container', () => {
      expect(() => {
        visState.getContainerAdjustedDimensions('non-existent');
      }).toThrow();
    });
  });

  describe('visibleContainers integration', () => {
    test('should return containers with adjusted dimensions', () => {
      visState.setContainer('container1', {
        expandedDimensions: { width: 300, height: 200 },
        collapsed: false,
        label: 'Test Container'
      });

      const containers = visState.visibleContainers;
      const container = containers.find((c: any) => c.id === 'container1');

      expect(container).toBeDefined();
      expect(container.width).toBe(300);
      expect(container.height).toBe(
        200 + LAYOUT_CONSTANTS.CONTAINER_LABEL_HEIGHT + LAYOUT_CONSTANTS.CONTAINER_LABEL_PADDING
      );
    });
  });

  describe('label positioning constants', () => {
    test('should have reasonable label constants', () => {
      expect(LAYOUT_CONSTANTS.CONTAINER_LABEL_HEIGHT).toBeGreaterThan(0);
      expect(LAYOUT_CONSTANTS.CONTAINER_LABEL_PADDING).toBeGreaterThan(0);
      expect(LAYOUT_CONSTANTS.CONTAINER_LABEL_FONT_SIZE).toBeGreaterThan(0);
      
      // Label height should be reasonable for 12px font
      expect(LAYOUT_CONSTANTS.CONTAINER_LABEL_HEIGHT).toBeGreaterThanOrEqual(16);
      expect(LAYOUT_CONSTANTS.CONTAINER_LABEL_HEIGHT).toBeLessThanOrEqual(32);
    });
  });
});

export {};
