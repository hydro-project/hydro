/**
 * Vis System v4 - Graph Visualization Homepage
 * 
 * Latest version of the graph visualization system using visualizer-v4 architecture.
 * Features file upload, JSON parsing, and ReactFlow v12 + ELK layout visualization.
 * 
 * This is the current/latest version - previous versions available at /vis3, /visualizer
 */

import React from 'react';
import Layout from '@theme/Layout';
import BrowserOnly from '@docusaurus/BrowserOnly';
import { useLocation } from '@docusaurus/router';

function VisV4Component() {
  const location = useLocation();
  const [createVisualizationState, setCreateVisualizationState] = React.useState(null);
  const [FlowGraph, setFlowGraph] = React.useState(null);
  const [parseGraphJSON, setParseGraphJSON] = React.useState(null);
  const [getAvailableGroupings, setGetAvailableGroupings] = React.useState(null);
  const [validateGraphJSON, setValidateGraphJSON] = React.useState(null);
  const [NODE_STYLES, setNodeStyles] = React.useState(null);
  const [EDGE_STYLES, setEdgeStyles] = React.useState(null);
  const [error, setError] = React.useState(null);
  const [loading, setLoading] = React.useState(true);
  const [currentVisualizationState, setCurrentVisualizationState] = React.useState(null);
  const [graphData, setGraphData] = React.useState(null);

  // Load components on mount
  React.useEffect(() => {
    const loadComponents = async () => {
      try {
        console.log('Loading visualizer-v4 components...');
        
        // Import individual v4 components (matching the working pattern from vis3)
        const visStateModule = await import('../components/visualizer-v4/core/VisState.ts');
        const FlowGraphModule = await import('../components/visualizer-v4/render/FlowGraph.tsx');
        const constantsModule = await import('../components/visualizer-v4/core/constants.ts');
        const parserModule = await import('../components/visualizer-v4/core/JSONParser.ts');
        const layoutModule = await import('../components/visualizer-v4/layout/index.ts');
        
        console.log('Loaded modules:', { visStateModule, FlowGraphModule, constantsModule, parserModule, layoutModule });
        
        // Set up the imported components (following the working vis3 pattern)
        setCreateVisualizationState(() => visStateModule.createVisualizationState);
        setFlowGraph(() => FlowGraphModule.FlowGraph);
        setParseGraphJSON(() => parserModule.parseGraphJSON);
        setGetAvailableGroupings(() => parserModule.getAvailableGroupings);
        setValidateGraphJSON(() => parserModule.validateGraphJSON);
        // Note: Don't store class constructors (ELKLayoutEngine, etc.) in React state
        setNodeStyles(constantsModule.NODE_STYLES);
        setEdgeStyles(constantsModule.EDGE_STYLES);
        
        console.log('✅ All v4 components loaded successfully');
        setLoading(false);
        setError(null);
        
      } catch (err) {
        console.error('❌ Failed to load visualizer-v4 components:', err);
        setError(`Failed to load v4 components: ${err.message}`);
        setLoading(false);
      }
    };

    loadComponents();
  }, []);

  // Handle URL data parameter (for sharing graphs via URL)
  React.useEffect(() => {
    if (!parseGraphJSON || !createVisualizationState || loading) return;
    
    const urlParams = new URLSearchParams(location.search);
    const dataParam = urlParams.get('data');
    
    if (dataParam && !currentVisualizationState) {
      try {
        console.log('Loading graph data from URL parameter...');
        
        // Decode the base64 data
        const jsonString = atob(dataParam);
        const jsonData = JSON.parse(jsonString);
        
        console.log('Parsed URL data:', jsonData);
        
        // Validate and parse the JSON
        const validationResult = validateGraphJSON(jsonData);
        if (!validationResult.isValid) {
          throw new Error(`Invalid graph data: ${validationResult.errors.join(', ')}`);
        }
        
        const parsedData = parseGraphJSON(jsonData);
        setCurrentVisualizationState(parsedData);
        setGraphData(jsonData);
        
        console.log('✅ Successfully loaded graph from URL');
        
      } catch (err) {
        console.error('❌ Error loading graph from URL:', err);
        setError(`Failed to load graph from URL: ${err.message}`);
      }
    }
  }, [location.search, parseGraphJSON, validateGraphJSON, createVisualizationState, loading, currentVisualizationState]);

  // File upload handler
  const handleFileLoad = React.useCallback((jsonData) => {
    if (!parseGraphJSON || !validateGraphJSON) {
      setError('Components not loaded yet');
      return;
    }
    
    try {
      console.log('Processing uploaded file:', jsonData);
      
      // Validate the JSON
      const validationResult = validateGraphJSON(jsonData);
      if (!validationResult.isValid) {
        throw new Error(`Invalid graph data: ${validationResult.errors.join(', ')}`);
      }
      
      // Parse the JSON
      const parsedData = parseGraphJSON(jsonData);
      setCurrentVisualizationState(parsedData);
      setGraphData(jsonData);
      setError(null);
      
      console.log('✅ File loaded successfully');
      
    } catch (err) {
      console.error('❌ Error processing file:', err);
      setError(`Failed to process file: ${err.message}`);
    }
  }, [parseGraphJSON, validateGraphJSON]);

  // Create test graph
  const createTestGraph = React.useCallback(() => {
    if (!createVisualizationState || !NODE_STYLES || !EDGE_STYLES) return;
    
    try {
      console.log('Creating test graph...');
      
      const visState = createVisualizationState();
      
      // Add some test nodes
      visState.setGraphNode('node1', { 
        label: 'Source Node', 
        style: NODE_STYLES.DEFAULT,
        position: { x: 0, y: 0 }
      });
      
      visState.setGraphNode('node2', { 
        label: 'Transform Node', 
        style: NODE_STYLES.DEFAULT,
        position: { x: 200, y: 100 }
      });
      
      visState.setGraphNode('node3', { 
        label: 'Sink Node', 
        style: NODE_STYLES.DEFAULT,
        position: { x: 400, y: 0 }
      });
      
      // Add edges
      visState.setGraphEdge('edge1', {
        source: 'node1',
        target: 'node2',
        style: EDGE_STYLES.DEFAULT
      });
      
      visState.setGraphEdge('edge2', {
        source: 'node2',
        target: 'node3',
        style: EDGE_STYLES.DEFAULT
      });
      
      setCurrentVisualizationState(visState);
      setError(null);
      
      console.log('✅ Test graph created');
      
    } catch (err) {
      console.error('❌ Error creating test graph:', err);
      setError(`Failed to create test graph: ${err.message}`);
    }
  }, [createVisualizationState, NODE_STYLES, EDGE_STYLES]);

  // Render loading state
  if (loading) {
    return (
      <div style={{ 
        display: 'flex', 
        justifyContent: 'center', 
        alignItems: 'center', 
        height: '400px',
        fontSize: '18px',
        color: '#666'
      }}>
        Loading visualizer-v4 components...
      </div>
    );
  }

  // Render error state
  if (error) {
    return (
      <div style={{ 
        padding: '24px',
        backgroundColor: '#ffebee',
        border: '1px solid #f44336',
        borderRadius: '8px',
        margin: '24px',
        color: '#d32f2f'
      }}>
        <h3 style={{ margin: '0 0 12px 0', color: '#d32f2f' }}>Error</h3>
        <p style={{ margin: 0 }}>{error}</p>
        <button 
          onClick={() => window.location.reload()}
          style={{
            marginTop: '16px',
            padding: '8px 16px',
            backgroundColor: '#f44336',
            color: 'white',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer'
          }}
        >
          Reload Page
        </button>
      </div>
    );
  }

  // Render main interface
  return (
    <div style={{ padding: '24px', maxWidth: '1200px', margin: '0 auto' }}>
      <div style={{ marginBottom: '24px' }}>
        <h1 style={{ margin: '0 0 8px 0' }}>Graph Visualizer v4</h1>
        <p style={{ margin: '0 0 16px 0', color: '#666' }}>
          Latest version of the Hydro graph visualization system with enhanced architecture and performance.
        </p>
        
        {!currentVisualizationState && (
          <div style={{ marginBottom: '24px' }}>
            <button 
              onClick={createTestGraph}
              style={{
                padding: '12px 24px',
                backgroundColor: '#28a745',
                color: 'white',
                border: 'none',
                borderRadius: '6px',
                cursor: 'pointer',
                marginRight: '12px',
                fontSize: '16px'
              }}
            >
              Create Test Graph
            </button>
            <span style={{ color: '#666' }}>or upload a JSON file below</span>
          </div>
        )}
      </div>

      {!currentVisualizationState ? (
        <div style={{
          border: '2px dashed #ccc',
          borderRadius: '8px',
          padding: '48px',
          textAlign: 'center',
          backgroundColor: '#fafafa'
        }}>
          <p style={{ margin: '0 0 16px 0', fontSize: '18px', color: '#666' }}>
            Drop a JSON file here or click to select
          </p>
          <input
            type="file"
            accept=".json"
            onChange={(e) => {
              const file = e.target.files[0];
              if (file) {
                const reader = new FileReader();
                reader.onload = (event) => {
                  try {
                    const jsonData = JSON.parse(event.target.result);
                    handleFileLoad(jsonData);
                  } catch (err) {
                    setError(`Failed to parse JSON file: ${err.message}`);
                  }
                };
                reader.readAsText(file);
              }
            }}
            style={{
              padding: '12px',
              fontSize: '16px',
              border: '1px solid #ccc',
              borderRadius: '4px'
            }}
          />
        </div>
      ) : (
        <div style={{ marginBottom: '24px' }}>
          <div style={{
            height: '600px',
            border: '1px solid #ddd',
            borderRadius: '8px',
            backgroundColor: 'white'
          }}>
            {FlowGraph && (
              <FlowGraph 
                visualizationState={currentVisualizationState}
                onLayoutComplete={() => console.log('Layout complete!')}
                onError={(err) => {
                  console.error('Visualization error:', err);
                  setError(`Visualization error: ${err.message}`);
                }}
                style={{ width: '100%', height: '100%' }}
              />
            )}
          </div>
          
          <div style={{ marginTop: '16px', display: 'flex', gap: '12px', alignItems: 'center' }}>
            <button 
              onClick={() => {
                setCurrentVisualizationState(null);
                setGraphData(null);
                setError(null);
              }}
              style={{
                padding: '8px 16px',
                backgroundColor: '#6c757d',
                color: 'white',
                border: 'none',
                borderRadius: '4px',
                cursor: 'pointer'
              }}
            >
              Clear Graph
            </button>
            
            {graphData && (
              <button 
                onClick={() => {
                  const dataString = JSON.stringify(graphData);
                  const encoded = btoa(dataString);
                  const url = `${window.location.origin}${window.location.pathname}?data=${encoded}`;
                  navigator.clipboard.writeText(url).then(() => {
                    alert('Share URL copied to clipboard!');
                  });
                }}
                style={{
                  padding: '8px 16px',
                  backgroundColor: '#007bff',
                  color: 'white',
                  border: 'none',
                  borderRadius: '4px',
                  cursor: 'pointer'
                }}
              >
                Copy Share URL
              </button>
            )}
            
            <span style={{ color: '#666', fontSize: '14px' }}>
              Powered by visualizer-v4 architecture
            </span>
          </div>
        </div>
      )}

      <div style={{
        marginTop: '32px',
        padding: '16px',
        backgroundColor: '#f8f9fa',
        borderRadius: '8px',
        fontSize: '14px',
        color: '#666'
      }}>
        <h4 style={{ margin: '0 0 8px 0' }}>About this version:</h4>
        <ul style={{ margin: 0, paddingLeft: '20px' }}>
          <li>Latest visualizer-v4 architecture with enhanced performance</li>
          <li>Improved bridge architecture eliminating layout bugs</li>
          <li>Full ReactFlow v12 + ELK layout integration</li>
          <li>Support for URL sharing of graphs</li>
          <li>Previous versions available at <a href="/vis3">/vis3</a> and <a href="/visualizer">/visualizer</a></li>
        </ul>
      </div>
    </div>
  );
}

export default function VisPage() {
  return (
    <Layout
      title="Graph Visualizer v4"
      description="Latest Hydro graph visualization system with enhanced architecture and performance">
      <main>
        <BrowserOnly fallback={<div>Loading...</div>}>
          {() => <VisV4Component />}
        </BrowserOnly>
      </main>
    </Layout>
  );
}
