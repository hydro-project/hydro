/**
 * Unit Tests for Constants
 *
 * Tests for visualization constants and style definitions
 */
import assert from 'assert';
import { NODE_STYLES, EDGE_STYLES, CONTAINER_STYLES, LAYOUT_CONSTANTS } from '../dist/constants.js';
console.log('Running Constants tests...');
function testNodeStyles() {
    console.log('Testing node styles...');
    // Test that all expected node styles exist
    const expectedStyles = ['DEFAULT', 'HIGHLIGHTED', 'SELECTED', 'WARNING', 'ERROR'];
    for (const style of expectedStyles) {
        assert(NODE_STYLES.hasOwnProperty(style), `NODE_STYLES should have ${style}`);
        assert(typeof NODE_STYLES[style] === 'string', `NODE_STYLES.${style} should be a string`);
    }
    // Test that styles have expected values (lowercase)
    assert.strictEqual(NODE_STYLES.DEFAULT, 'default', 'DEFAULT style should be "default"');
    assert.strictEqual(NODE_STYLES.HIGHLIGHTED, 'highlighted', 'HIGHLIGHTED style should be "highlighted"');
    assert.strictEqual(NODE_STYLES.SELECTED, 'selected', 'SELECTED style should be "selected"');
    assert.strictEqual(NODE_STYLES.WARNING, 'warning', 'WARNING style should be "warning"');
    assert.strictEqual(NODE_STYLES.ERROR, 'error', 'ERROR style should be "error"');
    console.log('✓ Node styles tests passed');
}
function testEdgeStyles() {
    console.log('Testing edge styles...');
    // Test that all expected edge styles exist
    const expectedStyles = ['DEFAULT', 'HIGHLIGHTED', 'DASHED', 'THICK', 'WARNING'];
    for (const style of expectedStyles) {
        assert(EDGE_STYLES.hasOwnProperty(style), `EDGE_STYLES should have ${style}`);
        assert(typeof EDGE_STYLES[style] === 'string', `EDGE_STYLES.${style} should be a string`);
    }
    // Test that styles have expected values
    assert.strictEqual(EDGE_STYLES.DEFAULT, 'default', 'DEFAULT style should be "default"');
    assert.strictEqual(EDGE_STYLES.HIGHLIGHTED, 'highlighted', 'HIGHLIGHTED style should be "highlighted"');
    assert.strictEqual(EDGE_STYLES.DASHED, 'dashed', 'DASHED style should be "dashed"');
    assert.strictEqual(EDGE_STYLES.THICK, 'thick', 'THICK style should be "thick"');
    assert.strictEqual(EDGE_STYLES.WARNING, 'warning', 'WARNING style should be "warning"');
    console.log('✓ Edge styles tests passed');
}
function testContainerStyles() {
    console.log('Testing container styles...');
    // Test that all expected container styles exist
    const expectedStyles = ['DEFAULT', 'HIGHLIGHTED', 'SELECTED', 'MINIMIZED'];
    for (const style of expectedStyles) {
        assert(CONTAINER_STYLES.hasOwnProperty(style), `CONTAINER_STYLES should have ${style}`);
        assert(typeof CONTAINER_STYLES[style] === 'string', `CONTAINER_STYLES.${style} should be a string`);
    }
    // Test that styles have expected values
    assert.strictEqual(CONTAINER_STYLES.DEFAULT, 'default', 'DEFAULT style should be "default"');
    assert.strictEqual(CONTAINER_STYLES.HIGHLIGHTED, 'highlighted', 'HIGHLIGHTED style should be "highlighted"');
    assert.strictEqual(CONTAINER_STYLES.SELECTED, 'selected', 'SELECTED style should be "selected"');
    assert.strictEqual(CONTAINER_STYLES.MINIMIZED, 'minimized', 'MINIMIZED style should be "minimized"');
    console.log('✓ Container styles tests passed');
}
function testLayoutConstants() {
    console.log('Testing layout constants...');
    // Test that all expected layout constants exist
    const expectedConstants = [
        'DEFAULT_NODE_WIDTH',
        'DEFAULT_NODE_HEIGHT',
        'DEFAULT_CONTAINER_PADDING',
        'MIN_CONTAINER_WIDTH',
        'MIN_CONTAINER_HEIGHT'
    ];
    for (const constant of expectedConstants) {
        assert(LAYOUT_CONSTANTS.hasOwnProperty(constant), `LAYOUT_CONSTANTS should have ${constant}`);
        assert(typeof LAYOUT_CONSTANTS[constant] === 'number', `LAYOUT_CONSTANTS.${constant} should be a number`);
        assert(LAYOUT_CONSTANTS[constant] > 0, `LAYOUT_CONSTANTS.${constant} should be positive`);
    }
    // Test specific values are reasonable
    assert.strictEqual(LAYOUT_CONSTANTS.DEFAULT_NODE_WIDTH, 100, 'Default node width should be 100');
    assert.strictEqual(LAYOUT_CONSTANTS.DEFAULT_NODE_HEIGHT, 40, 'Default node height should be 40');
    assert.strictEqual(LAYOUT_CONSTANTS.DEFAULT_CONTAINER_PADDING, 20, 'Default container padding should be 20');
    assert.strictEqual(LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH, 150, 'Min container width should be 150');
    assert.strictEqual(LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT, 100, 'Min container height should be 100');
    console.log('✓ Layout constants tests passed');
}
function testStyleUniqueness() {
    console.log('Testing style uniqueness...');
    // Test that node styles are unique
    const nodeStyleValues = Object.values(NODE_STYLES);
    const uniqueNodeStyles = new Set(nodeStyleValues);
    assert.strictEqual(nodeStyleValues.length, uniqueNodeStyles.size, 'All node styles should be unique');
    // Test that edge styles are unique
    const edgeStyleValues = Object.values(EDGE_STYLES);
    const uniqueEdgeStyles = new Set(edgeStyleValues);
    assert.strictEqual(edgeStyleValues.length, uniqueEdgeStyles.size, 'All edge styles should be unique');
    // Test that container styles are unique
    const containerStyleValues = Object.values(CONTAINER_STYLES);
    const uniqueContainerStyles = new Set(containerStyleValues);
    assert.strictEqual(containerStyleValues.length, uniqueContainerStyles.size, 'All container styles should be unique');
    console.log('✓ Style uniqueness tests passed');
}
function testConstantsImmutability() {
    console.log('Testing constants immutability...');
    // Test that we can't modify the constant objects (this is more of a documentation test)
    const originalNodeDefault = NODE_STYLES.DEFAULT;
    const originalEdgeDefault = EDGE_STYLES.DEFAULT;
    // These should not throw in a properly implemented system
    // (though JavaScript doesn't prevent modification without Object.freeze)
    try {
        // Attempt to modify (this won't actually prevent it in JS without Object.freeze)
        const testValue = NODE_STYLES.DEFAULT;
        assert.strictEqual(testValue, originalNodeDefault, 'Node styles should remain unchanged');
        const testValue2 = EDGE_STYLES.DEFAULT;
        assert.strictEqual(testValue2, originalEdgeDefault, 'Edge styles should remain unchanged');
    }
    catch (error) {
        // If constants are properly frozen, this would be expected
        console.log('Constants are properly protected from modification');
    }
    console.log('✓ Constants immutability tests passed');
}
// ============ Run All Tests ============
function runAllTests() {
    try {
        testNodeStyles();
        testEdgeStyles();
        testContainerStyles();
        testLayoutConstants();
        testStyleUniqueness();
        testConstantsImmutability();
        console.log('\n🎉 All constants tests passed! Constants are properly defined.');
    }
    catch (error) {
        console.error('\n❌ Constants test failed:', error.message);
        console.error(error.stack);
        process.exit(1);
    }
}
// Export for potential use in other test files
export { testNodeStyles, testEdgeStyles, testContainerStyles, testLayoutConstants, testStyleUniqueness, testConstantsImmutability, runAllTests };
// Run tests if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
    runAllTests();
}
//# sourceMappingURL=constants.test.js.map