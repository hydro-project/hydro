/**
 * Integration test for FileDropZone component
 * Tests that the component can be imported and used correctly
 */

import { FileDropZone } from '../index';

// Test that FileDropZone is properly exported
console.log('FileDropZone imported:', typeof FileDropZone);

// Test component props interface
const testProps = {
  onFileLoad: (data: any) => {
    console.log('File loaded:', data);
  },
  hasData: false,
  className: 'test-class'
};

// This would be used in a React component:
// <FileDropZone {...testProps} />

export { FileDropZone };
