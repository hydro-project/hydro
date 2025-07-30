/**
 * ReactFlow Graph Visualization Component
 */

import React, { useState, useEffect, useCallback } from 'react';
import Layout from '@theme/Layout';
import { useLocation } from '@docusaurus/router';
import { ReactFlowVisualization } from '../components/visualizer/ReactFlowVisualization.js';
import { FileDropZone } from '../components/visualizer/components/FileDropZone.js';
import styles from './visualizer.module.css';

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
