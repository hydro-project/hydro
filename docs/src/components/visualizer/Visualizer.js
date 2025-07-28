/**
 * Clean Hydro Graph Visualizer
 * 
 * A simplified, flat graph visualizer using ReactFlow v12 and ELK layout.
 */

import React, { useState, useEffect, useCallback } from 'react';
import { applyNodeChanges, applyEdgeChanges } from '@xyflow/react';
import { applyLayout } from './layout.js';
import { LayoutControls } from './LayoutControls.js';
import { Legend } from './Legend.js';
import { ReactFlowInner } from './ReactFlowInner.js';
import { processGraphData } from './reactFlowConfig.js';
import styles from '../../pages/visualizer.module.css';

export function Visualizer({ graphData }) {
  const [nodes, setNodes] = useState([]);
  const [edges, setEdges] = useState([]);
  const [currentLayout, setCurrentLayout] = useState('mrtree');
  const [colorPalette, setColorPalette] = useState('Set3');
  const [isLoading, setIsLoading] = useState(false);

  // Simple change handlers that pass through to ReactFlowInner
  const onNodesChange = useCallback((changes) => {
    setNodes((nds) => applyNodeChanges(changes, nds));
  }, []);

  const onEdgesChange = useCallback((changes) => {
    setEdges((eds) => applyEdgeChanges(changes, eds));
  }, []);

  // Process graph data and apply layout
  useEffect(() => {
    const processGraph = async () => {
      setIsLoading(true);

      try {
        const result = await processGraphData(graphData, colorPalette, currentLayout, applyLayout);
        setNodes(result.nodes);
        setEdges(result.edges);
      } catch (error) {
        console.error('Failed to process graph:', error);
        // Fallback to original data
        setNodes(graphData?.nodes || []);
        setEdges(graphData?.edges || []);
      } finally {
        setIsLoading(false);
      }
    };

    processGraph();
  }, [graphData, currentLayout, colorPalette]);

  if (isLoading) {
    return <div className={styles.loading}>Laying out graph...</div>;
  }

  return (
    <div className={styles.visualizationWrapper}>
      <LayoutControls 
        currentLayout={currentLayout}
        onLayoutChange={setCurrentLayout}
        colorPalette={colorPalette}
        onPaletteChange={setColorPalette}
      />
      
      <Legend colorPalette={colorPalette} />
      
      <ReactFlowInner
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        colorPalette={colorPalette}
      />
    </div>
  );
}
