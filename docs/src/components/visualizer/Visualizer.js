/**
 * Clean Hydro Graph Visualizer
 * 
 * A simplified, flat graph visualizer using ReactFlow v12 and ELK layout.
 */

import React, { useState, useEffect, useCallback } from 'react';
import { 
  useNodesState, 
  useEdgesState,
  applyNodeChanges, 
  applyEdgeChanges 
} from '@xyflow/react';
import { applyLayout } from './layout.js';
import { LayoutControls } from './LayoutControls.js';
import { Legend } from './Legend.js';
import { ReactFlowInner } from './ReactFlowInner.js';
import { processGraphData } from './reactFlowConfig.js';
import styles from '../../pages/visualizer.module.css';

export function Visualizer({ graphData }) {
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
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

  // Process graph data and apply layout with proper memoization
  useEffect(() => {
    console.log('[DEBUG] useEffect triggered for graphData change');
    
    let isCancelled = false;
    
    const processGraph = async () => {
      if (!graphData || !graphData.nodes || graphData.nodes.length === 0) {
        if (!isCancelled) {
          setNodes([]);
          setEdges([]);
        }
        return;
      }
      
      setIsLoading(true);

      try {
        // Import the functions inside the effect to avoid dependency issues
        const { processGraphData } = await import('./reactFlowConfig.js');
        const { applyLayout } = await import('./layout.js');
        
        const result = await processGraphData(graphData, colorPalette, currentLayout, applyLayout);
        
        if (isCancelled) return; // Don't update state if component unmounted or effect cancelled
        
        // Ensure nodes have unique IDs to prevent ReactFlow duplication
        const uniqueNodes = result.nodes.filter((node, index, array) => 
          array.findIndex(n => n.id === node.id) === index
        );
        
        console.log(`[DEBUG] Deduplication: ${result.nodes.length} -> ${uniqueNodes.length} nodes`);
        
        setNodes(uniqueNodes);
        setEdges(result.edges);
      } catch (error) {
        if (!isCancelled) {
          console.error('Failed to process graph:', error);
          // Let the error bubble up instead of hiding it with fallbacks
          setNodes([]);
          setEdges([]);
        }
      } finally {
        if (!isCancelled) {
          setIsLoading(false);
        }
      }
    };

    processGraph();
    
    // Cleanup function to prevent state updates if effect is cancelled
    return () => {
      isCancelled = true;
    };
  }, [graphData, currentLayout, colorPalette, setNodes, setEdges]); // Add setNodes and setEdges to dependencies

  if (isLoading) {
    return <div className={styles.loading}>Laying out graph...</div>;
  }

  // Only log group nodes on first render to avoid infinite logging
  const nodeTypeCounts = {
    total: nodes.length,
    group: nodes.filter(n => n.type === 'group').length,
    groupContainer: nodes.filter(n => n.type === 'groupContainer').length,
    default: nodes.filter(n => n.type === 'default').length,
    undefined: nodes.filter(n => !n.type || n.type === 'undefined').length
  };
  
  // Only log occasionally to avoid infinite loops
  if (nodeTypeCounts.total > 0 && Math.random() < 0.1) {
    console.log(`[DEBUG] Node types:`, nodeTypeCounts);
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
