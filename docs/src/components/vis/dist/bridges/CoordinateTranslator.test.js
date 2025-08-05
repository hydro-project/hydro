/**
 * @fileoverview CoordinateTranslator Unit Tests
 *
 * Comprehensive tests for coordinate system translation between ELK and ReactFlow
 */
import assert from 'assert';
import { CoordinateTranslator } from './CoordinateTranslator';
console.log('Running CoordinateTranslator tests...');
// ============ Basic Coordinate Translation Tests ============
function testElkToReactFlowTopLevel() {
    console.log('  Testing elkToReactFlow for top-level elements...');
    const elkCoords = { x: 100, y: 200 };
    const result = CoordinateTranslator.elkToReactFlow(elkCoords);
    assert.strictEqual(result.x, 100, 'Top-level x coordinate should pass through unchanged');
    assert.strictEqual(result.y, 200, 'Top-level y coordinate should pass through unchanged');
    console.log('    ✅ Top-level coordinate pass-through works correctly');
}
function testElkToReactFlowChildElement() {
    console.log('  Testing elkToReactFlow for child elements...');
    const elkCoords = { x: 150, y: 225 };
    const container = {
        id: 'container1',
        x: 50,
        y: 75,
        width: 300,
        height: 400
    };
    const result = CoordinateTranslator.elkToReactFlow(elkCoords, container);
    assert.strictEqual(result.x, 100, 'Child x should be relative: 150-50=100');
    assert.strictEqual(result.y, 150, 'Child y should be relative: 225-75=150');
    console.log('    ✅ Child element coordinate conversion works correctly');
}
function testReactFlowToElkTopLevel() {
    console.log('  Testing reactFlowToELK for top-level elements...');
    const reactFlowCoords = { x: 300, y: 400 };
    const result = CoordinateTranslator.reactFlowToELK(reactFlowCoords);
    assert.strictEqual(result.x, 300, 'Top-level x coordinate should pass through unchanged');
    assert.strictEqual(result.y, 400, 'Top-level y coordinate should pass through unchanged');
    console.log('    ✅ Top-level reverse coordinate conversion works correctly');
}
function testReactFlowToElkChildElement() {
    console.log('  Testing reactFlowToELK for child elements...');
    const reactFlowCoords = { x: 100, y: 150 };
    const container = {
        id: 'container1',
        x: 50,
        y: 75,
        width: 300,
        height: 400
    };
    const result = CoordinateTranslator.reactFlowToELK(reactFlowCoords, container);
    assert.strictEqual(result.x, 150, 'Child x should be absolute: 100+50=150');
    assert.strictEqual(result.y, 225, 'Child y should be absolute: 150+75=225');
    console.log('    ✅ Child element reverse coordinate conversion works correctly');
}
function testRoundTripConversion() {
    console.log('  Testing round-trip coordinate conversion...');
    // Test top-level round trip
    const originalELK = { x: 123.45, y: 678.90 };
    const reactFlow = CoordinateTranslator.elkToReactFlow(originalELK);
    const backToELK = CoordinateTranslator.reactFlowToELK(reactFlow);
    assert.strictEqual(backToELK.x, originalELK.x, 'Top-level round-trip should preserve x coordinate');
    assert.strictEqual(backToELK.y, originalELK.y, 'Top-level round-trip should preserve y coordinate');
    // Test child element round trip
    const container = {
        id: 'container1',
        x: 62.5,
        y: 87.25,
        width: 400,
        height: 300
    };
    const originalChildELK = { x: 175.25, y: 287.75 };
    const childReactFlow = CoordinateTranslator.elkToReactFlow(originalChildELK, container);
    const backToChildELK = CoordinateTranslator.reactFlowToELK(childReactFlow, container);
    // Use tolerance for floating point comparison
    const tolerance = 0.0001;
    assert.ok(Math.abs(backToChildELK.x - originalChildELK.x) < tolerance, `Child round-trip x should preserve coordinate: ${backToChildELK.x} ≈ ${originalChildELK.x}`);
    assert.ok(Math.abs(backToChildELK.y - originalChildELK.y) < tolerance, `Child round-trip y should preserve coordinate: ${backToChildELK.y} ≈ ${originalChildELK.y}`);
    console.log('    ✅ Round-trip coordinate conversion preserves original values');
}
function testGetContainerInfo() {
    console.log('  Testing getContainerInfo...');
    const mockVisState = {
        getContainer: (id) => {
            if (id === 'container1') {
                return {
                    layout: {
                        position: { x: 100, y: 150 },
                        dimensions: { width: 300, height: 200 }
                    }
                };
            }
            return null;
        }
    };
    const result = CoordinateTranslator.getContainerInfo('container1', mockVisState);
    assert.ok(result, 'Should return container info for existing container');
    assert.strictEqual(result.id, 'container1', 'Should preserve container ID');
    assert.strictEqual(result.x, 100, 'Should extract x position');
    assert.strictEqual(result.y, 150, 'Should extract y position');
    assert.strictEqual(result.width, 300, 'Should extract width');
    assert.strictEqual(result.height, 200, 'Should extract height');
    const nonExistent = CoordinateTranslator.getContainerInfo('nonexistent', mockVisState);
    assert.strictEqual(nonExistent, undefined, 'Should return undefined for non-existent container');
    console.log('    ✅ Container info extraction works correctly');
}
function testValidateConversion() {
    console.log('  Testing validateConversion...');
    // Test valid conversion
    const originalELK = { x: 100, y: 200 };
    const reactFlow = { x: 50, y: 125 };
    const backToELK = { x: 100, y: 200 };
    const isValid = CoordinateTranslator.validateConversion(originalELK, reactFlow, backToELK);
    assert.strictEqual(isValid, true, 'Should return true for valid conversions');
    // Test invalid conversion
    const invalidBackToELK = { x: 101, y: 200 };
    const isInvalid = CoordinateTranslator.validateConversion(originalELK, reactFlow, invalidBackToELK);
    assert.strictEqual(isInvalid, false, 'Should return false for invalid conversions');
    console.log('    ✅ Conversion validation works correctly');
}
function testEdgeCases() {
    console.log('  Testing edge cases...');
    // Test zero coordinates
    const zeroELK = { x: 0, y: 0 };
    const zeroContainer = { id: 'zero', x: 0, y: 0, width: 100, height: 100 };
    const zeroReactFlow = CoordinateTranslator.elkToReactFlow(zeroELK, zeroContainer);
    const zeroBackToELK = CoordinateTranslator.reactFlowToELK(zeroReactFlow, zeroContainer);
    assert.strictEqual(zeroReactFlow.x, 0, 'Should handle zero coordinates correctly');
    assert.strictEqual(zeroReactFlow.y, 0, 'Should handle zero coordinates correctly');
    assert.strictEqual(zeroBackToELK.x, 0, 'Should preserve zero coordinates through round trip');
    assert.strictEqual(zeroBackToELK.y, 0, 'Should preserve zero coordinates through round trip');
    // Test negative coordinates
    const negativeELK = { x: 25, y: 50 };
    const negativeContainer = { id: 'negative', x: 100, y: 100, width: 200, height: 200 };
    const negativeReactFlow = CoordinateTranslator.elkToReactFlow(negativeELK, negativeContainer);
    assert.strictEqual(negativeReactFlow.x, -75, 'Should handle negative relative coordinates: 25-100=-75');
    assert.strictEqual(negativeReactFlow.y, -50, 'Should handle negative relative coordinates: 50-100=-50');
    console.log('    ✅ Edge cases handled correctly');
}
// ============ Run All Tests ============
export function runCoordinateTranslatorTests() {
    console.log('🧪 CoordinateTranslator Tests:');
    try {
        testElkToReactFlowTopLevel();
        testElkToReactFlowChildElement();
        testReactFlowToElkTopLevel();
        testReactFlowToElkChildElement();
        testRoundTripConversion();
        testGetContainerInfo();
        testValidateConversion();
        testEdgeCases();
        console.log('✅ All CoordinateTranslator tests passed!');
    }
    catch (error) {
        console.error('❌ CoordinateTranslator test failed:', error);
        throw error;
    }
}
// Run tests if this file is executed directly
if (require.main === module) {
    runCoordinateTranslatorTests();
}
//# sourceMappingURL=CoordinateTranslator.test.js.map