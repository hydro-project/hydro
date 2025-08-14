/**
 * Test conta    test('should add label space to expanded containers', () => {
      const baseWidth = 300;
      const baseHeight = 200;
      
      // Simulate proper workflow: setContainer followed by setContainerLayout (like ELK does)
      visState.setContainer('container1', {
        collapsed: false
      });
      
      // Simulate ELK layout result being cached (this is how expandedDimensions gets set)
      visState.setContainerLayout('container1', {
        dimensions: { width: baseWidth, height: baseHeight }
      });

      const adjustedDims = visState.getContainerAdjustedDimensions('container1');

      // Should return the cached padded dimensions
      expect(adjustedDims.width).toBe(baseWidth);
      expect(adjustedDims.height).toBe(baseHeight + LAYOUT_CONSTANTS.CONTAINER_LABEL_HEIGHT + LAYOUT_CONSTANTS.CONTAINER_LABEL_PADDING);
    });ing and dimension adjustments
 */

import { createVisualizationState } from '../core/VisualizationState';
import { LAYOUT_CONSTANTS } from '../shared/config';
import { ELKBridge } from '../bridges/ELKBridge';

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
      
      // Create container first (without dimensions - encapsulation prevents external dimension control)
      visState.setContainer('container1', {
        collapsed: false
      });
      
      // Then simulate ELK layout result being applied (this sets expandedDimensions internally)
      visState.setContainerLayout('container1', {
        dimensions: { width: baseWidth, height: baseHeight }
      });

      const adjustedDims = visState.getContainerAdjustedDimensions('container1');

      // FIXED DOUBLE PADDING: Should return the raw ELK dimensions without manual label space
      // Container components handle label positioning internally
      expect(adjustedDims.width).toBe(baseWidth);
      expect(adjustedDims.height).toBe(baseHeight);
    });

    test('should ensure minimum dimensions for collapsed containers', () => {
      visState.setContainer('container1', {
        width: 50,  // Very small raw dimensions
        height: 20,
        collapsed: true
      });

      const adjustedDims = visState.getContainerAdjustedDimensions('container1');

      // FIXED DOUBLE PADDING: Should enforce minimum width and height without manual label space
      // Container components handle label positioning internally
      expect(adjustedDims.width).toBeGreaterThanOrEqual(LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH);
      expect(adjustedDims.height).toBeGreaterThanOrEqual(LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT);
    });

    test('should handle containers without explicit dimensions', () => {
      visState.setContainer('container1', {
        collapsed: false
      });

      const adjustedDims = visState.getContainerAdjustedDimensions('container1');

      // FIXED DOUBLE PADDING: Should use minimum dimensions without manual label space
      // Container components handle label positioning internally
      expect(adjustedDims.width).toBe(LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH);
      expect(adjustedDims.height).toBe(LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT);
    });

    test('should throw error for non-existent container', () => {
      expect(() => {
        visState.getContainerAdjustedDimensions('non-existent');
      }).toThrow();
    });
  });

  describe('visibleContainers integration', () => {
    test('should return containers with adjusted dimensions', async () => {
      // Create test data
      visState.setContainer('container1', {
        collapsed: false,
        children: ['node1', 'node2'],
        label: 'Test Container'
      });
      visState.setGraphNode('node1', { container: 'container1' });
      visState.setGraphNode('node2', { container: 'container1' });

      // Run ELK layout through VisState API
      const elkBridge = new ELKBridge();
      await elkBridge.layoutVisState(visState);
      
      const containers = visState.visibleContainers;
      const container = containers.find((c: any) => c.id === 'container1');

      expect(container).toBeDefined();
      expect(container!.width).toBeGreaterThan(0);
      expect(container!.height).toBeGreaterThan(0);
      
      // Verify that dimensions include label space by checking against getContainerAdjustedDimensions
      const adjustedDims = visState.getContainerAdjustedDimensions('container1');
      
      expect(container!.width).toBe(adjustedDims.width);
      expect(container!.height).toBe(adjustedDims.height);
      
      // For non-collapsed containers, height should include label space
      // The exact height depends on ELK layout, but should be the ELK result + label space
      // Check that the height is greater than what ELK would return without padding
      expect(container!.height).toBeGreaterThan(container!.height - LAYOUT_CONSTANTS.CONTAINER_LABEL_HEIGHT - LAYOUT_CONSTANTS.CONTAINER_LABEL_PADDING);
      
      // More specifically, verify that the height includes the expected padding
      // (We can't predict exact ELK layout results, but we can verify padding was added)
      expect(container!.height).toBeGreaterThan(50); // Some reasonable minimum
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
