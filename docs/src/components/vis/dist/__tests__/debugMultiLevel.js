import { createVisualizationState } from '../dist/VisState.js';
import assert from 'assert';
function testMultiLevelDebug() {
    console.log('Testing multi-level debug...');
    const state = createVisualizationState();
    // Create the exact same hierarchy as the failing test
    state.setGraphNode('node1', { label: 'Node 1' });
    state.setGraphNode('node2', { label: 'Node 2' });
    state.setGraphNode('node3', { label: 'Node 3' });
    state.setGraphNode('external', { label: 'External Node' });
    state.setContainer('level1', { children: ['node1'] });
    state.setContainer('level2', { children: ['level1', 'node2'] });
    state.setContainer('level3', { children: ['level2', 'node3'] });
    state.setGraphEdge('edge1-2', { source: 'node1', target: 'node2' });
    state.setGraphEdge('edge2-3', { source: 'node2', target: 'node3' });
    state.setGraphEdge('edge1-ext', { source: 'node1', target: 'external' });
    console.log('\nInitial state:');
    console.log('edge1-2 hidden:', state.getEdgeHidden('edge1-2'));
    console.log('edge2-3 hidden:', state.getEdgeHidden('edge2-3'));
    console.log('edge1-ext hidden:', state.getEdgeHidden('edge1-ext'));
    // Bottom-up collapse
    console.log('\n=== Bottom-up collapse ===');
    console.log('\nCollapsing level1...');
    state.collapseContainer('level1');
    console.log('edge1-2 hidden:', state.getEdgeHidden('edge1-2'));
    console.log('edge2-3 hidden:', state.getEdgeHidden('edge2-3'));
    console.log('edge1-ext hidden:', state.getEdgeHidden('edge1-ext'));
    console.log('HyperEdges:', state.getHyperEdges().map(he => `${he.source} -> ${he.target}`));
    console.log('\nCollapsing level2...');
    state.collapseContainer('level2');
    console.log('edge1-2 hidden:', state.getEdgeHidden('edge1-2'));
    console.log('edge2-3 hidden:', state.getEdgeHidden('edge2-3'));
    console.log('edge1-ext hidden:', state.getEdgeHidden('edge1-ext'));
    console.log('HyperEdges:', state.getHyperEdges().map(he => `${he.source} -> ${he.target}`));
    console.log('\nCollapsing level3...');
    state.collapseContainer('level3');
    console.log('edge1-2 hidden:', state.getEdgeHidden('edge1-2'));
    console.log('edge2-3 hidden:', state.getEdgeHidden('edge2-3'));
    console.log('edge1-ext hidden:', state.getEdgeHidden('edge1-ext'));
    console.log('HyperEdges:', state.getHyperEdges().map(he => `${he.source} -> ${he.target}`));
    // Top-down expand
    console.log('\n=== Top-down expand ===');
    console.log('\nExpanding level3...');
    state.expandContainer('level3');
    console.log('AFTER expanding level3:');
    console.log('edge1-2 hidden:', state.getEdgeHidden('edge1-2'));
    console.log('edge2-3 hidden:', state.getEdgeHidden('edge2-3'));
    console.log('edge1-ext hidden:', state.getEdgeHidden('edge1-ext'));
    console.log('HyperEdges:', state.getHyperEdges().map(he => `${he.source} -> ${he.target}`));
    console.log('Node visibility:', {
        node1: state.getNodeHidden('node1'),
        node2: state.getNodeHidden('node2'),
        node3: state.getNodeHidden('node3'),
        external: state.getNodeHidden('external')
    });
    console.log('Raw node objects:', {
        node1: state.getGraphNode('node1'),
        node2: state.getGraphNode('node2'),
        node3: state.getGraphNode('node3'),
        external: state.getGraphNode('external')
    });
    console.log('Container states:', {
        level1: { collapsed: state.getContainerCollapsed('level1'), hidden: state.getContainerHidden('level1') },
        level2: { collapsed: state.getContainerCollapsed('level2'), hidden: state.getContainerHidden('level2') },
        level3: { collapsed: state.getContainerCollapsed('level3'), hidden: state.getContainerHidden('level3') }
    });
    console.log('\nTrying to expand level2 (should be no-op since already expanded by level3)...');
    state.expandContainer('level2');
    console.log('AFTER trying to expand level2:');
    console.log('Node visibility:', {
        node1: state.getNodeHidden('node1'),
        node2: state.getNodeHidden('node2'),
        node3: state.getNodeHidden('node3'),
        external: state.getNodeHidden('external')
    });
    console.log('\nTrying to expand level1 (should be no-op since already expanded by level3)...');
    state.expandContainer('level1');
    console.log('AFTER trying to expand level1:');
    console.log('Node visibility:', {
        node1: state.getNodeHidden('node1'),
        node2: state.getNodeHidden('node2'),
        node3: state.getNodeHidden('node3'),
        external: state.getNodeHidden('external')
    });
    console.log('\nFinal edge states should all be false (visible):');
    console.log('edge1-2 hidden:', state.getEdgeHidden('edge1-2'), '(should be false)');
    console.log('edge2-3 hidden:', state.getEdgeHidden('edge2-3'), '(should be false)');
    console.log('edge1-ext hidden:', state.getEdgeHidden('edge1-ext'), '(should be false)');
}
testMultiLevelDebug();
//# sourceMappingURL=debugMultiLevel.js.map