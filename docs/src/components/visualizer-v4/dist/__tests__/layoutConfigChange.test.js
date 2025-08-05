/**
 * Test layout configuration changes
 */
import { describe, it, expect } from 'vitest';
import { createVisualizationState } from '../core/VisState';
import { ELKBridge } from '../bridges/ELKBridge';
describe('Layout Configuration Changes', () => {
    it('should accept different layout algorithms', async () => {
        const visState = createVisualizationState();
        // Add some test data
        visState.setGraphNode('node1', { label: 'Node 1', hidden: false, style: 'default' });
        visState.setGraphNode('node2', { label: 'Node 2', hidden: false, style: 'default' });
        visState.setGraphEdge('edge1', { source: 'node1', target: 'node2', hidden: false, style: 'default' });
        // Test different algorithms
        const algorithms = ['mrtree', 'layered', 'force', 'stress', 'radial'];
        for (const algorithm of algorithms) {
            console.log(`Testing algorithm: ${algorithm}`);
            const bridge = new ELKBridge({ algorithm });
            await bridge.layoutVisState(visState);
            // Verify that the layout completed without errors
            // (positions should be set on nodes)
            const node1Layout = visState.getNodeLayout('node1');
            const node2Layout = visState.getNodeLayout('node2');
            expect(node1Layout).toBeDefined();
            expect(node2Layout).toBeDefined();
            expect(typeof node1Layout?.position?.x).toBe('number');
            expect(typeof node1Layout?.position?.y).toBe('number');
            console.log(`✅ Algorithm ${algorithm} completed successfully`);
        }
    });
    it('should update layout config dynamically', async () => {
        const visState = createVisualizationState();
        // Add test data
        visState.setGraphNode('node1', { label: 'Node 1', hidden: false, style: 'default' });
        visState.setGraphNode('node2', { label: 'Node 2', hidden: false, style: 'default' });
        const bridge = new ELKBridge({ algorithm: 'mrtree' });
        // Initial layout
        await bridge.layoutVisState(visState);
        const initialPosition = visState.getNodeLayout('node1')?.position;
        // Update config and re-layout
        bridge.updateLayoutConfig({ algorithm: 'force' });
        await bridge.layoutVisState(visState);
        const newPosition = visState.getNodeLayout('node1')?.position;
        // Both should have valid positions (though they may be the same for this simple case)
        expect(initialPosition).toBeDefined();
        expect(newPosition).toBeDefined();
        expect(typeof newPosition?.x).toBe('number');
        expect(typeof newPosition?.y).toBe('number');
        console.log('✅ Dynamic layout config update completed successfully');
    });
});
//# sourceMappingURL=layoutConfigChange.test.js.map