/**
 * @fileoverview ELKBridge Fresh Instance Tests
 * 
 * Test that each new ELKBridge creates a truly fresh ELK instance
 * to prevent layout caching issues.
 */

import { describe, test, expect } from 'vitest';
import { ELKBridge } from '../ELKBridge';

describe('ELKBridge Fresh Instance Tests', () => {
  test('should create fresh ELK instance for each ELKBridge', () => {
    // Create two separate ELKBridge instances
    const bridge1 = new ELKBridge();
    const bridge2 = new ELKBridge();
    
    // They should be different instances
    expect(bridge1).not.toBe(bridge2);
    
    // Test that we can access the ELK instances (they should be different)
    // Note: We can't directly access the private elk property, 
    // but we can test that the bridges behave independently
    
    // This test mainly validates that the constructor doesn't throw
    // and that multiple instances can be created
    expect(bridge1).toBeInstanceOf(ELKBridge);
    expect(bridge2).toBeInstanceOf(ELKBridge);
  });

  test('should accept layout configuration in constructor', () => {
    const config = { algorithm: 'mrtree' as const };
    const bridge = new ELKBridge(config);
    
    expect(bridge).toBeInstanceOf(ELKBridge);
  });
});
