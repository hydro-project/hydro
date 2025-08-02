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
  const [debugLogs, setDebugLogs] = React.useState([]);
  const [loadingStatus, setLoadingStatus] = React.useState('Initializing...');

  // Early error boundary
  React.useEffect(() => {
    window.addEventListener('error', (e) => {
      console.error('Global error:', e.error);
      setError(`Global error: ${e.error?.message || 'Unknown error'}`);
    });
    
    window.addEventListener('unhandledrejection', (e) => {
      console.error('Unhandled promise rejection:', e.reason);
      setError(`Promise rejection: ${e.reason?.message || 'Unknown rejection'}`);
    });
  }, []);

  // Capture console logs for debugging (only ELK-related logs)
  React.useEffect(() => {
    const originalLog = console.log;
    const originalError = console.error;
    const originalWarn = console.warn;
    
    function addLog(level, args) {
      const message = args.map(arg => {
        if (typeof arg === 'object' && arg !== null) {
          try {
            // Handle circular references by using a replacer function
            return JSON.stringify(arg, (key, value) => {
              if (key === 'collapseExpandEngine' || key === 'state') {
                return '[Circular Reference]';
              }
              if (typeof value === 'function') {
                return '[Function]';
              }
              if (value instanceof Set) {
                return `[Set: ${value.size} items]`;
              }
              if (value instanceof Map) {
                return `[Map: ${value.size} items]`;
              }
              return value;
            }, 2);
          } catch (e) {
            return `[Object: ${e.message}]`;
          }
        }
        return String(arg);
      }).join(' ');
      
      // Only capture ELK and layout-related logs
      if (message.includes('ELKStateManager') || message.includes('ELKLayoutEngine') || message.includes('Layout')) {
        setDebugLogs(prev => [...prev.slice(-20), { // Keep last 20 relevant logs
          level,
          message,
          timestamp: new Date().toLocaleTimeString()
        }]);
      }
    }
    
    console.log = (...args) => {
      addLog('log', args);
      originalLog.apply(console, args);
    };
    
    console.error = (...args) => {
      addLog('error', args);
      originalError.apply(console, args);
    };
    
    console.warn = (...args) => {
      addLog('warn', args);
      originalWarn.apply(console, args);
    };
    
    return () => {
      console.log = originalLog;
      console.error = originalError;
      console.warn = originalWarn;
    };
  }, []);

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

  // Auto-load test data on startup
  React.useEffect(() => {
    async function loadTestData() {
      // Wait for all components to be loaded
      if (!parseHydroGraphJSON || !createVisualizationState || !GraphFlow) {
        console.log('Components not ready yet, waiting...');
        return;
      }
      
      // Don't auto-load if we already have data
      if (visualizationState) {
        console.log('Visualization state already exists, skipping auto-load');
        return;
      }
      
      try {
        console.log('🔄 Auto-loading chat.json test data...');
        
        // Import the test data
        const chatData = await import('../components/vis/test-data/chat.json');
        console.log('📁 Loaded chat data:', chatData);
        
        const parseResult = parseHydroGraphJSON(chatData.default || chatData);
        console.log('🔧 Parse result:', parseResult);
        
        setVisualizationState(parseResult.state);
        setError(null);
        
        console.log('✅ Chat.json loaded successfully');
      } catch (err) {
        console.error('❌ Error auto-loading test data:', err);
        setError('Failed to auto-load test data: ' + err.message);
      }
    }
    
    loadTestData();
  }, [parseHydroGraphJSON, createVisualizationState, GraphFlow, visualizationState]);

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
    
    // Create a simple test graph with containers
    const testState = createVisualizationState();
    
    console.log('Created visualization state:', testState);
    
    // Add nodes that will go inside containers
    testState.setGraphNode('node1', { 
      label: 'chat_server', 
      style: NODE_STYLES.DEFAULT 
    });
    testState.setGraphNode('node2', { 
      label: 'chat_server', 
      style: NODE_STYLES.DEFAULT 
    });
    testState.setGraphNode('node3', { 
      label: 'chat_server', 
      style: NODE_STYLES.DEFAULT 
    });
    testState.setGraphNode('node4', { 
      label: 'broadcast_bincode', 
      style: NODE_STYLES.DEFAULT 
    });
    testState.setGraphNode('node5', { 
      label: 'broadcast_bincode', 
      style: NODE_STYLES.DEFAULT 
    });
    
    console.log('Added nodes. Visible nodes:', testState.visibleNodes);
    
    // Add containers
    testState.setContainer('container_loc_0', {
      expandedDimensions: { width: 350, height: 450 },
      children: ['node1', 'node2', 'node3', 'node4']
    });
    
    testState.setContainer('container_loc_1', {
      expandedDimensions: { width: 300, height: 200 },
      children: ['node5']
    });
    
    console.log('Added containers. Visible containers:', testState.visibleContainers);
    
    // Add edges
    testState.setGraphEdge('edge1', { 
      source: 'node1', 
      target: 'node2',
      style: EDGE_STYLES.DEFAULT
    });
    testState.setGraphEdge('edge2', { 
      source: 'node2', 
      target: 'node3',
      style: EDGE_STYLES.DEFAULT
    });
    testState.setGraphEdge('edge3', { 
      source: 'node3', 
      target: 'node4',
      style: EDGE_STYLES.DEFAULT
    });

    console.log('Added edges. Visible edges:', testState.visibleEdges);
    
    setVisualizationState(testState);
  }, [createVisualizationState, NODE_STYLES, EDGE_STYLES]);

  const loadChatJson = React.useCallback(async () => {
    if (!parseHydroGraphJSON) return;
    
    try {
      console.log('🔄 Loading chat.json test data...');
      
      const chatData = await import('../components/vis/test-data/chat.json');
      console.log('📁 Loaded chat data:', chatData);
      
      const parseResult = parseHydroGraphJSON(chatData.default || chatData);
      console.log('🔧 Parse result:', parseResult);
      
      setVisualizationState(parseResult.state);
      setError(null);
      
      console.log('✅ Chat.json loaded successfully');
    } catch (err) {
      console.error('❌ Error loading chat.json:', err);
      setError('Failed to load chat.json: ' + err.message);
    }
  }, [parseHydroGraphJSON]);

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
              onClick={loadChatJson}
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
              Load Chat.json Test Data
            </button>
            <button 
              onClick={createTestGraph}
              style={{
                padding: '8px 16px',
                backgroundColor: '#28a745',
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
      
      {/* Debug Panel */}
      <div style={{ 
        marginTop: '24px', 
        padding: '16px', 
        border: '1px solid #ddd', 
        borderRadius: '8px',
        backgroundColor: '#f8f9fa',
        maxHeight: '400px',
        overflow: 'auto'
      }}>
        <h3 style={{ margin: '0 0 12px 0', fontSize: '16px' }}>
          Layout Debug Logs 
          <button 
            onClick={() => setDebugLogs([])}
            style={{
              marginLeft: '12px',
              padding: '4px 8px',
              fontSize: '12px',
              backgroundColor: '#666',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer'
            }}
          >
            Clear
          </button>
        </h3>
        <div style={{ fontSize: '12px', fontFamily: 'monospace' }}>
          {debugLogs.length === 0 ? (
            <div style={{ color: '#666', fontStyle: 'italic' }}>No debug logs yet...</div>
          ) : (
            debugLogs.map((log, index) => (
              <div 
                key={index} 
                style={{ 
                  marginBottom: '8px',
                  padding: '4px',
                  backgroundColor: log.level === 'error' ? '#ffebee' : 
                                   log.level === 'warn' ? '#fff3e0' : '#e8f5e8',
                  borderRadius: '4px',
                  whiteSpace: 'pre-wrap',
                  wordBreak: 'break-word'
                }}
              >
                <span style={{ color: '#666', fontSize: '10px' }}>[{log.timestamp}]</span>{' '}
                <span style={{ 
                  color: log.level === 'error' ? '#d32f2f' : 
                         log.level === 'warn' ? '#f57c00' : '#2e7d32' 
                }}>
                  {log.level.toUpperCase()}
                </span>{' '}
                {log.message}
              </div>
            ))
          )}
        </div>
      </div>
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
