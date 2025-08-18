/**
 * Fuzz Testing for VisualizationState
 *
 * Performs randomized collapse/expand operations on parsed JSON data
 * and validates all system invariants throughout the process.
 */
import assert from 'assert';
import { readFile } from 'fs/promises';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { parseGraphJSON, validateGraphJSON } from '../dist/core/JSONParser.js';
const __dirname = dirname(fileURLToPath(import.meta.url));
// Fuzz test configuration
const FUZZ_ITERATIONS = 100; // Number of random operations per test
const MAX_OPERATIONS_PER_ITERATION = 50; // Max operations in a single iteration
const OPERATION_SEED = 42; // For reproducible randomness
// Simple PRNG for reproducible tests
class SimpleRandom {
    constructor(seed) {
        this.seed = seed;
    }
    next() {
        this.seed = (this.seed * 9301 + 49297) % 233280;
        return this.seed / 233280;
    }
    choice(array) {
        return array[Math.floor(this.next() * array.length)];
    }
    boolean(probability = 0.5) {
        return this.next() < probability;
    }
}
/**
 * System invariants that must always hold
 */
class InvariantChecker {
    constructor(state) {
        this.state = state;
    }
    /**
     * Check all invariants and throw if any are violated
     */
    checkAll(context = '') {
        this.checkNodeVisibilityInvariant(context);
        this.checkEdgeVisibilityInvariant(context);
        this.checkContainerHierarchyInvariant(context);
        this.checkHyperEdgeConsistency(context);
        this.checkCollectionConsistency(context);
        this.checkNodeToEdgeMappingInvariant(context);
    }
    /**
     * Invariant: A node is visible iff it's not hidden and no parent container is collapsed
     */
    checkNodeVisibilityInvariant(context) {
        const allNodes = Array.from(this.state.graphNodes.values());
        const visibleNodes = this.state.visibleNodes;
        const visibleNodeIds = new Set(visibleNodes.map(n => n.id));
        for (const node of allNodes) {
            const shouldBeVisible = !node.hidden && !this.isNodeInCollapsedContainer(node.id);
            const isVisible = visibleNodeIds.has(node.id);
            assert.strictEqual(isVisible, shouldBeVisible, `${context}: Node ${node.id} visibility invariant violated. Hidden: ${node.hidden}, InCollapsed: ${this.isNodeInCollapsedContainer(node.id)}, Visible: ${isVisible}`);
        }
    }
    /**
     * Invariant: An edge is visible iff both its endpoints are visible
     */
    checkEdgeVisibilityInvariant(context) {
        const allEdges = Array.from(this.state.graphEdges.values());
        const visibleEdges = this.state.visibleEdges;
        const visibleEdgeIds = new Set(visibleEdges.map(e => e.id));
        for (const edge of allEdges) {
            const sourceNode = this.state.getGraphNode(edge.source);
            const targetNode = this.state.getGraphNode(edge.target);
            const sourceVisible = !sourceNode.hidden && !this.isNodeInCollapsedContainer(edge.source);
            const targetVisible = !targetNode.hidden && !this.isNodeInCollapsedContainer(edge.target);
            const shouldBeVisible = sourceVisible && targetVisible;
            const isVisible = visibleEdgeIds.has(edge.id);
            assert.strictEqual(isVisible, shouldBeVisible, `${context}: Edge ${edge.id} (${edge.source} -> ${edge.target}) visibility invariant violated. Source visible: ${sourceVisible}, Target visible: ${targetVisible}, Edge visible: ${isVisible}`);
        }
    }
    /**
     * Invariant: Container hierarchy relationships are consistent
     */
    checkContainerHierarchyInvariant(context) {
        const allContainers = Array.from(this.state.containers.values());
        for (const container of allContainers) {
            const children = this.state.getContainerChildren(container.id);
            // Check that all children reference this container as parent
            for (const childId of children) {
                if (this.state.graphNodes.has(childId)) {
                    // It's a node
                    assert.strictEqual(this.state.getNodeContainer(childId), container.id, `${context}: Node ${childId} should have container ${container.id} as parent`);
                }
                else if (this.state.containers.has(childId)) {
                    // It's a nested container - check parent relationship
                    // (This would need additional parent tracking in the future)
                }
            }
        }
    }
    /**
     * Invariant: HyperEdges exist only for visible, collapsed containers and connect to visible endpoints
     */
    checkHyperEdgeConsistency(context) {
        const hyperEdges = this.state.allHyperEdges;
        // Get container states
        const visibleCollapsedContainerIds = new Set();
        const hiddenContainerIds = new Set();
        const visibleExpandedContainerIds = new Set();
        for (const [id, container] of this.state.containers) {
            if (container.hidden) {
                hiddenContainerIds.add(id);
            }
            else if (container.collapsed) {
                visibleCollapsedContainerIds.add(id);
            }
            else {
                visibleExpandedContainerIds.add(id);
            }
        }
        // Check that no hyperEdges connect to hidden containers
        for (const hyperEdge of hyperEdges) {
            assert(!hiddenContainerIds.has(hyperEdge.source) && !hiddenContainerIds.has(hyperEdge.target), `${context}: HyperEdge ${hyperEdge.id} should not connect to hidden containers. Source: ${hyperEdge.source}, Target: ${hyperEdge.target}`);
        }
        // Check that no hyperEdges connect to visible but expanded containers
        for (const hyperEdge of hyperEdges) {
            assert(!visibleExpandedContainerIds.has(hyperEdge.source) && !visibleExpandedContainerIds.has(hyperEdge.target), `${context}: HyperEdge ${hyperEdge.id} should not connect to visible but expanded containers. Source: ${hyperEdge.source}, Target: ${hyperEdge.target}`);
        }
        // Check that every hyperEdge connects to at least one visible, collapsed container
        for (const hyperEdge of hyperEdges) {
            const sourceIsVisibleCollapsed = visibleCollapsedContainerIds.has(hyperEdge.source);
            const targetIsVisibleCollapsed = visibleCollapsedContainerIds.has(hyperEdge.target);
            assert(sourceIsVisibleCollapsed || targetIsVisibleCollapsed, `${context}: HyperEdge ${hyperEdge.id} should connect to at least one visible, collapsed container. Source: ${hyperEdge.source}, Target: ${hyperEdge.target}`);
            // The other endpoint should be either a visible, collapsed container OR a visible node
            if (!sourceIsVisibleCollapsed) {
                // Source should be a visible node
                const sourceNode = this.state.getGraphNode(hyperEdge.source);
                assert(sourceNode && !sourceNode.hidden, `${context}: HyperEdge source ${hyperEdge.source} should be a visible node when not a collapsed container`);
            }
            if (!targetIsVisibleCollapsed) {
                // Target should be a visible node  
                const targetNode = this.state.getGraphNode(hyperEdge.target);
                assert(targetNode && !targetNode.hidden, `${context}: HyperEdge target ${hyperEdge.target} should be a visible node when not a collapsed container`);
            }
        }
        // Additional check: verify no hyperEdges exist for containers that shouldn't have them
        this._checkContainerHyperEdgeConstraints(context, hiddenContainerIds, visibleExpandedContainerIds);
    }
    /**
     * Check that hidden and expanded containers have no adjacent hyperEdges
     */
    _checkContainerHyperEdgeConstraints(context, hiddenContainerIds, visibleExpandedContainerIds) {
        const hyperEdges = this.state.allHyperEdges;
        // Hidden containers should have NO hyperEdges
        for (const containerId of hiddenContainerIds) {
            const adjacentHyperEdges = hyperEdges.filter(he => he.source === containerId || he.target === containerId);
            assert(adjacentHyperEdges.length === 0, `${context}: Hidden container ${containerId} should have no adjacent hyperEdges, but found: ${adjacentHyperEdges.map(he => he.id).join(', ')}`);
        }
        // Visible but expanded containers should have NO hyperEdges
        for (const containerId of visibleExpandedContainerIds) {
            const adjacentHyperEdges = hyperEdges.filter(he => he.source === containerId || he.target === containerId);
            assert(adjacentHyperEdges.length === 0, `${context}: Visible but expanded container ${containerId} should have no adjacent hyperEdges, but found: ${adjacentHyperEdges.map(he => he.id).join(', ')}`);
        }
    }
    /**
     * Invariant: Visible collections contain exactly the items that should be visible
     */
    checkCollectionConsistency(context) {
        // Check visible nodes collection
        const allNodes = Array.from(this.state.graphNodes.values());
        const visibleNodes = this.state.visibleNodes;
        const expectedVisibleNodes = allNodes.filter(node => !node.hidden);
        assert.strictEqual(visibleNodes.length, expectedVisibleNodes.length, `${context}: Visible nodes collection size mismatch. Expected: ${expectedVisibleNodes.length}, Actual: ${visibleNodes.length}`);
        // Check visible edges collection
        const allEdges = Array.from(this.state.graphEdges.values());
        const visibleEdges = this.state.visibleEdges;
        const expectedVisibleEdges = allEdges.filter(edge => !edge.hidden);
        assert.strictEqual(visibleEdges.length, expectedVisibleEdges.length, `${context}: Visible edges collection size mismatch. Expected: ${expectedVisibleEdges.length}, Actual: ${visibleEdges.length}`);
        // Check expanded containers collection
        const allContainers = Array.from(this.state.containers.values());
        const expandedContainers = this.state.expandedContainers;
        const expectedExpandedContainers = allContainers.filter(container => !container.collapsed);
        assert.strictEqual(expandedContainers.length, expectedExpandedContainers.length, `${context}: Expanded containers collection size mismatch. Expected: ${expectedExpandedContainers.length}, Actual: ${expandedContainers.length}`);
    }
    /**
     * Invariant: nodeToEdges mapping is consistent with actual edges
     */
    checkNodeToEdgeMappingInvariant(context) {
        const allEdges = Array.from(this.state.graphEdges.values());
        const expectedMapping = new Map();
        // Build expected mapping from actual edges
        for (const edge of allEdges) {
            if (!expectedMapping.has(edge.source)) {
                expectedMapping.set(edge.source, new Set());
            }
            if (!expectedMapping.has(edge.target)) {
                expectedMapping.set(edge.target, new Set());
            }
            expectedMapping.get(edge.source).add(edge.id);
            expectedMapping.get(edge.target).add(edge.id);
        }
        // Check that actual mapping matches expected
        for (const [nodeId, expectedEdgeIds] of expectedMapping) {
            const actualEdgeIds = this.state.nodeToEdges.get(nodeId) || new Set();
            assert.strictEqual(actualEdgeIds.size, expectedEdgeIds.size, `${context}: Node ${nodeId} edge mapping size mismatch. Expected: ${expectedEdgeIds.size}, Actual: ${actualEdgeIds.size}`);
            for (const edgeId of expectedEdgeIds) {
                assert(actualEdgeIds.has(edgeId), `${context}: Node ${nodeId} should be connected to edge ${edgeId}`);
            }
        }
        // Check for extra mappings
        for (const [nodeId, actualEdgeIds] of this.state.nodeToEdges) {
            const expectedEdgeIds = expectedMapping.get(nodeId) || new Set();
            assert.strictEqual(actualEdgeIds.size, expectedEdgeIds.size, `${context}: Node ${nodeId} has unexpected edge mappings`);
        }
    }
    /**
     * Helper: Check if a node is in a collapsed container
     */
    isNodeInCollapsedContainer(nodeId) {
        const containerId = this.state.getNodeContainer(nodeId);
        if (!containerId)
            return false;
        const container = this.state.getContainer(containerId);
        return container && container.collapsed;
    }
}
/**
 * Fuzz test runner
 */
