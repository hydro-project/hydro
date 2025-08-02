/**
 * Vis System Homepage
 * 
 * Main entry point for the new framework-independent graph visualization system.
 * Features file upload, JSON parsing, and ReactFlow v12 + ELK layout visualization.
 */

import React from 'react';
import Layout from '@theme/Layout';
import BrowserOnly from '@docusaurus/BrowserOnly';

function VisHomepageComponent() {
  const [GraphFlow, setGraphFlow] = React.useState(null);
  const [createVisualizationState, setCreateVisualizationState] = React.useState(null);
  const [parseHydroGraphJSON, setParseHydroGraphJSON] = React.useState(null);
  const [FileDropZone, setFileDropZone] = React.useState(null);
  const [NODE_STYLES, setNodeStyles] = React.useState(null);
  const [EDGE_STYLES, setEdgeStyles] = React.useState(null);
  const [error, setError] = React.useState(null);
  const [visualizationState, setVisualizationState] = React.useState(null);
  const [parseMetadata, setParseMetadata] = React.useState(null);

  React.useEffect(() => {
    // Dynamically import the visualization components
    const loadComponents = async () => {
      try {
        const visStateModule = await import('../components/vis/core/VisState');
        const graphFlowModule = await import('../components/vis/render/GraphFlow');
        const constantsModule = await import('../components/vis/shared/constants');
        const parserModule = await import('../components/vis/core/JSONParser');
        const componentsModule = await import('../components/vis/components');
        
        setCreateVisualizationState(() => visStateModule.createVisualizationState);
        setGraphFlow(() => graphFlowModule.GraphFlow);
        setParseHydroGraphJSON(() => parserModule.parseHydroGraphJSON);
        setFileDropZone(() => componentsModule.FileDropZone);
        setNodeStyles(constantsModule.NODE_STYLES);
        setEdgeStyles(constantsModule.EDGE_STYLES);
      } catch (err) {
        console.error('Failed to load visualization components:', err);
        setError(err.message);
      }
    };

    loadComponents();
  }, []);

  const handleFileLoad = React.useCallback((jsonData) => {
    try {
      console.log('Loading JSON data:', jsonData);
      
      // Parse the JSON data into a VisualizationState
      const parseResult = parseHydroGraphJSON(jsonData);
      console.log('Parse result:', parseResult);
      
      setVisualizationState(parseResult.state);
      setParseMetadata(parseResult.metadata);
      setError(null);
    } catch (err) {
      console.error('Error parsing JSON:', err);
      setError('Failed to parse JSON data: ' + err.message);
    }
  }, [parseHydroGraphJSON]);

  const createSampleGraph = React.useCallback(() => {
    if (!createVisualizationState || !NODE_STYLES || !EDGE_STYLES) return;
    
    // Create a sample demonstration graph
    const sampleState = createVisualizationState();
    
    // Add sample nodes with different styles
    sampleState.setGraphNode('input', { 
      label: 'Data Input', 
      style: NODE_STYLES.DEFAULT 
    });
    sampleState.setGraphNode('transform', { 
      label: 'Transform', 
      style: NODE_STYLES.HIGHLIGHTED 
    });
    sampleState.setGraphNode('filter', { 
      label: 'Filter', 
      style: NODE_STYLES.WARNING 
    });
    sampleState.setGraphNode('output', { 
      label: 'Output', 
      style: NODE_STYLES.DEFAULT 
    });
    sampleState.setGraphNode('error_handler', { 
      label: 'Error Handler', 
      style: NODE_STYLES.ERROR 
    });
    
    // Add sample edges with different styles
    sampleState.setGraphEdge('edge1', { 
      source: 'input', 
      target: 'transform',
      style: EDGE_STYLES.DEFAULT
    });
    sampleState.setGraphEdge('edge2', { 
      source: 'transform', 
      target: 'filter',
      style: EDGE_STYLES.THICK
    });
    sampleState.setGraphEdge('edge3', { 
      source: 'filter', 
      target: 'output',
      style: EDGE_STYLES.DEFAULT
    });
    sampleState.setGraphEdge('edge4', { 
      source: 'transform', 
      target: 'error_handler',
      style: EDGE_STYLES.DASHED
    });
    
    setVisualizationState(sampleState);
    setParseMetadata({
      selectedGrouping: null,
      nodeCount: 5,
      edgeCount: 4,
      containerCount: 0,
      availableGroupings: []
    });
  }, [createVisualizationState, NODE_STYLES, EDGE_STYLES]);

  if (error) {
    return (
      <div style={{ padding: '40px 20px', textAlign: 'center' }}>
        <div style={{ 
          background: '#ffebee', 
          border: '1px solid #f44336', 
          color: '#c62828', 
          padding: '16px', 
          borderRadius: '4px',
          maxWidth: '600px',
          margin: '0 auto'
        }}>
          <strong>Error:</strong> {error}
          <br />
          <button 
            onClick={() => window.location.reload()}
            style={{
              marginTop: '12px',
              padding: '6px 12px',
              backgroundColor: '#f44336',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer',
              fontSize: '12px'
            }}
          >
            Reload
          </button>
        </div>
      </div>
    );
  }

  if (!GraphFlow || !createVisualizationState || !NODE_STYLES || !EDGE_STYLES || !FileDropZone || !parseHydroGraphJSON) {
    return (
      <div style={{ 
        display: 'flex', 
        alignItems: 'center', 
        justifyContent: 'center', 
        minHeight: '400px',
        fontSize: '14px',
        color: '#999'
      }}>
        Loading...
      </div>
    );
  }

  return (
    <div style={{ minHeight: '100vh', padding: '40px 20px' }}>
      {!visualizationState ? (
        <div style={{ maxWidth: '800px', margin: '0 auto', textAlign: 'center' }}>
          <h1 style={{ 
            fontSize: '32px', 
            marginBottom: '16px',
            color: '#333'
          }}>
            Hydro Graph Visualizer
          </h1>
          <p style={{ 
            fontSize: '16px', 
            marginBottom: '40px',
            color: '#666'
          }}>
            Interactive graph visualization with ReactFlow and ELK layout
          </p>
          
          <div style={{ marginBottom: '32px' }}>
            <button 
              onClick={createSampleGraph}
              style={{
                padding: '12px 24px',
                backgroundColor: '#007acc',
                color: 'white',
                border: 'none',
                borderRadius: '6px',
                cursor: 'pointer',
                fontSize: '14px',
                marginRight: '16px'
              }}
            >
              Try Sample
            </button>
            <span style={{ color: '#999' }}>or drag a JSON file below</span>
          </div>

          <FileDropZone onFileLoad={handleFileLoad} />
        </div>
      ) : (
        // Visualization Section
        <div style={{ maxWidth: '1200px', margin: '0 auto' }}>
          {/* Simple Controls */}
          <div style={{ 
            display: 'flex', 
            justifyContent: 'space-between', 
            alignItems: 'center',
            marginBottom: '20px',
            padding: '12px 0',
            borderBottom: '1px solid #eee'
          }}>
            <div style={{ fontSize: '14px', color: '#666' }}>
              {parseMetadata && (
                <>
                  {parseMetadata.nodeCount} nodes, {parseMetadata.edgeCount} edges
                </>
              )}
            </div>
            <button 
              onClick={() => {
                setVisualizationState(null);
                setParseMetadata(null);
                setError(null);
              }}
              style={{
                padding: '6px 12px',
                backgroundColor: '#f5f5f5',
                color: '#666',
                border: '1px solid #ddd',
                borderRadius: '4px',
                cursor: 'pointer',
                fontSize: '12px'
              }}
            >
              Reset
            </button>
          </div>

          {/* Visualization */}
          <div style={{ 
            height: '600px',
            border: '1px solid #ddd',
            borderRadius: '4px',
            overflow: 'hidden'
          }}>
            <GraphFlow 
              visualizationState={visualizationState}
              onLayoutComplete={() => console.log('Layout complete!')}
              onError={(err) => {
                console.error('Visualization error:', err);
                setError('Visualization error: ' + err.message);
              }}
              style={{ width: '100%', height: '100%' }}
            />
          </div>
        </div>
      )}
    </div>
  );
}

export default function VisHomepage() {
  return (
    <Layout
      title="Hydro Graph Visualizer"
      description="Interactive graph visualization with ReactFlow v12 and ELK layout">
      <main>
        <BrowserOnly fallback={<div>Loading...</div>}>
          {() => <VisHomepageComponent />}
        </BrowserOnly>
      </main>
    </Layout>
  );
}
