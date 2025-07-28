/**
 * Graph Canvas Component
 * 
 * Main component that manages graph state, layout, and rendering
 */

import React, { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { 
  useNodesState, 
  useEdgesState, 
  applyNodeChanges, 
  applyEdgeChanges 
} from '@xyflow/react';
import { ELK } from './externalLibraries.js';
import { generateNodeColors } from './colorUtils.js';
import { applyHierarchicalLayout } from './layoutAlgorithms.js';
import { LayoutControls } from './LayoutControls.js';
import { Legend } from './Legend.js';
import { ReactFlowInner } from './ReactFlowInner.js';
import styles from '../../pages/visualizer.module.css';

export function GraphCanvas({ graphData, maxVisibleNodes = 50 }) {
  // Track component creation vs re-render for debugging purposes
  const componentId = useRef(Math.random().toString(36).substr(2, 9));
  const renderCount = useRef(0);
  renderCount.current += 1;
  
  // Add mount/unmount tracking to verify if component is being recreated
  useEffect(() => {
    return () => {
      // Component cleanup - no logging needed
    };
  }, []);
  
  // ReactFlow v12 has excellent built-in state management - minimal intervention needed
  const [nodes, setNodes] = useState([]);
  const [edges, setEdges] = useState([]);
  
  // ReactFlow v12 change handlers with minimal filtering for ELK compatibility
  const onNodesChange = useCallback((changes) => {
    setNodes((nds) => {
      // ReactFlow v12: Only filter dimension changes during ELK layout operations
      const validChanges = changes.filter(change => {
        // Allow all changes except automatic dimensions during layout
        return change.type !== 'dimensions' || change.resizing;
      });
      
      return validChanges.length > 0 ? applyNodeChanges(validChanges, nds) : nds;
    });
  }, []);
  
  const onEdgesChange = useCallback((changes) => {
    setEdges((eds) => applyEdgeChanges(changes, eds));
  }, []);
    
  // Track nodes/edges reference changes
  const nodesRef = useRef(nodes);
  const edgesRef = useRef(edges);
  if (nodesRef.current !== nodes) {
    nodesRef.current = nodes;
  }
  if (edgesRef.current !== edges) {
    edgesRef.current = edges;
  }

  const [currentLayout, setCurrentLayout] = useState('mrtree');
  const [colorPalette, setColorPalette] = useState('Set3');
  
  // Remove locationData state - just compute it directly when needed
  // This prevents the infinite re-render cycle
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

  // Process graph data when ReactFlow is loaded and data changes
  useEffect(() => {
    if (!graphData || !ELK) {
      return;
    }

    const processData = async () => {
      // Convert nodes with enhanced styling
      let processedNodes = (graphData.nodes || []).map(node => {
        const nodeColors = generateNodeColors(node.data?.nodeType || 'Transform', colorPalette);
        
        return {
          ...node,
          position: { x: 0, y: 0 },
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

      // Convert edges with enhanced styling
      const processedEdges = (graphData.edges || []).map(edge => ({
        ...edge,
        type: 'smoothstep',
        style: { strokeWidth: 2, stroke: '#666666' },
        markerEnd: { type: 'arrowclosed', width: 20, height: 20, color: '#666666' },
      }));

      // Apply ELK layout
      const layoutResult = await applyHierarchicalLayout(processedNodes, processedEdges, currentLayout, locationData, colorPalette);
      
      setNodes(layoutResult.nodes);
      setEdges(layoutResult.edges);
    };

    processData().catch(error => {
      console.error('🚨 LAYOUT ERROR:', error);
    });
  }, [graphData, currentLayout, colorPalette, locationData]);

  const handleLayoutChange = useCallback((newLayout) => {
    setCurrentLayout(newLayout);
  }, []);

  const handlePaletteChange = useCallback((newPalette) => {
    setColorPalette(newPalette);
  }, []);

  if (!nodes) {
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
        locationData={locationData}
      />
      
      <ReactFlowInner 
        nodes={nodes} 
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        locationData={locationData}
        colorPalette={colorPalette}
      />
    </div>
  );
}
