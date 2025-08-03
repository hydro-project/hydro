/**
 * Layout Configuration Tests (TypeScript)
 * 
 * Tests for layout configuration system including presets,
 * custom configuration creation, and configuration validation.
 */

import assert from 'assert';
import {
  DEFAULT_LAYOUT_CONFIG,
  LAYOUT_CONFIGS,
  getLayoutConfig,
  createLayoutConfig
} from '../config.js';
import { LayoutConfig } from '../types.js';

/**
 * Test default layout configuration structure
 */
function testDefaultLayoutConfig(): void {
  console.log('Testing default layout configuration...');
  
  // Verify structure
  assert(typeof DEFAULT_LAYOUT_CONFIG === 'object', 'DEFAULT_LAYOUT_CONFIG should be an object');
  assert('algorithm' in DEFAULT_LAYOUT_CONFIG, 'Should have algorithm property');
  assert('direction' in DEFAULT_LAYOUT_CONFIG, 'Should have direction property');
  assert('spacing' in DEFAULT_LAYOUT_CONFIG, 'Should have spacing property');
  assert('nodeSize' in DEFAULT_LAYOUT_CONFIG, 'Should have nodeSize property');
  
  // Verify types
  assert(typeof DEFAULT_LAYOUT_CONFIG.algorithm === 'string', 'Algorithm should be string');
  assert(typeof DEFAULT_LAYOUT_CONFIG.direction === 'string', 'Direction should be string');
  assert(typeof DEFAULT_LAYOUT_CONFIG.spacing === 'number', 'Spacing should be number');
  assert(typeof DEFAULT_LAYOUT_CONFIG.nodeSize === 'object', 'NodeSize should be object');
  
  // Verify nodeSize structure
  assert('width' in DEFAULT_LAYOUT_CONFIG.nodeSize, 'NodeSize should have width');
  assert('height' in DEFAULT_LAYOUT_CONFIG.nodeSize, 'NodeSize should have height');
  assert(typeof DEFAULT_LAYOUT_CONFIG.nodeSize.width === 'number', 'NodeSize width should be number');
  assert(typeof DEFAULT_LAYOUT_CONFIG.nodeSize.height === 'number', 'NodeSize height should be number');
  
  // Verify reasonable values
  assert(DEFAULT_LAYOUT_CONFIG.spacing > 0, 'Spacing should be positive');
  assert(DEFAULT_LAYOUT_CONFIG.nodeSize.width > 0, 'Node width should be positive');
  assert(DEFAULT_LAYOUT_CONFIG.nodeSize.height > 0, 'Node height should be positive');
  
  console.log('✅ Default layout configuration test passed');
}

/**
 * Test layout configuration presets
 */
function testLayoutConfigPresets(): void {
  console.log('Testing layout configuration presets...');
  
  // Verify all expected presets exist
  const expectedPresets = ['DEFAULT', 'COMPACT', 'LOOSE', 'HORIZONTAL'];
  expectedPresets.forEach(preset => {
    assert(preset in LAYOUT_CONFIGS, `Should have ${preset} preset`);
  });
  
  // Test each preset has required properties
  Object.entries(LAYOUT_CONFIGS).forEach(([name, config]) => {
    assert(typeof config === 'object', `${name} config should be object`);
    assert('algorithm' in config, `${name} should have algorithm`);
    assert('direction' in config, `${name} should have direction`);
    assert('spacing' in config, `${name} should have spacing`);
    assert('nodeSize' in config, `${name} should have nodeSize`);
    
    // Verify values are reasonable
    assert(config.spacing > 0, `${name} spacing should be positive`);
    assert(config.nodeSize.width > 0, `${name} node width should be positive`);
    assert(config.nodeSize.height > 0, `${name} node height should be positive`);
  });
  
  // Test specific preset characteristics
  assert(LAYOUT_CONFIGS.COMPACT.spacing < LAYOUT_CONFIGS.DEFAULT.spacing, 'COMPACT should have smaller spacing than DEFAULT');
  assert(LAYOUT_CONFIGS.LOOSE.spacing > LAYOUT_CONFIGS.DEFAULT.spacing, 'LOOSE should have larger spacing than DEFAULT');
  assert(LAYOUT_CONFIGS.HORIZONTAL.direction !== LAYOUT_CONFIGS.DEFAULT.direction, 'HORIZONTAL should have different direction');
  
  console.log('✅ Layout configuration presets test passed');
}

