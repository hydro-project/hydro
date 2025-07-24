import React, { useState, useCallback, useEffect, useRef } from 'react';
import Layout from '@theme/Layout';
import { useLocation } from '@docusaurus/router';
import styles from './visualizer.module.css';

// We'll use the same CDN approach as the template for ReactFlow
// This avoids needing to modify package.json for now
let ReactFlowComponents = null;

const loadReactFlow = async () => {
  // Don't load React - use Docusaurus's React instead
  // Just ensure window.React points to the same React that Docusaurus uses
  if (!window.React) {
    window.React = React;
  }
  
  // Don't load ReactDOM - use Docusaurus's ReactDOM instead
  if (!window.ReactDOM) {
    const ReactDOM = await import('react-dom');
    window.ReactDOM = ReactDOM;
  }
  
  // Load ReactFlow
  if (!window.ReactFlow) {
    const reactFlowScript = document.createElement('script');
    reactFlowScript.src = 'https://unpkg.com/reactflow@11.11.4/dist/umd/index.js';
    document.head.appendChild(reactFlowScript);
    
    await new Promise((resolve) => {
      reactFlowScript.onload = resolve;
    });
  }
  
  // Load CSS
  if (!document.querySelector('link[href*="reactflow"]')) {
    const link = document.createElement('link');
    link.rel = 'stylesheet';
    link.href = 'https://unpkg.com/reactflow@11.11.4/dist/style.css';
    document.head.appendChild(link);
  }
  
  // Extract components like template.html does
  const ReactFlowLib = window.ReactFlow;
  const { 
    default: ReactFlowComponent, 
    Controls, 
    MiniMap, 
    Background, 
    useNodesState, 
    useEdgesState, 
    addEdge, 
    applyNodeChanges, 
    applyEdgeChanges 
  } = ReactFlowLib;
  
  ReactFlowComponents = {
    ReactFlow: ReactFlowComponent,
    Controls,
    MiniMap,
    Background,
    useNodesState,
    useEdgesState,
    addEdge,
    applyNodeChanges,
    applyEdgeChanges
  };
  
  return ReactFlowComponents;
};

function ReactFlowVisualization({ graphData }) {
  const [reactFlowReady, setReactFlowReady] = useState(false);
  const [initialNodes, setInitialNodes] = useState([]);
  const [initialEdges, setInitialEdges] = useState([]);

  // Load ReactFlow when component mounts
  useEffect(() => {
    loadReactFlow().then(() => {
      console.log('ReactFlow loaded and components extracted');
      setReactFlowReady(true);
    }).catch((error) => {
      console.error('Failed to load ReactFlow:', error);
    });
  }, []);

  // Process graph data when ReactFlow is loaded and data changes
  useEffect(() => {
    if (!reactFlowReady || !graphData) return;

    // Convert the graph data to ReactFlow format
    const processedNodes = (graphData.nodes || []).map(node => ({
      ...node,
      position: node.position || { x: 0, y: 0 }
    }));

    const processedEdges = (graphData.edges || []).map(edge => ({
      ...edge,
      type: edge.type || 'smoothstep',
      animated: edge.animated || false
    }));

    setInitialNodes(processedNodes);
    setInitialEdges(processedEdges);
  }, [reactFlowReady, graphData]);

  if (!reactFlowReady) {
    return (
      <div className={styles.loading}>
        Loading ReactFlow visualization...
      </div>
    );
  }

  if (!ReactFlowComponents) {
    return (
      <div className={styles.loading}>
        ReactFlow not available. Check console for errors.
      </div>
    );
  }

  return (
    <ReactFlowInner 
      nodes={initialNodes} 
      edges={initialEdges} 
    />
  );
}

