/**
 * Test grouping/hierarchy change functionality
 */
import { describe, it, expect } from 'vitest';
import { parseGraphJSON } from '../core/JSONParser';
describe('Hierarchy Change Functionality', () => {
    const mockGraphData = {
        nodes: [
            { id: 'node1', label: 'Node 1', nodeType: 'Source' },
            { id: 'node2', label: 'Node 2', nodeType: 'Transform' },
            { id: 'node3', label: 'Node 3', nodeType: 'Sink' }
        ],
        edges: [
            { id: 'edge1', source: 'node1', target: 'node2' },
            { id: 'edge2', source: 'node2', target: 'node3' }
        ],
        hierarchies: [
            {
                id: 'grouping1',
                name: 'Grouping 1',
                groups: {
                    'container1': ['node1', 'node2']
                }
            },
            {
                id: 'grouping2',
                name: 'Grouping 2',
                groups: {
                    'container2': ['node2', 'node3']
                }
            }
        ]
    };
    it('should parse graph with default grouping', () => {
        const result = parseGraphJSON(mockGraphData);
        expect(result.state).toBeDefined();
        expect(result.metadata.selectedGrouping).toBe('grouping1'); // First available
        expect(result.metadata.availableGroupings).toHaveLength(2);
        expect(result.metadata.containerCount).toBe(1); // One container from grouping1
    });
    it('should parse graph with specific grouping', () => {
        const result = parseGraphJSON(mockGraphData, 'grouping2');
        expect(result.state).toBeDefined();
        expect(result.metadata.selectedGrouping).toBe('grouping2');
        expect(result.metadata.containerCount).toBe(1); // One container from grouping2
        // Verify the right container was created
        const container = result.state.getContainer('container2');
        expect(container).toBeDefined();
        expect(Array.from(container.children)).toEqual(['node2', 'node3']);
    });
    it('should switch between groupings with different container structures', () => {
        // Parse with first grouping
        const result1 = parseGraphJSON(mockGraphData, 'grouping1');
        const container1 = result1.state.getContainer('container1');
        expect(container1).toBeDefined();
        expect(Array.from(container1.children)).toEqual(['node1', 'node2']);
        // Re-parse with second grouping (simulating hierarchy change)
        const result2 = parseGraphJSON(mockGraphData, 'grouping2');
        const container2 = result2.state.getContainer('container2');
        expect(container2).toBeDefined();
        expect(Array.from(container2.children)).toEqual(['node2', 'node3']);
        // Verify the first container doesn't exist in the second grouping
        const nonExistentContainer = result2.state.getContainer('container1');
        expect(nonExistentContainer).toBeUndefined();
    });
});
//# sourceMappingURL=hierarchyChange.test.js.map