/**
 * Test getLayoutConfig function
 */
function testGetLayoutConfig(): void {
  console.log('Testing getLayoutConfig function...');
  
  // Test valid preset names
  const defaultConfig = getLayoutConfig('DEFAULT');
  assert.deepStrictEqual(defaultConfig, LAYOUT_CONFIGS.DEFAULT, 'Should return DEFAULT preset');
  
  const compactConfig = getLayoutConfig('COMPACT');
  assert.deepStrictEqual(compactConfig, LAYOUT_CONFIGS.COMPACT, 'Should return COMPACT preset');
  
  const looseConfig = getLayoutConfig('LOOSE');
  assert.deepStrictEqual(looseConfig, LAYOUT_CONFIGS.LOOSE, 'Should return LOOSE preset');
  
  const horizontalConfig = getLayoutConfig('HORIZONTAL');
  assert.deepStrictEqual(horizontalConfig, LAYOUT_CONFIGS.HORIZONTAL, 'Should return HORIZONTAL preset');
  
  // Verify returned configs are complete
  [defaultConfig, compactConfig, looseConfig, horizontalConfig].forEach((config, index) => {
    const presetName = ['DEFAULT', 'COMPACT', 'LOOSE', 'HORIZONTAL'][index];
    assert('algorithm' in config, `${presetName} should have all required properties`);
    assert('direction' in config, `${presetName} should have all required properties`);
    assert('spacing' in config, `${presetName} should have all required properties`);
    assert('nodeSize' in config, `${presetName} should have all required properties`);
  });
  
  console.log('✅ getLayoutConfig function test passed');
}

/**
 * Test createLayoutConfig function
 */
function testCreateLayoutConfig(): void {
  console.log('Testing createLayoutConfig function...');
  
  // Test with no overrides (should return default)
  const defaultCreated = createLayoutConfig();
  assert.deepStrictEqual(defaultCreated, DEFAULT_LAYOUT_CONFIG, 'Should return default when no overrides');
  
  // Test with partial overrides
  const customSpacing = createLayoutConfig({ spacing: 50 });
  assert.strictEqual(customSpacing.spacing, 50, 'Should override spacing');
  assert.strictEqual(customSpacing.algorithm, DEFAULT_LAYOUT_CONFIG.algorithm, 'Should keep default algorithm');
  assert.strictEqual(customSpacing.direction, DEFAULT_LAYOUT_CONFIG.direction, 'Should keep default direction');
  assert.deepStrictEqual(customSpacing.nodeSize, DEFAULT_LAYOUT_CONFIG.nodeSize, 'Should keep default nodeSize');
  
  // Test with multiple overrides
  const customMultiple = createLayoutConfig({
    algorithm: 'force' as any,
    spacing: 75,
    nodeSize: { width: 150, height: 50 }
  });
  assert.strictEqual(customMultiple.algorithm, 'force', 'Should override algorithm');
  assert.strictEqual(customMultiple.spacing, 75, 'Should override spacing');
  assert.strictEqual(customMultiple.nodeSize.width, 150, 'Should override node width');
  assert.strictEqual(customMultiple.nodeSize.height, 50, 'Should override node height');
  assert.strictEqual(customMultiple.direction, DEFAULT_LAYOUT_CONFIG.direction, 'Should keep default direction');
  
  // Test with empty object override
  const emptyOverride = createLayoutConfig({});
  assert.deepStrictEqual(emptyOverride, DEFAULT_LAYOUT_CONFIG, 'Should return default with empty override');
  
  // Test partial nodeSize override
  const partialNodeSize = createLayoutConfig({
    nodeSize: { width: 200 } as any // TypeScript would normally catch this, but testing runtime behavior
  });
  assert.strictEqual(partialNodeSize.nodeSize.width, 200, 'Should override node width');
  // Note: This would be undefined in JavaScript, but TypeScript makes this a compile error
  
  console.log('✅ createLayoutConfig function test passed');
}

/**
 * Test configuration validation and edge cases
 */
