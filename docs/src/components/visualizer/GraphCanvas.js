/**
 * Graph Canvas Component
 * 
 * Advanced graph visualizer with layout controls and state management
 */

import React, { useState, useEffect, useCallback, useMemo } from 'react';
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

export function GraphCanvas({ graphData, maxVisibleNodes = 50 }) {
  const [nodes, setNodes] = useState([]);
  const [edges, setEdges] = useState([]);
  const [currentLayout, setCurrentLayout] = useState('mrtree');
  const [colorPalette, setColorPalette] = useState('Set3');

  // Simple change handlers
  const onNodesChange = useCallback((changes) => {
    setNodes((nds) => {
      // Filter out automatic dimensions during layout
      const validChanges = changes.filter(change => 
        change.type !== 'dimensions' || change.resizing
      );
      return validChanges.length > 0 ? applyNodeChanges(validChanges, nds) : nds;
    });
  }, []);

  const onEdgesChange = useCallback((changes) => {
    setEdges((eds) => applyEdgeChanges(changes, eds));
  }, []);
  
  // Keep locationData for internal tracking but remove from visualization components
  const locationData = useMemo(() => {
    const locations = new Map();
    if (graphData?.locations) {
      graphData.locations.forEach(location => {
        if (location && typeof location.id !== 'undefined') {
          locations.set(parseInt(location.id, 10), location);
        }
      });
    }
    
    (graphData?.nodes || []).forEach(node => {
      if (node.data?.locationId !== undefined && node.data?.location && !locations.has(node.data.locationId)) {
        locations.set(node.data.locationId, { id: node.data.locationId, label: node.data.location });
      }
    });
    
    return locations;
  }, [graphData]);

  // Process graph data when data changes
  useEffect(() => {
    if (!graphData) {
      return;
    }

    const processData = async () => {
      try {
        const result = await processGraphData(graphData, colorPalette, currentLayout, applyLayout);
        setNodes(result.nodes);
        setEdges(result.edges);
      } catch (error) {
        console.error('🚨 LAYOUT ERROR:', error);
        // Fallback to original data
        setNodes(graphData.nodes || []);
        setEdges(graphData.edges || []);
      }
    };

    processData();
  }, [graphData, currentLayout, colorPalette]);

  const handleLayoutChange = useCallback((newLayout) => {
    setCurrentLayout(newLayout);
  }, []);

  const handlePaletteChange = useCallback((newPalette) => {
    setColorPalette(newPalette);
  }, []);

  if (!nodes.length && graphData?.nodes?.length) {
    return <div className={styles.loading}>Preparing visualization...</div>;
  }

  return (
    <div className={styles.visualizationWrapper}>
      <LayoutControls 
        currentLayout={currentLayout}
        onLayoutChange={handleLayoutChange}
        colorPalette={colorPalette}
        onPaletteChange={handlePaletteChange}
      />

      <Legend 
        colorPalette={colorPalette}
      />
      
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
