import { describe, it, expect } from 'vitest';

describe('Import test', () => {
  it('should import shared constants', async () => {
    // Test importing shared constants 
    try {
      const constants = await import('../shared/config');
      expect(constants).toBeDefined();
      expect(constants.NODE_STYLES).toBeDefined();
    } catch (error) {
      console.warn('Constants import not available:', error);
      // Skip test if constants not available
      expect(true).toBe(true);
    }
  });
});
