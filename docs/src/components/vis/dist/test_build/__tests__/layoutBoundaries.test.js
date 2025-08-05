/**
 * @fileoverview Layout Boundary Tests
 *
 * Tests to ensure that after ELK layout, all child nodes fall within their parent container boundaries.
 * These tests validate the coordinate translation between ELK and ReactFlow.
 */
import assert from 'assert';
import { parseGraphJSON } from '../core/JSONParser.js';
import { createVisualizationEngine } from '../core/VisualizationEngine.js';
import { ReactFlowBridge } from '../bridges/ReactFlowBridge.js';
import { CoordinateTranslator } from '../bridges/CoordinateTranslator.js';
console.log('Running Layout Boundary Validation tests...');
function validateNodeWithinContainer(childNode, containerNode) {
    const violations = [];
    // Child node bounds
    const childLeft = childNode.position.x;
    const childTop = childNode.position.y;
    const childRight = childLeft + (childNode.data.width || 120); // Default node width
    const childBottom = childTop + (childNode.data.height || 40); // Default node height
    // Container bounds (should start from 0,0 since child coordinates are relative)
    const containerWidth = containerNode.data.width || containerNode.style?.width || 200;
    const containerHeight = containerNode.data.height || containerNode.style?.height || 300;
    // Check boundaries
    if (childLeft < 0) {
        violations.push(`Child ${childNode.id} left boundary ${childLeft} < 0`);
    }
    if (childTop < 0) {
        violations.push(`Child ${childNode.id} top boundary ${childTop} < 0`);
    }
    if (childRight > containerWidth) {
        violations.push(`Child ${childNode.id} right boundary ${childRight} > container width ${containerWidth}`);
    }
    if (childBottom > containerHeight) {
        violations.push(`Child ${childNode.id} bottom boundary ${childBottom} > container height ${containerHeight}`);
    }
    return {
        isWithin: violations.length === 0,
        violations
    };
}
function validateAllBoundaries(reactFlowData) {
    const violations = [];
    // Create maps for quick lookup
    const nodeMap = new Map(reactFlowData.nodes.map(n => [n.id, n]));
    // Check each child node against its parent container
    reactFlowData.nodes.forEach(node => {
        if (node.parentId && node.extent === 'parent') {
            const parentContainer = nodeMap.get(node.parentId);
            if (parentContainer && parentContainer.type === 'container') {
                const validation = validateNodeWithinContainer(node, parentContainer);
                if (!validation.isWithin) {
                    violations.push({
                        childId: node.id,
                        containerId: node.parentId,
                        issues: validation.violations
                    });
                }
            }
        }
    });
    return {
        allValid: violations.length === 0,
        violations
    };
}
// ============ Boundary Validation Tests ============
function testBoundaryCheckingFunction() {
    console.log('Testing boundary checking function...');
    const containerNode = {
        id: 'container1',
        type: 'container',
        position: { x: 0, y: 0 },
        data: { label: 'Container', style: 'default', width: 200, height: 300 },
        style: { width: 200, height: 300 }
    };
    // Valid child node
    const validChild = {
        id: 'child1',
        type: 'standard',
        position: { x: 10, y: 10 },
        data: { label: 'Valid Child', style: 'default' },
        parentId: 'container1',
        extent: 'parent'
    };
    // Invalid child node (outside bounds)
    const invalidChild = {
        id: 'child2',
        type: 'standard',
        position: { x: -50, y: 350 }, // Negative X, Y beyond container height
        data: { label: 'Invalid Child', style: 'default' },
        parentId: 'container1',
        extent: 'parent'
    };
    const validResult = validateNodeWithinContainer(validChild, containerNode);
    assert(validResult.isWithin, 'Valid child should be within bounds');
    assert.strictEqual(validResult.violations.length, 0, 'Valid child should have no violations');
    const invalidResult = validateNodeWithinContainer(invalidChild, containerNode);
    assert(!invalidResult.isWithin, 'Invalid child should not be within bounds');
    assert(invalidResult.violations.some(v => v.includes('left boundary -50 < 0')), 'Should detect negative X');
    assert(invalidResult.violations.some(v => v.includes('bottom boundary 390 > container height 300')), 'Should detect Y overflow');
    console.log('✅ Boundary checking function tests passed');
}
async function testSimpleLayoutBoundaries() {
    console.log('Testing simple layout boundaries...');
    // Create simple test data with one container and one child
    const testData = {
        nodes: [
            { id: '0', label: 'Child Node', x: 0, y: 0 }
        ],
        edges: [],
        containers: [
            { id: 'container1', children: ['0'], collapsed: false }
        ]
    };
    // Parse and create visualization state
    const { state } = parseGraphJSON(testData);
    // Run layout
    const engine = createVisualizationEngine(state);
    await engine.runLayout();
    // Convert to ReactFlow
    const bridge = new ReactFlowBridge();
    const reactFlowData = bridge.visStateToReactFlow(state);
    // Validate boundaries
    const validation = validateAllBoundaries(reactFlowData);
    if (!validation.allValid) {
        console.error('❌ Boundary violations found:');
        validation.violations.forEach(violation => {
            console.error(`  Child ${violation.childId} in container ${violation.containerId}:`);
            violation.issues.forEach(issue => console.error(`    - ${issue}`));
        });
        // Print ReactFlow data for debugging
        console.error('ReactFlow data:', JSON.stringify(reactFlowData, null, 2));
    }
    assert(validation.allValid, 'All child nodes should be within their container boundaries');
    console.log('✅ Simple layout boundary tests passed');
}
async function testMultiContainerBoundaries() {
    console.log('Testing multi-container boundaries...');
    const testData = {
        nodes: [
            { id: '0', label: 'Node 0', x: 0, y: 0 },
            { id: '1', label: 'Node 1', x: 0, y: 0 },
            { id: '2', label: 'Node 2', x: 0, y: 0 },
            { id: '3', label: 'Node 3', x: 0, y: 0 }
        ],
        edges: [
            { id: 'e0', source: '0', target: '1' },
            { id: 'e1', source: '2', target: '3' }
        ],
        containers: [
            { id: 'container1', children: ['0', '1'], collapsed: false },
            { id: 'container2', children: ['2', '3'], collapsed: false }
        ]
    };
    const { state } = parseGraphJSON(testData);
    const engine = createVisualizationEngine(state);
    await engine.runLayout();
    const bridge = new ReactFlowBridge();
    const reactFlowData = bridge.visStateToReactFlow(state);
    const validation = validateAllBoundaries(reactFlowData);
    if (!validation.allValid) {
        console.error('❌ Multi-container boundary violations:');
        validation.violations.forEach(violation => {
            console.error(`  Child ${violation.childId} in container ${violation.containerId}:`);
            violation.issues.forEach(issue => console.error(`    - ${issue}`));
        });
    }
    assert(validation.allValid, 'All child nodes should be within their container boundaries in multi-container layout');
    console.log('✅ Multi-container boundary tests passed');
}
// ============ Coordinate Translation Tests ============
function testCoordinateTranslation() {
    console.log('Testing coordinate translation...');
    const elkCoords = { x: 100, y: 200 };
    const containerInfo = { id: 'container1', x: 50, y: 100, width: 300, height: 400 };
    const reactFlowCoords = CoordinateTranslator.elkToReactFlow(elkCoords, containerInfo);
    // Should be relative to container
    assert.strictEqual(reactFlowCoords.x, 50, 'X coordinate should be relative to container: 100 - 50 = 50');
    assert.strictEqual(reactFlowCoords.y, 100, 'Y coordinate should be relative to container: 200 - 100 = 100');
    // Convert back to ELK
    const backToELK = CoordinateTranslator.reactFlowToELK(reactFlowCoords, containerInfo);
    assert.strictEqual(backToELK.x, elkCoords.x, 'Round-trip X coordinate should match original');
    assert.strictEqual(backToELK.y, elkCoords.y, 'Round-trip Y coordinate should match original');
    console.log('✅ Coordinate translation tests passed');
}
function testTopLevelNodeTranslation() {
    console.log('Testing top-level node translation...');
    const elkCoords = { x: 100, y: 200 };
    const reactFlowCoords = CoordinateTranslator.elkToReactFlow(elkCoords);
    // Should remain the same for top-level nodes
    assert.strictEqual(reactFlowCoords.x, 100, 'Top-level X coordinate should remain unchanged');
    assert.strictEqual(reactFlowCoords.y, 200, 'Top-level Y coordinate should remain unchanged');
    console.log('✅ Top-level node translation tests passed');
}
// ============ Run All Tests ============
export async function runLayoutBoundaryTests() {
    console.log('\n🧪 Starting Layout Boundary Tests...\n');
    try {
        // Synchronous tests
        testBoundaryCheckingFunction();
        testCoordinateTranslation();
        testTopLevelNodeTranslation();
        // Asynchronous tests
        await testSimpleLayoutBoundaries();
        await testMultiContainerBoundaries();
        console.log('\n✅ All Layout Boundary Tests Passed!\n');
    }
    catch (error) {
        console.error('\n❌ Layout Boundary Tests Failed:', error);
        throw error;
    }
}
// Run tests if this file is executed directly
if (typeof process !== 'undefined' && process.argv && process.argv[1]?.endsWith('layoutBoundaries.test.ts')) {
    runLayoutBoundaryTests().catch(console.error);
}
//# sourceMappingURL=layoutBoundaries.test.js.map