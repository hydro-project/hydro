/**
 * Test script to verify node type color mapping
 * Run this to see how different node types map to colors
 */

import { generateNodeColors } from './shared/colorUtils';

// Test the node types from the chat.json data
const testNodeTypes = ['Transform', 'Tee', 'Sink', 'Network'];
const palette = 'Set2';

console.log('=== Node Type Color Mapping Test ===');
console.log(`Using palette: ${palette}`);
console.log('');

testNodeTypes.forEach(nodeType => {
  const colors = generateNodeColors(nodeType, palette);
  console.log(`${nodeType}:`);
  console.log(`  Primary: ${colors.primary}`);
  console.log(`  Border: ${colors.border}`);
  console.log(`  Gradient: ${colors.gradient}`);
  console.log('');
});

// Test what the legend will show
const legendItems = testNodeTypes.map(type => ({
  type: type,
  label: type.charAt(0).toUpperCase() + type.slice(1),
  description: {
    'Transform': 'Data transformation operations',
    'Tee': 'Data splitting operations', 
    'Sink': 'Data output destinations',
    'Network': 'Network communication nodes'
  }[type]
}));

console.log('=== Legend Items ===');
legendItems.forEach(item => {
  console.log(`${item.label}: ${item.description}`);
});
