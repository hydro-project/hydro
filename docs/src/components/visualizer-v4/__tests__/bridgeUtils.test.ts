import { describe, it, expect } from 'vitest';
import { hasMeaningfulELKPosition, getAdjustedContainerDimensionsSafe, safeNum } from '../bridges/bridgeUtils';

// Minimal test doubles for VisualizationState methods used by getAdjustedContainerDimensionsSafe
const makeVisState = (dims?: Record<string, { width?: number; height?: number }>) => ({
  getContainerAdjustedDimensions: (id: string) => dims?.[id] ?? undefined,
} as any);

describe('bridgeUtils', () => {
  describe('hasMeaningfulELKPosition', () => {
    it('returns false when undefined', () => {
      expect(hasMeaningfulELKPosition(undefined)).toBe(false);
    });
    it('returns false when position missing', () => {
      expect(hasMeaningfulELKPosition({})).toBe(false);
    });
    it('returns false for (0,0)', () => {
      expect(hasMeaningfulELKPosition({ position: { x: 0, y: 0 } })).toBe(false);
    });
    it('returns true when at least one coord is non-zero', () => {
      expect(hasMeaningfulELKPosition({ position: { x: 10, y: 0 } })).toBe(true);
      expect(hasMeaningfulELKPosition({ position: { x: 0, y: -5 } })).toBe(true);
    });
  });

  describe('getAdjustedContainerDimensionsSafe', () => {
    it('falls back to defaults when dims are missing or invalid', () => {
      const visState = makeVisState({});
      const result = getAdjustedContainerDimensionsSafe(visState as any, 'c1');
      expect(result.width).toBeGreaterThan(0);
      expect(result.height).toBeGreaterThan(0);
    });

    it('uses positive numbers when provided', () => {
      const visState = makeVisState({ c1: { width: 300, height: 200 } });
      const result = getAdjustedContainerDimensionsSafe(visState as any, 'c1');
      expect(result).toEqual({ width: 300, height: 200 });
    });
  });

  describe('safeNum', () => {
    it('returns 0 for invalid inputs', () => {
      expect(safeNum(undefined)).toBe(0);
      expect(safeNum(NaN)).toBe(0);
      expect(safeNum(Infinity)).toBe(0);
    });
    it('passes through finite numbers', () => {
      expect(safeNum(5)).toBe(5);
      expect(safeNum(-3.2)).toBe(-3.2);
    });
  });
});
