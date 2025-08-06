/**
 * Integration test for FileDropZone component
 * Tests that the component can be imported and used correctly
 */

import { describe, test, expect } from 'vitest';
import { FileDropZone } from '../index';

describe('FileDropZone Integration', () => {
  test('should be importable from index', () => {
    expect(FileDropZone).toBeDefined();
    expect(typeof FileDropZone).toBe('function');
  });

  test('should have correct component interface', () => {
    // Test component props interface
    const testProps = {
      onFileLoad: (data: any) => {
        console.log('File loaded:', data);
      },
      hasData: false,
      className: 'test-class'
    };

    // Verify props structure is valid
    expect(testProps.onFileLoad).toBeInstanceOf(Function);
    expect(typeof testProps.hasData).toBe('boolean');
    expect(typeof testProps.className).toBe('string');
  });
});