class FuzzTester {
    constructor(testData, testName) {
        this.testData = testData;
        this.testName = testName;
        this.random = new SimpleRandom(OPERATION_SEED);
    }
    /**
     * Run the fuzz test with the given grouping
     */
    async runTest(groupingId = null) {
        console.log(`🎲 Running fuzz test on ${this.testName} with grouping: ${groupingId || 'default'}`);
        // Parse the data
        const result = parseGraphJSON(this.testData, groupingId);
        const state = result.state;
        const checker = new InvariantChecker(state);
        const containers = state.visibleContainers;
        if (containers.length === 0) {
            console.log(`⚠️  No containers found, skipping fuzz test for ${this.testName}`);
            return;
        }
        console.log(`   📊 Initial state: ${state.visibleNodes.length} nodes, ${state.visibleEdges.length} edges, ${containers.length} containers`);
        // Check initial invariants
        checker.checkAll('Initial state');
        let totalOperations = 0;
        // Run fuzz iterations
        for (let iteration = 0; iteration < FUZZ_ITERATIONS; iteration++) {
            const operationsThisIteration = Math.floor(this.random.next() * MAX_OPERATIONS_PER_ITERATION) + 1;
            for (let op = 0; op < operationsThisIteration; op++) {
                const operation = this.generateRandomOperation(state);
                if (operation) {
                    // Record state before operation
                    const beforeState = this.captureStateSnapshot(state);
                    try {
                        // Execute operation
                        this.executeOperation(state, operation);
                        totalOperations++;
                        // Check invariants after operation
                        checker.checkAll(`After operation ${totalOperations}: ${operation.type} ${operation.containerId}`);
                    }
                    catch (error) {
                        console.error(`❌ Operation ${totalOperations} failed:`, operation);
                        console.error(`   Before:`, beforeState);
                        console.error(`   Error:`, error.message);
                        throw error;
                    }
                }
            }
            // Periodic progress update
            if ((iteration + 1) % 20 === 0) {
                console.log(`   ⚡ Completed ${iteration + 1}/${FUZZ_ITERATIONS} iterations (${totalOperations} operations)`);
            }
        }
        console.log(`✅ Fuzz test completed: ${totalOperations} operations, all invariants maintained`);
        // Final state summary
        const finalNodes = state.visibleNodes.length;
        const finalEdges = state.visibleEdges.length;
        const finalHyperEdges = state.allHyperEdges.length;
        const collapsedContainers = Array.from(state.collapsedContainers.keys()).length;
        console.log(`   📈 Final state: ${finalNodes} visible nodes, ${finalEdges} visible edges, ${finalHyperEdges} hyperEdges, ${collapsedContainers} collapsed containers`);
    }
    /**
     * Generate a random collapse or expand operation
     */
    generateRandomOperation(state) {
        const allContainers = state.visibleContainers;
        if (allContainers.length === 0)
            return null;
        const expandedContainers = allContainers.filter(c => !c.collapsed);
        const collapsedContainers = allContainers.filter(c => c.collapsed);
        // Choose operation type based on available containers
        let operationType;
        if (expandedContainers.length === 0) {
            operationType = 'expand';
        }
        else if (collapsedContainers.length === 0) {
            operationType = 'collapse';
        }
        else {
            operationType = this.random.boolean() ? 'collapse' : 'expand';
        }
        // Choose container
        const targetContainers = operationType === 'collapse' ? expandedContainers : collapsedContainers;
        if (targetContainers.length === 0)
            return null;
        const container = this.random.choice(targetContainers);
        return {
            type: operationType,
            containerId: container.id
        };
    }
    /**
     * Execute a collapse or expand operation
     */
    executeOperation(state, operation) {
        if (operation.type === 'collapse') {
            state.collapseContainer(operation.containerId);
        }
        else if (operation.type === 'expand') {
            state.expandContainer(operation.containerId);
        }
    }
    /**
     * Capture a snapshot of the current state for debugging
     */
    captureStateSnapshot(state) {
        return {
            visibleNodes: state.visibleNodes.length,
            visibleEdges: state.visibleEdges.length,
            hyperEdges: state.allHyperEdges.length,
            expandedContainers: state.expandedContainers.length,
            collapsedContainers: Array.from(state.collapsedContainers.keys()).length
        };
    }
}
/**
 * Load test data and run fuzz tests
 */
