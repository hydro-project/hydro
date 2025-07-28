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

  // Warn about any group node with missing style before passing to ReactFlowInner
  useEffect(() => {
    if (Array.isArray(nodes) && nodes.length > 0) {
      nodes.forEach(n => {
        if (n.type === 'group' && (!n.style || typeof n.style.width === 'undefined' || typeof n.style.height === 'undefined' || !n.id)) {
          console.warn('[Visualizer] Invalid group node before ReactFlowInner:', n);
          console.trace('[Visualizer] Stack trace for invalid group node');
        }
      });
    }
  }, [nodes]);

  // Simple change handlers that pass through to ReactFlowInner
  const onNodesChange = useCallback((changes) => {
    setNodes((nds) => applyNodeChanges(changes, nds));
  }, []);

  const onEdgesChange = useCallback((changes) => {
    setEdges((eds) => applyEdgeChanges(changes, eds));
  }, []);

  // Process graph data and apply layout
  useEffect(() => {
    console.log('[DEBUG] useEffect triggered for graphData change');
    const processGraph = async () => {
      setIsLoading(true);

      try {
        const result = await processGraphData(graphData, colorPalette, currentLayout, applyLayout);
        // Debug: print style for all group nodes after processGraphData returns
        result.nodes.filter(n => n.type === 'group').forEach(n => {
          console.log(`[POST-PROCESSGRAPH-GROUPNODE-STYLE] id=${n.id} style=`, n.style);
        });
        setNodes(result.nodes);
        // Debug: print style for all group nodes at state update
        result.nodes.filter(n => n.type === 'group').forEach(n => {
          console.log(`[SET-NODES-GROUPNODE-STYLE] id=${n.id} style=`, n.style);
        });
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

  // Debug: print all nodes before rendering
  nodes.forEach(n => {
    console.log(`[PRE-RENDER-NODE] id=${n.id} type=${n.type} label=${n.data?.label} style=`, n.style);
  });
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
