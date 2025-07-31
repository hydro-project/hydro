/**
 * ReactFlow Graph Visualization Component
 */

import React, { useState, useEffect, useCallback } from 'react';
import Layout from '@theme/Layout';
import { useLocation } from '@docusaurus/router';
import { ReactFlowVisualization } from '../components/visualizer/ReactFlowVisualization.js';
import { FileDropZone } from '../components/visualizer/components/FileDropZone.js';
import styles from './visualizer.module.css';

// Global ResizeObserver error suppression - must be at the top level
const suppressResizeObserverErrors = () => {
  const originalError = window.console.error;
  const originalOnError = window.onerror;
  const originalOnUnhandledRejection = window.onunhandledrejection;
  const resizeObserverErrorPattern = /ResizeObserver loop completed with undelivered notifications/;
  
  // Suppress console.error
  window.console.error = (...args) => {
    if (args[0] && resizeObserverErrorPattern.test(args[0])) {
      return;
    }
    originalError.apply(console, args);
  };
  
  // Suppress window.onerror (catches uncaught errors that webpack overlay shows)
  window.onerror = (message, source, lineno, colno, error) => {
    if (message && resizeObserverErrorPattern.test(message)) {
      return true; // Suppress the error
    }
    if (originalOnError) {
      return originalOnError(message, source, lineno, colno, error);
    }
    return false;
  };
  
  // Suppress unhandled promise rejections
  window.onunhandledrejection = (event) => {
    if (event.reason && resizeObserverErrorPattern.test(event.reason.message || event.reason)) {
      event.preventDefault();
      return;
    }
    if (originalOnUnhandledRejection) {
      originalOnUnhandledRejection(event);
    }
  };
  
  // Also try to suppress addEventListener error events
  const originalAddEventListener = window.addEventListener;
  window.addEventListener = function(type, listener, options) {
    if (type === 'error') {
      const wrappedListener = function(event) {
        if (event.message && resizeObserverErrorPattern.test(event.message)) {
          event.preventDefault();
          event.stopPropagation();
          return;
        }
        if (typeof listener === 'function') {
          listener.call(this, event);
        } else if (listener && typeof listener.handleEvent === 'function') {
          listener.handleEvent(event);
        }
      };
      return originalAddEventListener.call(this, type, wrappedListener, options);
    }
    return originalAddEventListener.call(this, type, listener, options);
  };
  
  return () => {
    window.console.error = originalError;
    window.onerror = originalOnError;
    window.onunhandledrejection = originalOnUnhandledRejection;
    window.addEventListener = originalAddEventListener;
  };
};

// Apply error suppression immediately when module loads
const restoreErrorHandling = suppressResizeObserverErrors();

export default function Visualizer() {
  const location = useLocation();
  const [graphData, setGraphData] = useState(null);
  const [error, setError] = useState(null);
  const [toolbarControls, setToolbarControls] = useState(null);

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
              <div className={styles.toolbarControls}>
                {toolbarControls}
                <button onClick={handleClearData} className={styles.clearButton}>
                  Load New Graph
                </button>
              </div>
            </div>
            <ReactFlowVisualization 
              graphData={graphData} 
              onControlsReady={setToolbarControls}
            />
          </div>
        ) : (
          <FileDropZone onFileLoad={handleFileLoad} hasData={!!graphData} />
        )}
      </div>
    </Layout>
  );
}