async function runFuzzTests() {
    console.log('🧪 Starting Fuzz Testing Suite\n');
    console.log('==============================\n');
    const testFiles = ['chat.json', 'paxos.json'];
    for (const filename of testFiles) {
        try {
            console.log(`📁 Loading ${filename}...`);
            const filePath = join(__dirname, '../test-data', filename);
            const jsonData = await readFile(filePath, 'utf-8');
            // Validate the data first
            const validation = validateGraphJSON(jsonData);
            if (!validation.isValid) {
                console.error(`❌ ${filename} failed validation:`, validation.errors);
                continue;
            }
            console.log(`✅ ${filename} loaded and validated (${validation.nodeCount} nodes, ${validation.edgeCount} edges)`);
            // Parse to get available groupings
            const data = JSON.parse(jsonData);
            const groupings = data.hierarchyChoices || [];
            if (groupings.length === 0) {
                console.log(`⚠️  No groupings found in ${filename}, testing with flat structure`);
                const tester = new FuzzTester(data, filename);
                await tester.runTest();
            }
            else {
                console.log(`📊 Found ${groupings.length} groupings: ${groupings.map(g => g.name).join(', ')}`);
                // Test each grouping
                for (const grouping of groupings) {
                    const tester = new FuzzTester(data, filename);
                    await tester.runTest(grouping.id);
                }
            }
            console.log(''); // Blank line between files
        }
        catch (error) {
            console.error(`❌ Error testing ${filename}:`, error.message);
            throw error;
        }
    }
    console.log('🎉 All fuzz tests completed successfully!');
}
/**
 * Run a focused fuzz test on specific data
 */
export async function runFuzzTest(testData, testName = 'Custom', groupingId = null, iterations = FUZZ_ITERATIONS) {
    const originalIterations = FUZZ_ITERATIONS;
    // Temporarily override iterations
    global.FUZZ_ITERATIONS = iterations;
    const tester = new FuzzTester(testData, testName);
    await tester.runTest(groupingId);
    // Restore original
    global.FUZZ_ITERATIONS = originalIterations;
}
// Export components for use in other tests
export { FuzzTester, InvariantChecker };
// Run tests if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
    runFuzzTests().catch(error => {
        console.error('Fuzz testing failed:', error);
        process.exit(1);
    });
}
//# sourceMappingURL=fuzzTest.js.map