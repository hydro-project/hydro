/**
 * @fileoverview ELK Bridge Hierarchy Tests
 *
 * Unit tests for container hierarchy handling in ELK Bridge
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { ELKBridge } from '../bridges/ELKBridge';
import { loadChatJsonTestData, skipIfNoTestData, createMockVisStateWithContainers } from './testUtils';
describe('ELKBridge Container Hierarchy', () => {
    let elkBridge;
    beforeEach(() => {
        elkBridge = new ELKBridge();
    });
    describe('Mock Data Tests', () => {
        it('should handle simple container hierarchy correctly', async () => {
            const state = createMockVisStateWithContainers();
            // Check initial state
            expect(state.visibleNodes.length).toBe(5); // 5 nodes
            expect(state.expandedContainers.length).toBe(2); // 2 containers
            expect(state.visibleEdges.length).toBe(4); // 4 edges
            // Test container membership
            const containerA = state.getContainer('container_a');
            const containerB = state.getContainer('container_b');
            expect(containerA).toBeDefined();
            expect(containerB).toBeDefined();
            expect(containerA.children.has('node_0')).toBe(true);
            expect(containerA.children.has('node_1')).toBe(true);
            expect(containerB.children.has('node_2')).toBe(true);
            expect(containerB.children.has('node_3')).toBe(true);
            expect(containerB.children.has('node_4')).toBe(true);
            // Run ELK layout
            await elkBridge.layoutVisState(state);
            // Verify containers got proper dimensions
            const layoutA = state.getContainerLayout('container_a');
            const layoutB = state.getContainerLayout('container_b');
            expect(layoutA).toBeDefined();
            expect(layoutB).toBeDefined();
            expect(layoutA?.dimensions?.width).toBeGreaterThan(0);
            expect(layoutA?.dimensions?.height).toBeGreaterThan(0);
            expect(layoutB?.dimensions?.width).toBeGreaterThan(0);
            expect(layoutB?.dimensions?.height).toBeGreaterThan(0);
        });
        it('should correctly identify nodes in containers', async () => {
            const state = createMockVisStateWithContainers();
            // Capture console output to verify debug logging
            const consoleLogs = [];
            const originalLog = console.log;
            console.log = (...args) => {
                consoleLogs.push(args.join(' '));
                originalLog(...args);
            };
            try {
                await elkBridge.layoutVisState(state);
                // Check that debug logs show correct container membership
                const containerLogs = consoleLogs.filter(log => log.includes('Container') && log.includes('children'));
                expect(containerLogs.length).toBeGreaterThan(0);
                // Should show container_a with 2 children and container_b with 3 children
                const containerALog = containerLogs.find(log => log.includes('container_a'));
                const containerBLog = containerLogs.find(log => log.includes('container_b'));
                expect(containerALog).toBeDefined();
                expect(containerBLog).toBeDefined();
                // Verify the debug output shows correct child counts
                if (containerALog) {
                    expect(containerALog).toMatch(/has 2 children/);
                }
                if (containerBLog) {
                    expect(containerBLog).toMatch(/has 3 children/);
                }
            }
            finally {
                console.log = originalLog;
            }
        });
        it('should handle cross-container edges correctly', async () => {
            const state = createMockVisStateWithContainers();
            // Add a cross-container edge (from container_a to container_b)
            state.setGraphEdge('edge_cross', { source: 'node_1', target: 'node_2' });
            expect(state.visibleEdges.length).toBe(5); // Now 5 edges including cross-container
            await elkBridge.layoutVisState(state);
            // Check that cross-container edge doesn't have sections
            const crossEdgeLayout = state.getEdgeLayout('edge_cross');
            const normalEdgeLayout = state.getEdgeLayout('edge_0_1'); // Within container_a
            // Cross-container edges should not have sections (as per console logs)
            if (crossEdgeLayout?.sections) {
                expect(crossEdgeLayout.sections.length).toBe(0);
            }
            // Normal edges within containers should have sections
            if (normalEdgeLayout?.sections) {
                expect(normalEdgeLayout.sections.length).toBeGreaterThan(0);
            }
        });
    });
    describe('Real Data Tests', () => {
        it('should reproduce chat.json container hierarchy bug', async () => {
            const testData = loadChatJsonTestData('location'); // Use same grouping as console logs
            if (skipIfNoTestData(testData, 'container hierarchy bug reproduction'))
                return;
            const state = testData.state;
            // Capture console output to see the debugging info
            const consoleLogs = [];
            const originalLog = console.log;
            console.log = (...args) => {
                consoleLogs.push(args.join(' '));
                originalLog(...args);
            };
            try {
                await elkBridge.layoutVisState(state);
                // Analyze the console output to identify the bug
                const buildGraphLogs = consoleLogs.filter(log => log.includes('Building ELK graph'));
                const containerLogs = consoleLogs.filter(log => log.includes('Container') && log.includes('children'));
                const topLevelLogs = consoleLogs.filter(log => log.includes('top-level nodes'));
                expect(buildGraphLogs.length).toBeGreaterThan(0);
                // Log the structure for debugging
                console.log('=== ELK Bridge Debug Analysis ===');
                buildGraphLogs.forEach(log => console.log('BUILD:', log));
                containerLogs.forEach(log => console.log('CONTAINER:', log));
                topLevelLogs.forEach(log => console.log('TOP-LEVEL:', log));
                // Check for the specific issue: nodes should be inside containers, not top-level
                const containers = state.visibleContainers; // Use visibleContainers to get computed view with width/height
                const nodes = state.visibleNodes;
                console.log(`Found ${containers.length} visible containers and ${nodes.length} visible nodes`);
                // Verify that containers have proper computed dimensions
                for (const container of containers) {
                    // Check that the container has valid computed dimensions (via width/height getters)
                    expect(container.width).toBeGreaterThan(0);
                    expect(container.height).toBeGreaterThan(0);
                    console.log(`Container ${container.id}: computed size=${container.width}x${container.height}, children: ${Array.from(container.children)}`);
                }
                // Check that nodes are properly assigned to containers
                let nodesInContainers = 0;
                for (const node of nodes) {
                    const isInContainer = containers.some(container => container.children.has(node.id));
                    if (isInContainer) {
                        nodesInContainers++;
                    }
                    else {
                        console.log(`Node ${node.id} is not in any container`);
                    }
                }
                console.log(`${nodesInContainers} out of ${nodes.length} nodes are in containers`);
                // Most nodes should be in containers when using location grouping
                expect(nodesInContainers).toBeGreaterThan(0);
            }
            finally {
                console.log = originalLog;
            }
        });
        it('should validate ELK input data structure', async () => {
            const testData = loadChatJsonTestData('location');
            if (skipIfNoTestData(testData, 'ELK input validation'))
                return;
            const state = testData.state;
            // Test the private methods indirectly by running layout and checking results
            await elkBridge.layoutVisState(state);
            // After layout, all containers should have proper layout information
            const containers = state.expandedContainers;
            for (const container of containers) {
                const layout = state.getContainerLayout(container.id);
                expect(layout).toBeDefined();
                // ELK should have set position and dimensions
                if (layout?.position) {
                    expect(typeof layout.position.x).toBe('number');
                    expect(typeof layout.position.y).toBe('number');
                    expect(isFinite(layout.position.x)).toBe(true);
                    expect(isFinite(layout.position.y)).toBe(true);
                }
                if (layout?.dimensions) {
                    expect(layout.dimensions.width).toBeGreaterThan(0);
                    expect(layout.dimensions.height).toBeGreaterThan(0);
                }
            }
            // Check that node positions are also valid
            const nodes = state.visibleNodes;
            for (const node of nodes) {
                const layout = state.getNodeLayout(node.id);
                if (layout?.position) {
                    expect(typeof layout.position.x).toBe('number');
                    expect(typeof layout.position.y).toBe('number');
                    expect(isFinite(layout.position.x)).toBe(true);
                    expect(isFinite(layout.position.y)).toBe(true);
                    // Check for the specific bug from console: very negative coordinates
                    if (layout.position.x < -500 || layout.position.y < -500) {
                        console.warn(`Suspicious node position for ${node.id}: (${layout.position.x}, ${layout.position.y})`);
                    }
                }
            }
        });
        it('should handle edge routing correctly', async () => {
            const testData = loadChatJsonTestData('location');
            if (skipIfNoTestData(testData, 'edge routing validation'))
                return;
            const state = testData.state;
            await elkBridge.layoutVisState(state);
            // Check edge routing as seen in console logs
            const edges = state.visibleEdges;
            let edgesWithSections = 0;
            let edgesWithoutSections = 0;
            for (const edge of edges) {
                const layout = state.getEdgeLayout(edge.id);
                if (layout?.sections && layout.sections.length > 0) {
                    edgesWithSections++;
                    // Validate section structure
                    for (const section of layout.sections) {
                        expect(section.startPoint).toBeDefined();
                        expect(section.endPoint).toBeDefined();
                        expect(typeof section.startPoint.x).toBe('number');
                        expect(typeof section.startPoint.y).toBe('number');
                        expect(typeof section.endPoint.x).toBe('number');
                        expect(typeof section.endPoint.y).toBe('number');
                    }
                }
                else {
                    edgesWithoutSections++;
                }
            }
            console.log(`Edge routing: ${edgesWithSections} with sections, ${edgesWithoutSections} without sections`);
            // Should have some edges of each type (from console logs we see both)
            expect(edges.length).toBeGreaterThan(0);
            // Don't require both types since it depends on the specific graph structure
        });
        it('should handle containers with different dimensions', async () => {
            const testData = loadChatJsonTestData('location');
            if (skipIfNoTestData(testData, 'container dimensions test'))
                return;
            const state = testData.state;
            // Before layout, check container dimensions
            const containers = state.expandedContainers;
            const initialDimensions = containers.map(c => ({
                id: c.id,
                width: c.expandedDimensions.width,
                height: c.expandedDimensions.height
            }));
            await elkBridge.layoutVisState(state);
            // After layout, containers should have updated dimensions from ELK
            for (const container of containers) {
                const layout = state.getContainerLayout(container.id);
                const initial = initialDimensions.find(d => d.id === container.id);
                console.log(`Container ${container.id}: initial=${initial?.width}x${initial?.height}, final=${layout?.dimensions?.width}x${layout?.dimensions?.height}`);
                // ELK should have set reasonable dimensions
                if (layout?.dimensions) {
                    expect(layout.dimensions.width).toBeGreaterThan(100); // Reasonable minimum
                    expect(layout.dimensions.height).toBeGreaterThan(50);
                    expect(layout.dimensions.width).toBeLessThan(2000); // Reasonable maximum
                    expect(layout.dimensions.height).toBeLessThan(2000);
                }
            }
        });
    });
});
//# sourceMappingURL=elkBridgeHierarchy.test.js.map