// Inner component that uses ReactFlow hooks
function ReactFlowInner({ nodes, edges }) {
  // Use the global ReactFlowComponents like template.html does
  const { ReactFlow, Controls, MiniMap, Background, useNodesState, useEdgesState, addEdge } = ReactFlowComponents;

  const [currentNodes, setNodes, onNodesChange] = useNodesState(nodes);
  const [currentEdges, setEdges, onEdgesChange] = useEdgesState(edges);

  // Update nodes and edges when props change
  useEffect(() => {
    setNodes(nodes);
  }, [nodes, setNodes]);

  useEffect(() => {
    setEdges(edges);
  }, [edges, setEdges]);

  const onConnect = useCallback((connection) => {
    setEdges((eds) => addEdge(connection, eds));
  }, [setEdges, addEdge]);

  return (
    <div className={styles.reactflowWrapper}>
      <ReactFlow
        nodes={currentNodes}
        edges={currentEdges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        fitView
        attributionPosition="bottom-left"
      >
        <Controls />
        <MiniMap />
        <Background />
      </ReactFlow>
    </div>
  );
}

function FileDropZone({ onFileLoad, hasData }) {
  const [isDragOver, setIsDragOver] = useState(false);

  const handleDragOver = useCallback((e) => {
    e.preventDefault();
    setIsDragOver(true);
  }, []);

  const handleDragLeave = useCallback((e) => {
    e.preventDefault();
    setIsDragOver(false);
  }, []);

  const handleDrop = useCallback((e) => {
    e.preventDefault();
    setIsDragOver(false);
    
    const files = Array.from(e.dataTransfer.files);
    const jsonFile = files.find(file => file.name.endsWith('.json'));
    
    if (jsonFile) {
      const reader = new FileReader();
      reader.onload = (event) => {
        try {
          const data = JSON.parse(event.target.result);
          onFileLoad(data);
        } catch (error) {
          alert('Invalid JSON file: ' + error.message);
        }
      };
      reader.readAsText(jsonFile);
    } else {
      alert('Please drop a JSON file');
    }
  }, [onFileLoad]);

  const handleFileInput = useCallback((e) => {
    const file = e.target.files[0];
    if (file && file.name.endsWith('.json')) {
      const reader = new FileReader();
      reader.onload = (event) => {
        try {
          const data = JSON.parse(event.target.result);
          onFileLoad(data);
        } catch (error) {
          alert('Invalid JSON file: ' + error.message);
        }
      };
      reader.readAsText(file);
    }
  }, [onFileLoad]);

  if (hasData) return null;

  return (
    <div 
      className={`${styles.dropZone} ${isDragOver ? styles.dragOver : ''}`}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      <div className={styles.dropContent}>
        <h3>Hydro Graph Visualizer</h3>
        <p>Drop a Hydro ReactFlow JSON file here or click to select</p>
        <input 
          type="file" 
          accept=".json"
          onChange={handleFileInput}
          className={styles.fileInput}
          id="file-input"
        />
        <label htmlFor="file-input" className={styles.fileInputLabel}>
          Choose File
        </label>
        <div className={styles.helpText}>
          <p>Generate JSON files using:</p>
          <code>built_flow.reactflow_to_file("graph.json")</code>
        </div>
      </div>
    </div>
  );
}

export default function Visualizer() {
  const location = useLocation();
  const [graphData, setGraphData] = useState(null);
  const [error, setError] = useState(null);

  // Check for URL-encoded data on component mount
  useEffect(() => {
    // Parse URL hash for data parameter (like mermaid.live)
    const hash = location.hash;
    if (hash.startsWith('#data=')) {
      try {
        const encodedData = hash.substring(6); // Remove '#data='
        // Convert Base64URL to regular Base64
        const base64 = encodedData.replace(/-/g, '+').replace(/_/g, '/');
        // Add padding if needed
        const padded = base64 + '==='.slice(0, (4 - base64.length % 4) % 4);
        const jsonString = atob(padded); // Base64 decode
        const data = JSON.parse(jsonString);
        setGraphData(data);
      } catch (error) {
        setError('Failed to decode graph data from URL: ' + error.message);
      }
    }
  }, [location.hash]);

  const handleFileLoad = useCallback((data) => {
    setGraphData(data);
    setError(null);
  }, []);

  const handleClearData = useCallback(() => {
    setGraphData(null);
    setError(null);
    // Clear URL hash
    window.history.replaceState(null, null, window.location.pathname);
  }, []);

  return (
    <Layout
      title="Graph Visualizer"
      description="Interactive ReactFlow visualization for Hydro graphs"
    >
      <div className={styles.container}>
        {error && (
          <div className={styles.error}>
            <strong>Error:</strong> {error}
            <button onClick={() => setError(null)} className={styles.closeError}>×</button>
          </div>
        )}
        
        {graphData ? (
          <div className={styles.visualizationContainer}>
            <div className={styles.toolbar}>
              <h2>Hydro Graph Visualization</h2>
              <button onClick={handleClearData} className={styles.clearButton}>
                Load New Graph
              </button>
            </div>
            <ReactFlowVisualization graphData={graphData} />
          </div>
        ) : (
          <FileDropZone onFileLoad={handleFileLoad} hasData={!!graphData} />
        )}
      </div>
    </Layout>
  );
}
