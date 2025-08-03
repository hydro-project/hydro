/**
 * @fileoverview Working Demo Page - REAL ELK + ReactFlow Integration
 * 
 * This is a fully functional demo that:
 * 1. Loads real graph data (including chat.json subset)
 * 2. Runs actual ELK layout via our ELKBridge
 * 3. Renders with actual ReactFlow via our ReactFlowBridge
 * 4. Demonstrates the hyperedge layout fix in action
 */

import React, { useState, useEffect } from 'react';
import { ReactFlow, Background, Controls, MiniMap } from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { createVisualizationState } from '../core/VisState';
import { createVisualizationEngine } from '../core/VisualizationEngine';
import { 
  loadGraphFromJSON, 
  SAMPLE_CHAT_SUBSET, 
  SAMPLE_COMPLEX_GRAPH 
} from '../utils/EnhancedJSONLoader';
import type { SimpleGraphData } from '../utils/EnhancedJSONLoader';
import type { ReactFlowData } from '../bridges/ReactFlowBridge';

type DemoDataset = 'chat' | 'complex' | 'simple';

export function SimpleDemoPage(): JSX.Element {
  const [visState] = useState(() => createVisualizationState());
  const [engine] = useState(() => createVisualizationEngine(visState, {
    autoLayout: false, // Manual control for demo
    enableLogging: true
  }));
  
  const [reactFlowData, setReactFlowData] = useState<ReactFlowData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedDataset, setSelectedDataset] = useState<DemoDataset>('chat');
  const [engineState, setEngineState] = useState(engine.getState());

  // Listen to engine state changes
  useEffect(() => {
    engine.onStateChange('demo-page', (state) => {
      setEngineState(state);
      console.log('🔄 Engine state changed:', state.phase);
    });
    
    return () => {
      engine.removeStateListener('demo-page');
    };
  }, [engine]);

  // Get dataset
  const getDataset = (dataset: DemoDataset): SimpleGraphData => {
    switch (dataset) {
      case 'chat': return SAMPLE_CHAT_SUBSET;
      case 'complex': return SAMPLE_COMPLEX_GRAPH;
      case 'simple': return {
        nodes: [
          { id: 'a', label: 'Node A', style: 'default' },
          { id: 'b', label: 'Node B', style: 'default' },
          { id: 'c', label: 'Node C', style: 'default' }
        ],
        edges: [
          { id: 'e1', source: 'a', target: 'b', style: 'default' },
          { id: 'e2', source: 'b', target: 'c', style: 'default' }
        ]
      };
      default: return SAMPLE_CHAT_SUBSET;
    }
  };

  // Load data and run the complete pipeline
  const runVisualization = async (dataset: DemoDataset) => {
    try {
      setLoading(true);
      setError(null);
      setReactFlowData(null);
      
      const data = getDataset(dataset);
      
      console.log('� Step 1: Loading data into VisState...');
      loadGraphFromJSON(data, visState);
      
      console.log('📊 Step 2: Running ELK layout...');
      await engine.runLayout(); // This calls our ELKBridge!
      
      console.log('🎨 Step 3: Converting to ReactFlow...');
      const result = engine.getReactFlowData(); // This calls our ReactFlowBridge!
      
      setReactFlowData(result);
      setLoading(false);
      
      console.log('✅ Complete visualization pipeline finished!');
      console.log('📊 Result:', {
        nodes: result.nodes.length,
        edges: result.edges.length,
        hyperEdges: result.edges.filter(e => e.type === 'hyper').length
      });
      
    } catch (err) {
      console.error('❌ Visualization pipeline failed:', err);
      setError(err instanceof Error ? err.message : String(err));
      setLoading(false);
    }
  };

  // Load initial data
  useEffect(() => {
    runVisualization('chat');
  }, []);

  const handleDatasetChange = (dataset: DemoDataset) => {
    setSelectedDataset(dataset);
    runVisualization(dataset);
  };

  return (
    <div style={{ padding: '20px', height: '100vh', display: 'flex', flexDirection: 'column' }}>
      {/* Header */}
      <div style={{ marginBottom: '20px' }}>
        <h1 style={{ margin: '0 0 16px 0', color: '#333' }}>
          🚀 Real ELK + ReactFlow Demo
        </h1>
        
        <div style={{ 
          display: 'flex', 
          gap: '16px', 
          alignItems: 'center',
          padding: '16px',
          background: '#f8f9fa',
          borderRadius: '8px',
          border: '1px solid #e9ecef',
          flexWrap: 'wrap'
        }}>
          <div style={{ fontSize: '14px', color: '#666', minWidth: '120px' }}>
            <strong>Dataset:</strong>
          </div>
          
          <button
            onClick={() => handleDatasetChange('simple')}
            style={{
              padding: '8px 16px',
              background: selectedDataset === 'simple' ? '#007bff' : '#6c757d',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer',
              fontSize: '14px'
            }}
          >
            Simple (3 nodes)
          </button>
          
          <button
            onClick={() => handleDatasetChange('chat')}
            style={{
              padding: '8px 16px',
              background: selectedDataset === 'chat' ? '#007bff' : '#6c757d',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer',
              fontSize: '14px'
            }}
          >
            Chat System (10 nodes)
          </button>
          
          <button
            onClick={() => handleDatasetChange('complex')}
            style={{
              padding: '8px 16px',
              background: selectedDataset === 'complex' ? '#007bff' : '#6c757d',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer',
              fontSize: '14px'
            }}
          >
            Complex Pipeline (10 nodes)
          </button>
          
          {/* Status indicator */}
          <div style={{ 
            marginLeft: 'auto',
            padding: '8px 12px',
            background: engineState.phase === 'displayed' ? '#d4edda' : 
                       engineState.phase === 'error' ? '#f8d7da' :
                       loading ? '#fff3cd' : '#e2e3e5',
            borderRadius: '4px',
            fontSize: '12px',
            color: engineState.phase === 'displayed' ? '#155724' :
                   engineState.phase === 'error' ? '#721c24' :
                   loading ? '#856404' : '#6c757d'
          }}>
            Status: {loading ? 'Processing...' : engineState.phase} | 
            Layouts: {engineState.layoutCount}
          </div>
        </div>
        
        {/* Architecture info */}
        <div style={{
          marginTop: '12px',
          padding: '12px',
          background: '#e8f4fd',
          borderRadius: '6px',
          fontSize: '12px',
          color: '#0056b3'
        }}>
          <strong>🔥 REAL Bridge Architecture:</strong> VisState → ELKBridge (with hyperedges!) → ReactFlowBridge → Display
          {reactFlowData && (
            <span style={{ marginLeft: '16px' }}>
              | Nodes: {reactFlowData.nodes.length} 
              | Edges: {reactFlowData.edges.length}
              | Hyperedges: {reactFlowData.edges.filter(e => e.type === 'hyper').length}
            </span>
          )}
        </div>
      </div>

      {/* Error display */}
      {error && (
        <div style={{
          padding: '16px',
          background: '#ffe6e6',
          border: '1px solid #ff9999',
          borderRadius: '8px',
          marginBottom: '20px',
          color: '#cc0000'
        }}>
          <strong>Pipeline Error:</strong> {error}
          <div style={{ marginTop: '8px' }}>
            <button 
              onClick={() => runVisualization(selectedDataset)}
              style={{
                padding: '4px 12px',
                background: '#dc3545',
                color: 'white',
                border: 'none',
                borderRadius: '4px',
                cursor: 'pointer',
                fontSize: '12px'
              }}
            >
              Retry Pipeline
            </button>
          </div>
        </div>
      )}

      {/* Visualization area */}
      <div style={{ 
        flex: 1, 
        border: '2px solid #ddd', 
        borderRadius: '8px',
        overflow: 'hidden',
        position: 'relative'
      }}>
        {loading ? (
          <div style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100%',
            background: '#f5f5f5'
          }}>
            <div style={{ textAlign: 'center' }}>
              <div style={{ fontSize: '48px', marginBottom: '16px' }}>
                {engineState.phase === 'laying_out' ? '�' : 
                 engineState.phase === 'rendering' ? '🎨' : '�🔄'}
              </div>
              <div style={{ fontSize: '18px', color: '#666', marginBottom: '8px' }}>
                {engineState.phase === 'laying_out' && 'Running ELK Layout Engine...'}
                {engineState.phase === 'rendering' && 'Converting to ReactFlow...'}
                {engineState.phase === 'initial' && 'Initializing Pipeline...'}
              </div>
              <div style={{ fontSize: '14px', color: '#999' }}>
                {engineState.phase === 'laying_out' && '🔥 Including ALL edges (regular + hyperedges)'}
                {engineState.phase === 'rendering' && '🌉 Translating coordinates via bridges'}
                {engineState.phase === 'initial' && 'Loading data into VisState...'}
              </div>
            </div>
          </div>
        ) : reactFlowData ? (
          <>
            {/* Stats overlay */}
            <div style={{
              position: 'absolute',
              top: '10px',
              left: '10px',
              zIndex: 1000,
              background: 'rgba(255, 255, 255, 0.95)',
              padding: '10px 12px',
              borderRadius: '6px',
              border: '1px solid #ddd',
              fontSize: '12px',
              color: '#666',
              boxShadow: '0 2px 4px rgba(0,0,0,0.1)'
            }}>
              <div><strong>Real ELK + ReactFlow</strong></div>
              <div>Nodes: {reactFlowData.nodes.length}</div>
              <div>Edges: {reactFlowData.edges.length}</div>
              <div>Hyperedges: {reactFlowData.edges.filter(e => e.type === 'hyper').length}</div>
              <div>Layouts: {engineState.layoutCount}</div>
            </div>
            
            {/* Re-layout button */}
            <div style={{
              position: 'absolute',
              top: '10px',
              right: '10px',
              zIndex: 1000
            }}>
              <button
                onClick={() => runVisualization(selectedDataset)}
                style={{
                  padding: '8px 12px',
                  background: '#28a745',
                  color: 'white',
                  border: 'none',
                  borderRadius: '4px',
                  cursor: 'pointer',
                  fontSize: '12px',
                  boxShadow: '0 2px 4px rgba(0,0,0,0.1)'
                }}
              >
                🔄 Re-run Pipeline
              </button>
            </div>
            
            {/* Actual ReactFlow */}
            <ReactFlow
              nodes={reactFlowData.nodes}
              edges={reactFlowData.edges}
              fitView
              fitViewOptions={{ padding: 0.1, maxZoom: 1.2 }}
              attributionPosition="bottom-left"
              nodesDraggable={true}
              nodesConnectable={false}
              elementsSelectable={true}
              panOnDrag={true}
              zoomOnScroll={true}
              minZoom={0.1}
              maxZoom={2}
            >
              <Background 
                color="#ccc"
              />
              <Controls />
              <MiniMap 
                nodeColor="#666"
                nodeStrokeWidth={2}
                position="bottom-right"
              />
            </ReactFlow>
          </>
        ) : (
          <div style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100%',
            background: '#f9f9f9'
          }}>
            <div style={{ textAlign: 'center', color: '#666' }}>
              <div style={{ fontSize: '48px', marginBottom: '16px' }}>📊</div>
              <div>Select a dataset above to run the pipeline</div>
            </div>
          </div>
        )}
      </div>
      
      {/* Footer with technical details */}
      <div style={{
        marginTop: '16px',
        padding: '12px',
        background: '#f8f9fa',
        borderRadius: '6px',
        fontSize: '12px',
        color: '#666'
      }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', flexWrap: 'wrap', gap: '16px' }}>
          <div>
            <strong>💡 Hyperedge Fix:</strong> Try "Complex Pipeline" to see collapsed containers 
            connecting to external nodes via hyperedges (no overlaps!)
          </div>
          <div>
            <strong>⚡ Pipeline:</strong> {engineState.phase} 
            {engineState.lastUpdate && ` (${new Date(engineState.lastUpdate).toLocaleTimeString()})`}
          </div>
        </div>
      </div>
    </div>
  );
}

/**
 * Export for easy integration
 */
export default SimpleDemoPage;
