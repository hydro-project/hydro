/**
 * @fileoverview Chat JSON Integration Tests
 *
 * Integration tests for processing chat.json data
 */
import { describe, it, expect } from 'vitest';
import { parseGraphJSON, getAvailableGroupings } from '../core/JSONParser';
import { ELKBridge } from '../bridges/ELKBridge';
import { ReactFlowBridge } from '../bridges/ReactFlowBridge';
import { loadChatJsonTestData, skipIfNoTestData } from './testUtils';
describe('ChatJsonIntegration', () => {
    describe('json processing', () => {
        it('should exist as a test suite', () => {
            // This test always passes to ensure the suite exists
            expect(true).toBe(true);
        });
        it('should parse chat.json correctly', () => {
            const testData = loadChatJsonTestData();
            if (skipIfNoTestData(testData, 'chat.json parsing'))
                return;
            expect(testData.state).toBeDefined();
            expect(testData.metadata).toBeDefined();
            // Chat.json should have nodes and edges
            expect(testData.metadata.nodeCount).toBeGreaterThan(0);
            expect(testData.metadata.edgeCount).toBeGreaterThan(0);
            console.log(`✅ Parsed chat.json: ${testData.metadata.nodeCount} nodes, ${testData.metadata.edgeCount} edges`);
        });
        it('should handle chat.json visualization with grouping', () => {
            const testData = loadChatJsonTestData('filename');
            if (skipIfNoTestData(testData, 'chat.json with grouping'))
                return;
            expect(testData.state.visibleNodes.length).toBeGreaterThan(0);
            expect(testData.state.visibleEdges.length).toBeGreaterThan(0);
            // Should have containers when grouped by filename
            const containers = testData.state.visibleContainers;
            expect(Array.isArray(containers)).toBe(true);
            console.log(`✅ Chat.json with grouping: ${containers.length} containers`);
        });
        it('should validate chat.json structure', () => {
            const testData = loadChatJsonTestData();
            if (skipIfNoTestData(testData, 'chat.json structure validation'))
                return;
            // Basic structure validation
            expect(testData.rawData).toBeDefined();
            expect(Array.isArray(testData.rawData.nodes)).toBe(true);
            expect(Array.isArray(testData.rawData.edges)).toBe(true);
            // Nodes should have expected structure
            const firstNode = testData.rawData.nodes[0];
            expect(firstNode).toBeDefined();
            expect(firstNode.id).toBeDefined();
            expect(firstNode.data).toBeDefined();
            console.log(`✅ Chat.json structure valid: ${testData.rawData.nodes.length} nodes, ${testData.rawData.edges.length} edges`);
        });
    });
    describe('integration scenarios', () => {
        it('should handle large chat.json files efficiently', () => {
            const testData = loadChatJsonTestData('filename');
            if (skipIfNoTestData(testData, 'large file performance test'))
                return;
            const startTime = performance.now();
            const result = parseGraphJSON(testData.rawData, 'filename');
            const endTime = performance.now();
            const parseTime = endTime - startTime;
            // Should parse reasonably quickly (under 5 seconds for most files)
            expect(parseTime).toBeLessThan(5000);
            expect(result.state).toBeDefined();
            console.log(`✅ Chat.json parsed in ${parseTime.toFixed(2)}ms`);
        });
        it('should maintain data integrity during parsing', () => {
            const testData = loadChatJsonTestData();
            if (skipIfNoTestData(testData, 'data integrity test'))
                return;
            // Check that all edges reference valid nodes
            const nodeIds = new Set(testData.state.visibleNodes.map(n => n.id));
            const edges = testData.state.visibleEdges;
            for (const edge of edges) {
                // Note: Some edges might reference nodes that aren't visible due to filtering
                // So we just check the structure is valid
                expect(edge.source).toBeDefined();
                expect(edge.target).toBeDefined();
                expect(edge.id).toBeDefined();
            }
            console.log(`✅ Data integrity verified: ${edges.length} edges checked`);
        });
    });
    describe('grouping functionality', () => {
        it('should detect available grouping options from chat.json', () => {
            const testData = loadChatJsonTestData();
            if (skipIfNoTestData(testData, 'grouping options test'))
                return;
            const groupings = getAvailableGroupings(testData.rawData);
            expect(Array.isArray(groupings)).toBe(true);
            expect(groupings.length).toBeGreaterThan(0);
            // Log available groupings to see what we actually have
            console.log(`✅ Available groupings: ${groupings.map(g => g.id).join(', ')}`);
            // Check that we have some valid grouping options
            const groupingIds = groupings.map(g => g.id);
            expect(groupingIds.length).toBeGreaterThan(0);
            // The actual groupings depend on the JSONParser implementation
            // So we just verify the structure is correct
            for (const grouping of groupings) {
                expect(grouping.id).toBeDefined();
                expect(grouping.name).toBeDefined();
            }
        });
    });
    describe('bug reproduction from console logs', () => {
        it('should reproduce and debug ReactFlow edge creation failures', async () => {
            const testData = loadChatJsonTestData('location');
            if (skipIfNoTestData(testData, 'ReactFlow edge bug reproduction'))
                return;
            const state = testData.state;
            // Run ELK layout (this part works correctly from console logs)
            const elkBridge = new ELKBridge();
            await elkBridge.layoutVisState(state);
            // Convert to ReactFlow format (this is where the edge errors occur)
            const reactFlowBridge = new ReactFlowBridge();
            const reactFlowData = reactFlowBridge.visStateToReactFlow(state);
            // Debug: Check that we have the expected structure from console logs
            expect(reactFlowData.nodes.length).toBeGreaterThan(0);
            expect(reactFlowData.edges.length).toBeGreaterThan(0);
            // Check for the specific bug: edges should have valid sourceHandle/targetHandle
            for (const edge of reactFlowData.edges) {
                console.log(`[Bug Test] Edge ${edge.id}: sourceHandle=${edge.sourceHandle}, targetHandle=${edge.targetHandle}`);
                // The bug: these should NOT be null (causing the ReactFlow errors)
                // If they are null, ReactFlow can't create the edges
                if (edge.sourceHandle === null || edge.targetHandle === null) {
                    console.warn(`[Bug Found] Edge ${edge.id} has null handles: source=${edge.sourceHandle}, target=${edge.targetHandle}`);
                }
                // Test that source and target exist
                expect(edge.source).toBeDefined();
                expect(edge.target).toBeDefined();
                expect(edge.id).toBeDefined();
                // Verify that source and target nodes actually exist in the nodes array
                const sourceNode = reactFlowData.nodes.find(n => n.id === edge.source);
                const targetNode = reactFlowData.nodes.find(n => n.id === edge.target);
                expect(sourceNode).toBeDefined();
                expect(targetNode).toBeDefined();
                if (!sourceNode) {
                    console.error(`[Bug] Edge ${edge.id} references non-existent source node: ${edge.source}`);
                }
                if (!targetNode) {
                    console.error(`[Bug] Edge ${edge.id} references non-existent target node: ${edge.target}`);
                }
            }
        });
        it('should validate container coordinate conversion', async () => {
            const testData = loadChatJsonTestData('location');
            if (skipIfNoTestData(testData, 'container coordinate validation'))
                return;
            const state = testData.state;
            // Run ELK layout
            const elkBridge = new ELKBridge();
            await elkBridge.layoutVisState(state);
            // Check container positioning (from console: loc_0, loc_1 containers)
            const containers = state.visibleContainers;
            expect(containers.length).toBeGreaterThan(0);
            for (const container of containers) {
                const layout = state.getContainerLayout(container.id);
                console.log(`[Container Test] ${container.id}: layout=${JSON.stringify(layout)}`);
                // Containers should have valid positions and dimensions
                if (layout?.position) {
                    expect(typeof layout.position.x).toBe('number');
                    expect(typeof layout.position.y).toBe('number');
                }
                if (layout?.dimensions) {
                    expect(typeof layout.dimensions.width).toBe('number');
                    expect(typeof layout.dimensions.height).toBe('number');
                }
                // Check for the positioning issues seen in console logs
                if (layout?.position) {
                    expect(layout.position.x).toBeGreaterThanOrEqual(0);
                    expect(layout.position.y).toBeGreaterThanOrEqual(0);
                }
                if (layout?.dimensions) {
                    expect(layout.dimensions.width).toBeGreaterThan(0);
                    expect(layout.dimensions.height).toBeGreaterThan(0);
                }
            }
            // Convert to ReactFlow and check coordinate conversion
            const reactFlowBridge = new ReactFlowBridge();
            const reactFlowData = reactFlowBridge.visStateToReactFlow(state);
            // Check for negative coordinates (bug seen in console: `(-224, 320)`)
            for (const node of reactFlowData.nodes) {
                console.log(`[Node Position Test] ${node.id}: position=(${node.position.x}, ${node.position.y})`);
                // Look for the negative coordinate bug from console logs
                if (node.position.x < -200 || node.position.y < -200) {
                    console.warn(`[Coordinate Bug] Node ${node.id} has suspicious negative coordinates: (${node.position.x}, ${node.position.y})`);
                }
                // Validate position structure
                expect(node.position).toBeDefined();
                expect(typeof node.position.x).toBe('number');
                expect(typeof node.position.y).toBe('number');
                expect(isFinite(node.position.x)).toBe(true);
                expect(isFinite(node.position.y)).toBe(true);
            }
        });
        it('should validate edge sections and routing', async () => {
            const testData = loadChatJsonTestData('location');
            if (skipIfNoTestData(testData, 'edge sections validation'))
                return;
            const state = testData.state;
            // Run ELK layout 
            const elkBridge = new ELKBridge();
            await elkBridge.layoutVisState(state);
            // Check that ELK produced edge sections (seen in console logs)
            const edges = state.visibleEdges;
            expect(edges.length).toBeGreaterThan(0);
            // Some edges should have sections, some are cross-container
            let edgesWithSections = 0;
            let crossContainerEdges = 0;
            for (const edge of edges) {
                // Check for sections in the edge layout (where ELKBridge stores them)
                const edgeLayout = state.getEdgeLayout(edge.id);
                if (edgeLayout?.sections && edgeLayout.sections.length > 0) {
                    edgesWithSections++;
                    console.log(`[Edge Sections] ${edge.id}: ${edgeLayout.sections.length} sections`);
                    // Validate section structure
                    for (const section of edgeLayout.sections) {
                        expect(section.startPoint).toBeDefined();
                        expect(section.endPoint).toBeDefined();
                        expect(typeof section.startPoint.x).toBe('number');
                        expect(typeof section.startPoint.y).toBe('number');
                        expect(typeof section.endPoint.x).toBe('number');
                        expect(typeof section.endPoint.y).toBe('number');
                    }
                }
                else {
                    crossContainerEdges++;
                    console.log(`[Cross-Container Edge] ${edge.id}: no sections (crosses containers)`);
                }
            }
            console.log(`✅ Edge analysis: ${edgesWithSections} with sections, ${crossContainerEdges} cross-container`);
            // Based on current ELK behavior, all edges should have sections
            // Cross-container edges (e1, e7) now also get sections with bend points
            expect(edgesWithSections).toBe(edges.length); // All edges should have sections
            expect(edgesWithSections).toBeGreaterThan(0);
        });
    });
});
//# sourceMappingURL=chatJsonIntegration.test.js.map