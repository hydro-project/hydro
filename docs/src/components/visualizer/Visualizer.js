/**
 * Clean Hydro Graph Visualizer
 * 
 * A simplified, flat graph visualizer using ReactFlow v12 and ELK layout.
 * Removed all container/grouping functionality for clarity.
 */

import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { 
  ReactFlow, 
  Controls, 
  MiniMap, 
  Background, 
  useNodesState,
  useEdgesState,
  addEdge
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { applyLayout } from './layout.js';
import { generateNodeColors } from './utils.js';
import { FileDropZone } from './FileDropZone.js';
import { LayoutControls } from './LayoutControls.js';
import { Legend } from './Legend.js';
import styles from '../../pages/visualizer.module.css';

export function Visualizer({ graphData }) {
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [currentLayout, setCurrentLayout] = useState('mrtree');
  const [colorPalette, setColorPalette] = useState('Set3');
  const [isLoading, setIsLoading] = useState(false);

  const onConnect = useCallback((connection) => {
    setEdges((eds) => addEdge(connection, eds));
  }, [setEdges]);

  // Process graph data and apply layout
  useEffect(() => {
    if (!graphData?.nodes?.length) {
      setNodes([]);
      setEdges([]);
      return;
    }

    const processGraph = async () => {
      setIsLoading(true);

      try {
        // Convert nodes with styling
        const processedNodes = graphData.nodes.map(node => {
          const nodeColors = generateNodeColors(node.data?.nodeType || 'Transform', colorPalette);
          
          return {
            ...node,
            position: { x: 0, y: 0 }, // Will be set by layout
            style: {
              background: nodeColors.gradient,
              border: `2px solid ${nodeColors.border}`,
              borderRadius: '8px',
              padding: '10px',
              color: '#333',
              fontSize: '12px',
              fontWeight: '500',
              width: 200,
              height: 60,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              textAlign: 'center',
            },
          };
        });

        // Convert edges with styling
        const processedEdges = graphData.edges.map(edge => ({
          ...edge,
          type: 'smoothstep',
          style: { strokeWidth: 2, stroke: '#666666' },
          markerEnd: { type: 'arrowclosed', width: 20, height: 20, color: '#666666' },
        }));

        // Apply layout
        const layoutResult = await applyLayout(processedNodes, processedEdges, currentLayout);
        
        setNodes(layoutResult.nodes);
        setEdges(layoutResult.edges);
      } catch (error) {
        console.error('Failed to process graph:', error);
        // Fallback to original data
        setNodes(graphData.nodes || []);
        setEdges(graphData.edges || []);
      } finally {
        setIsLoading(false);
      }
    };

    processGraph();
  }, [graphData, currentLayout, colorPalette]);

  const miniMapNodeColor = useCallback((node) => {
    const nodeColors = generateNodeColors(node.data?.nodeType || 'Transform', colorPalette);
    return nodeColors.primary;
  }, [colorPalette]);

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
      
      <div className={styles.reactflowWrapper}>
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
          fitView
          nodesDraggable={true}
          nodesConnectable={true}
          elementsSelectable={true}
          maxZoom={2}
          minZoom={0.1}
        >
          <Controls />
          <MiniMap 
            nodeColor={miniMapNodeColor}
            nodeStrokeWidth={2}
            nodeStrokeColor="#666"
            maskColor="rgba(240, 240, 240, 0.6)"
          />
          <Background color="#f5f5f5" gap={20} />
        </ReactFlow>
      </div>
    </div>
  );
}