function testConfigurationValidation(): void {
  console.log('Testing configuration validation...');
  
  // Test all presets have valid algorithms
  const validAlgorithms = ['layered', 'stress', 'force', 'mrtree', 'radial', 'disco'];
  Object.entries(LAYOUT_CONFIGS).forEach(([name, config]) => {
    assert(validAlgorithms.includes(config.algorithm), `${name} should have valid algorithm: ${config.algorithm}`);
  });
  
  // Test all presets have valid directions
  const validDirections = ['RIGHT', 'LEFT', 'UP', 'DOWN'];
  Object.entries(LAYOUT_CONFIGS).forEach(([name, config]) => {
    assert(validDirections.includes(config.direction), `${name} should have valid direction: ${config.direction}`);
  });
  
  // Test spacing ranges are reasonable
  Object.entries(LAYOUT_CONFIGS).forEach(([name, config]) => {
    assert(config.spacing >= 10 && config.spacing <= 200, `${name} spacing should be in reasonable range: ${config.spacing}`);
  });
  
  // Test node sizes are reasonable
  Object.entries(LAYOUT_CONFIGS).forEach(([name, config]) => {
    assert(config.nodeSize.width >= 50 && config.nodeSize.width <= 500, `${name} node width should be reasonable: ${config.nodeSize.width}`);
    assert(config.nodeSize.height >= 20 && config.nodeSize.height <= 200, `${name} node height should be reasonable: ${config.nodeSize.height}`);
  });
  
  console.log('✅ Configuration validation test passed');
}

/**
 * Test configuration immutability
 */
function testConfigurationImmutability(): void {
  console.log('Testing configuration immutability...');
  
  // Test that returned configs can't modify the originals
  const config1 = getLayoutConfig('DEFAULT');
  const config2 = getLayoutConfig('DEFAULT');
  
  // Modify one config
  config1.spacing = 999;
  
  // Verify the other is unchanged
  assert.notStrictEqual(config2.spacing, 999, 'Modifying one config should not affect another');
  assert.strictEqual(config2.spacing, DEFAULT_LAYOUT_CONFIG.spacing, 'Original config should remain unchanged');
  
  // Test createLayoutConfig immutability
  const custom1 = createLayoutConfig({ spacing: 100 });
  const custom2 = createLayoutConfig({ spacing: 100 });
  
  custom1.algorithm = 'force' as any;
  assert.notStrictEqual(custom2.algorithm, 'force', 'Created configs should be independent');
  
  console.log('✅ Configuration immutability test passed');
}

/**
 * Test TypeScript type safety (compile-time checks)
 */
function testTypeScriptTypes(): void {
  console.log('Testing TypeScript type safety...');
  
  // Test that LayoutConfig interface works properly
  const validConfig: LayoutConfig = {
    algorithm: 'layered',
    direction: 'RIGHT',
    spacing: 50,
    nodeSize: { width: 180, height: 60 }
  };
  
  // This should compile without errors
  const result = createLayoutConfig(validConfig);
  assert(result, 'Valid config should work');
  
  // Test partial configs work
  const partialConfig: Partial<LayoutConfig> = {
    spacing: 75
  };
  
  const resultPartial = createLayoutConfig(partialConfig);
  assert.strictEqual(resultPartial.spacing, 75, 'Partial config should work');
  
  console.log('✅ TypeScript type safety test passed');
}

/**
 * Run all layout configuration tests
 */
export function runAllTests(): Promise<void> {
  console.log('🧪 Running Layout Configuration Tests');
  console.log('====================================\n');
  
  return new Promise((resolve, reject) => {
    try {
      testDefaultLayoutConfig();
      testLayoutConfigPresets();
      testGetLayoutConfig();
      testCreateLayoutConfig();
      testConfigurationValidation();
      testConfigurationImmutability();
      testTypeScriptTypes();
      
      console.log('\n🎉 All layout configuration tests passed!');
      console.log('✅ Configuration system is working correctly!');
      resolve();
    } catch (error: unknown) {
      console.error('\n❌ Layout configuration test failed:', error instanceof Error ? error.message : String(error));
      if (error instanceof Error) {
        console.error(error.stack);
      }
      reject(error);
    }
  });
}

// Export individual test functions for selective testing
export {
  testDefaultLayoutConfig,
  testLayoutConfigPresets,
  testGetLayoutConfig,
  testCreateLayoutConfig,
  testConfigurationValidation,
  testConfigurationImmutability,
  testTypeScriptTypes
};

// Run tests if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runAllTests().catch(() => process.exit(1));
}
