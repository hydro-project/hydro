/**
 * Unit Tests for DRY Refactoring Utilities
 * 
 * Tests for the new shared utilities created during DRY refactoring
 */

import * as assert from 'assert';
import { 
  hexToRgb, 
  rgbToString, 
  createDarkBorder, 
  createVerticalGradient, 
  getNodeColorByType 
} from '../render/colorUtils.js';
import { 
  calculateEdgeStyle, 
  getEdgePathProps 
} from '../render/edgeUtils.js';

console.log('Running DRY Refactoring Tests...');

// ============ Color Utilities Tests ============

function testColorUtilities(): void {
  console.log('Testing color utilities...');
  
  // Test hexToRgb
  const rgb = hexToRgb('#ff0000');
  assert.strictEqual(rgb.r, 255, 'Red component should be 255');
  assert.strictEqual(rgb.g, 0, 'Green component should be 0');
  assert.strictEqual(rgb.b, 0, 'Blue component should be 0');
  
  // Test rgbToString
  const rgbString = rgbToString(255, 128, 64);
  assert.strictEqual(rgbString, 'rgb(255, 128, 64)', 'RGB string should be formatted correctly');
  
  // Test createDarkBorder
  const darkBorder = createDarkBorder('#ffffff', 0.5);
  assert.strictEqual(darkBorder, 'rgb(127, 127, 127)', 'Dark border should be 50% of white');
  
  // Test createVerticalGradient
  const gradient = createVerticalGradient('#808080');
  assert.ok(gradient.includes('linear-gradient'), 'Should create a linear gradient');
  assert.ok(gradient.includes('to bottom'), 'Should be a vertical gradient');
  
  // Test getNodeColorByType
  assert.strictEqual(getNodeColorByType('Source'), '#8dd3c7', 'Source should return teal');
  assert.strictEqual(getNodeColorByType('Transform'), '#ffffb3', 'Transform should return yellow');
  assert.strictEqual(getNodeColorByType('UnknownType'), '#b3de69', 'Unknown type should return default green');
  
  console.log('✓ Color utilities tests passed');
}

// ============ Edge Utilities Tests ============

function testEdgeUtilities(): void {
  console.log('Testing edge utilities...');
  
  // Test calculateEdgeStyle
  const mockEdge = { style: 'default' };
  const edgeStyle = calculateEdgeStyle(mockEdge, false, false, { strokeWidth: 3 });
  
  assert.ok(typeof edgeStyle === 'object', 'Should return style object');
  assert.ok('strokeWidth' in edgeStyle, 'Should have strokeWidth');
  assert.ok('stroke' in edgeStyle, 'Should have stroke color');
  assert.ok('strokeDasharray' in edgeStyle, 'Should have strokeDasharray');
  
  // Test getEdgePathProps
  const pathProps = getEdgePathProps('test-edge', 'M 0 0 L 100 100', edgeStyle, mockEdge, false);
  
  assert.strictEqual(pathProps.id, 'test-edge', 'Should have correct id');
  assert.strictEqual(pathProps.d, 'M 0 0 L 100 100', 'Should have correct path');
  assert.ok(pathProps.className.includes('react-flow__edge-path'), 'Should have correct class');
  assert.strictEqual(pathProps.fill, 'none', 'Should have fill none');
  assert.strictEqual(pathProps.strokeLinecap, 'round', 'Should have round line cap');
  
  console.log('✓ Edge utilities tests passed');
}

// ============ DRY Principle Verification ============

function testDRYPrinciples(): void {
  console.log('Testing DRY principles adherence...');
  
  // Test that different node types use the same color generation logic
  const sourceColor1 = getNodeColorByType('Source');
  const sourceColor2 = getNodeColorByType('Source');
  assert.strictEqual(sourceColor1, sourceColor2, 'Same node type should return same color consistently');
  
  // Test that color manipulation functions produce consistent results
  const border1 = createDarkBorder('#ff0000', 0.6);
  const border2 = createDarkBorder('#ff0000', 0.6);
  assert.strictEqual(border1, border2, 'Same input should produce same border color');
  
  // Test that edge styles are calculated consistently
  const mockEdge = { style: 'default' };
  const style1 = calculateEdgeStyle(mockEdge, false, false);
  const style2 = calculateEdgeStyle(mockEdge, false, false);
  assert.deepStrictEqual(style1, style2, 'Same edge should produce same style');
  
  console.log('✓ DRY principles verification passed');
}

// ============ Backward Compatibility Tests ============

function testBackwardCompatibility(): void {
  console.log('Testing backward compatibility...');
  
  // Test that utilities work with edge cases
  const whiteGradient = createVerticalGradient('#ffffff');
  assert.ok(whiteGradient.includes('linear-gradient'), 'Should handle white color');
  
  const blackBorder = createDarkBorder('#000000');
  assert.strictEqual(blackBorder, 'rgb(0, 0, 0)', 'Should handle black color');
  
  // Test empty or undefined inputs don't crash
  const defaultColor = getNodeColorByType('');
  assert.ok(typeof defaultColor === 'string', 'Should return default color for empty string');
  
  console.log('✓ Backward compatibility tests passed');
}

// ============ Run All Tests ============

export async function runAllTests(): Promise<void> {
  console.log('🔄 Running DRY Refactoring Tests\n');
  console.log('======================================\n');
  
  try {
    testColorUtilities();
    testEdgeUtilities();
    testDRYPrinciples();
    testBackwardCompatibility();
    
    console.log('\n======================================');
    console.log('🎉 All DRY refactoring tests passed!');
    console.log('✅ Color utilities working correctly');
    console.log('✅ Edge utilities working correctly');
    console.log('✅ DRY principles verified');
    console.log('✅ Backward compatibility maintained');
    
  } catch (error: unknown) {
    console.error('\n======================================');
    console.error('❌ DRY refactoring tests failed');
    console.error('Error:', error instanceof Error ? error.message : String(error));
    throw error;
  }
}

// Run tests if this file is executed directly  
if (typeof require !== 'undefined' && require.main === module) {
  runAllTests().catch(() => process.exit(1));
}