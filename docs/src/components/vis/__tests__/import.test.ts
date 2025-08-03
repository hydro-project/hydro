import { describe, it, expect } from 'vitest';

describe('Import test', () => {
  it('should import constants', async () => {
    const constants = await import('./shared/constants');
    expect(constants).toBeDefined();
    expect(constants.NODE_STYLES).toBeDefined();
  });
});
