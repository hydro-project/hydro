/**
 * ReactFlow Visualization Component
 * 
 * Main wrapper component that loads external libraries and renders the graph
 */

import React, { useState, useEffect, useRef, useMemo } from 'react';
import { loadExternalLibraries } from './externalLibraries.js';
import { GraphCanvas } from './GraphCanvas.js';
import styles from '../../pages/visualizer.module.css';

export function ReactFlowVisualization({ graphData }) {
  const [reactFlowReady, setReactFlowReady] = useState(false);
  
  // Track what's causing parent re-renders for debugging
  const renderCount = useRef(0);
  renderCount.current += 1;

  // Memoize graphData to prevent GraphCanvas re-mounting
  const stableGraphData = useMemo(() => {
    return graphData;
  }, [graphData]);

  // Load external libraries when component mounts
  useEffect(() => {
    if (reactFlowReady) {
      return;
    }
    
    loadExternalLibraries().then(() => {
      setReactFlowReady(true);
    }).catch((error) => {
      console.error('Failed to load external libraries:', error);
    });
  }, []); // Empty dependency array to run only once

  if (!reactFlowReady) {
    return <div className={styles.loading}>Loading ReactFlow visualization...</div>;
  }

  // ReactFlow v12 is imported directly via npm, no loading needed
  return <GraphCanvas graphData={stableGraphData} />;
}
