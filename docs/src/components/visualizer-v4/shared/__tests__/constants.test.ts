import { describe, test, expect } from 'vitest';
import { NODE_STYLES, EDGE_STYLES, CONTAINER_STYLES, LAYOUT_CONSTANTS } from '../config';

describe('Constants', () => {
  describe('NODE_STYLES', () => {
    it('should have all expected node styles', () => {
      expect(NODE_STYLES.DEFAULT).toBe('default');
      expect(NODE_STYLES.HIGHLIGHTED).toBe('highlighted');
      expect(NODE_STYLES.SELECTED).toBe('selected');
      expect(NODE_STYLES.WARNING).toBe('warning');
      expect(NODE_STYLES.ERROR).toBe('error');
    });
  });

  describe('EDGE_STYLES', () => {
    it('should have all expected edge styles', () => {
      expect(EDGE_STYLES.DEFAULT).toBe('default');
      expect(EDGE_STYLES.HIGHLIGHTED).toBe('highlighted');
      expect(EDGE_STYLES.DASHED).toBe('dashed');
      expect(EDGE_STYLES.THICK).toBe('thick');
      expect(EDGE_STYLES.WARNING).toBe('warning');
    });
  });

  describe('CONTAINER_STYLES', () => {
    it('should have all expected container styles', () => {
      expect(CONTAINER_STYLES.DEFAULT).toBe('default');
      expect(CONTAINER_STYLES.HIGHLIGHTED).toBe('highlighted');
      expect(CONTAINER_STYLES.SELECTED).toBe('selected');
      expect(CONTAINER_STYLES.MINIMIZED).toBe('minimized');
    });
  });

  describe('LAYOUT_CONSTANTS', () => {
    it('should have positive numeric values', () => {
      expect(LAYOUT_CONSTANTS.DEFAULT_NODE_WIDTH).toBeGreaterThan(0);
      expect(LAYOUT_CONSTANTS.DEFAULT_NODE_HEIGHT).toBeGreaterThan(0);
      expect(LAYOUT_CONSTANTS.DEFAULT_CONTAINER_PADDING).toBeGreaterThanOrEqual(0);
      expect(LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH).toBeGreaterThan(0);
      expect(LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT).toBeGreaterThan(0);
    });
  });
});
