/**
 * Simple Graph Visualization Test Page
 */

import React from 'react';
import Layout from '@theme/Layout';
import BrowserOnly from '@docusaurus/BrowserOnly';
import styles from './visualizer.module.css';

function GraphTestComponent() {
  const [GraphFlow, setGraphFlow] = React.useState(null);
  const [createVisualizationState, setCreateVisualizationState] = React.useState(null);
  const [parseHydroGraphJSON, setParseHydroGraphJSON] = React.useState(null);
  const [FileDropZone, setFileDropZone] = React.useState(null);
  const [NODE_STYLES, setNodeStyles] = React.useState(null);
  const [EDGE_STYLES, setEdgeStyles] = React.useState(null);
  const [error, setError] = React.useState(null);
  const [visualizationState, setVisualizationState] = React.useState(null);

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
      setError(null);
    } catch (err) {
      console.error('Error parsing JSON:', err);
      setError('Failed to parse JSON data: ' + err.message);
    }
  }, [parseHydroGraphJSON]);

  const createTestGraph = React.useCallback(() => {
    if (!createVisualizationState || !NODE_STYLES || !EDGE_STYLES) return;
    
    // Create a simple test graph
    const testState = createVisualizationState();
    
    console.log('Created visualization state:', testState);
    
    // Add nodes
    testState.setGraphNode('node1', { 
      label: 'Start', 
      style: NODE_STYLES.DEFAULT 
    });
    testState.setGraphNode('node2', { 
      label: 'Process', 
      style: NODE_STYLES.HIGHLIGHTED 
    });
    testState.setGraphNode('node3', { 
      label: 'End', 
      style: NODE_STYLES.DEFAULT 
    });
    testState.setGraphNode('node4', { 
      label: 'Error', 
      style: NODE_STYLES.ERROR 
    });
    
    console.log('Added nodes. Visible nodes:', testState.visibleNodes);
    
    // Add edges
    testState.setGraphEdge('edge1', { 
      source: 'node1', 
      target: 'node2',
      style: EDGE_STYLES.DEFAULT
    });
    testState.setGraphEdge('edge2', { 
      source: 'node2', 
      target: 'node3',
      style: EDGE_STYLES.THICK
    });
    testState.setGraphEdge('edge3', { 
      source: 'node2', 
      target: 'node4',
      style: EDGE_STYLES.DASHED
    });

    console.log('Added edges. Visible edges:', testState.visibleEdges);
    
    setVisualizationState(testState);
  }, [createVisualizationState, NODE_STYLES, EDGE_STYLES]);

  if (error) {
    return (
      <div className={styles.container}>
        <div className={styles.errorMessage}>
          Error loading visualization: {error}
        </div>
      </div>
    );
  }

  if (!GraphFlow || !createVisualizationState || !NODE_STYLES || !EDGE_STYLES || !FileDropZone || !parseHydroGraphJSON) {
    return (
      <div className={styles.container}>
        <div className={styles.loadingMessage}>
          Loading visualization components...
        </div>
      </div>
    );
  }

  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <h1>Graph Visualization Test</h1>
        <p>Testing the new GraphFlow component with ReactFlow v12 and ELK layout</p>
        {!visualizationState && (
          <div style={{ marginTop: '16px' }}>
            <button 
              onClick={createTestGraph}
              style={{
                padding: '8px 16px',
                backgroundColor: '#007acc',
                color: 'white',
                border: 'none',
                borderRadius: '4px',
                cursor: 'pointer',
                marginRight: '8px'
              }}
            >
              Create Test Graph
            </button>
            <span style={{ color: '#666' }}>or load a JSON file below</span>
          </div>
        )}
      </div>
      
      {!visualizationState ? (
        <FileDropZone onFileLoad={handleFileLoad} />
      ) : (
        <div className={styles.visualizationContainer}>
          <GraphFlow 
            visualizationState={visualizationState}
            onLayoutComplete={() => console.log('Layout complete!')}
            onError={(err) => console.error('Visualization error:', err)}
            style={{ 
              width: '100%', 
              height: '600px',
              border: '1px solid #ccc',
              borderRadius: '8px'
            }}
          />
        </div>
      )}
      
      {visualizationState && (
        <div className={styles.info}>
          <h3>Current Graph Contains:</h3>
          <ul>
            <li>{visualizationState.visibleNodes.length} visible nodes</li>
            <li>{visualizationState.visibleEdges.length} visible edges</li>
            <li>{visualizationState.visibleContainers.length} visible containers</li>
            <li>{visualizationState.allHyperEdges.length} hyper edges</li>
          </ul>
          <button 
            onClick={() => setVisualizationState(null)}
            style={{
              padding: '8px 16px',
              backgroundColor: '#666',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer',
              marginTop: '16px'
            }}
          >
            Clear Graph
          </button>
        </div>
      )}
    </div>
  );
}

export default function GraphTestPage() {
  return (
    <Layout
      title="Graph Visualization Test"
      description="Test page for the new GraphFlow component">
      <main>
        <BrowserOnly fallback={<div>Loading...</div>}>
          {() => <GraphTestComponent />}
        </BrowserOnly>
      </main>
    </Layout>
  );
}
