/**
 * Visual Test Component for Collapsed Containers
 * Shows two collapsed containers with hyperEdges for debugging rendering issues
 */

import React, { useEffect, useState } from 'react';
import { FlowGraph } from '../render/FlowGraph';
import { createCollapsedContainersTestGraph } from '../utils/testGraphUtils';
import type { VisualizationState } from '../core/VisState';

export function CollapsedContainersVisualTest() {
  const [visState, setVisState] = useState<VisualizationState | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    try {
      console.log('🔧 Creating test graph...');
      const testGraph = createCollapsedContainersTestGraph();
      setVisState(testGraph);
      
      // Log debug info
      console.log('📊 Test Graph Created:');
      console.log('- Visible Containers:', testGraph.visibleContainers.length);
      console.log('- Visible Nodes:', testGraph.visibleNodes.length);
      console.log('- Visible Edges:', testGraph.visibleEdges.length);
      console.log('- Container Details:', testGraph.visibleContainers.map(c => ({
        id: c.id,
        collapsed: c.collapsed,
        hidden: c.hidden,
        width: c.width,
        height: c.height
      })));
      console.log('- Edge Details:', testGraph.visibleEdges.map(e => ({
        id: e.id,
        source: e.source,
        target: e.target,
        hidden: e.hidden
      })));
      
    } catch (err) {
      console.error('❌ Error creating test graph:', err);
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  if (error) {
    return (
      <div style={{ padding: '20px', color: 'red' }}>
        <h3>Error creating test graph:</h3>
        <pre>{error}</pre>
      </div>
    );
  }

  if (!visState) {
    return (
      <div style={{ padding: '20px' }}>
        <h3>Loading test graph...</h3>
      </div>
    );
  }

  return (
    <div style={{ width: '100%', height: '600px', border: '2px solid #ddd', borderRadius: '8px' }}>
      <div style={{ 
        padding: '10px', 
        backgroundColor: '#f5f5f5', 
        borderBottom: '1px solid #ddd',
        fontSize: '14px',
        fontWeight: 'bold'
      }}>
        🧪 Visual Test: Collapsed Containers with HyperEdges
        <div style={{ fontSize: '12px', fontWeight: 'normal', marginTop: '4px' }}>
          Expected: Two collapsed containers (loc_0, loc_1) with bidirectional hyperEdges
        </div>
      </div>
      <div style={{ width: '100%', height: 'calc(100% - 60px)' }}>
        <FlowGraph
          visualizationState={visState}
          eventHandlers={{
            onNodeClick: (nodeId: string) => {
              console.log('🖱️ Node clicked:', nodeId);
              const container = visState.visibleContainers.find(c => c.id === nodeId);
              if (container) {
                console.log('📦 Container details:', {
                  id: container.id,
                  collapsed: container.collapsed,
                  hidden: container.hidden,
                  width: container.width,
                  height: container.height
                });
              }
            },
            onEdgeClick: (edgeId: string) => {
              console.log('🔗 Edge clicked:', edgeId);
              const edge = visState.visibleEdges.find(e => e.id === edgeId);
              if (edge) {
                console.log('🔗 Edge details:', {
                  id: edge.id,
                  source: edge.source,
                  target: edge.target,
                  hidden: edge.hidden
                });
              }
            }
          }}
        />
      </div>
    </div>
  );
}

export default CollapsedContainersVisualTest;
