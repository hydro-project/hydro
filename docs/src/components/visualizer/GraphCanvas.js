/**
 * Graph Canvas Component
 * 
 * Main component that manages graph state, layout, and rendering
 */

import React, { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { ELK, ReactFlowComponents } from './externalLibraries.js';
import { generateNodeColors } from './colorUtils.js';
import { applyHierarchicalLayout } from './layoutAlgorithms.js';
import { LayoutControls } from './LayoutControls.js';
import { Legend } from './Legend.js';
import { ReactFlowInner } from './ReactFlowInner.js';
import styles from '../../pages/visualizer.module.css';

export function GraphCanvas({ graphData }) {
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
  
  // FIXED: Replace broken CDN ReactFlow hooks with standard React state
  const [nodes, setNodes] = useState([]);
  const [edges, setEdges] = useState([]);
  
  // Add logging to track when nodes change
  useEffect(() => {  }, [nodes]);
  
  // Track which containers are being dragged by ReactFlow
  const isDraggedRef = useRef({});
  
  // Create stable change handlers using useCallback
  const onNodesChange = useCallback((changes) => {
    // Track container drag states based on ReactFlow position changes
    changes.forEach(change => {
      if (change.type === 'position' && change.id.startsWith('container_') && change.dragging) {
        isDraggedRef.current[change.id] = true;
      }
    });
    
    setNodes((nds) => {
      // Filter out automatic dimension changes that cause infinite loops
      // Only allow user-initiated changes like position and select
      const meaningfulChanges = changes.filter(change => {
        // Exclude 'dimensions' type changes as these are automatic ReactFlow measurements
        // Only allow position (drag) and select (click) changes
        return ['position', 'select'].includes(change.type);
      });
      
      if (meaningfulChanges.length === 0) {
        return nds; // Return current nodes unchanged
      }
      
      const updatedNodes = ReactFlowComponents.applyNodeChanges(meaningfulChanges, nds);
      
      if (updatedNodes.length === 0 && nds.length > 0) {
        console.error(`🚨 APPLY_NODE_CHANGES RETURNED EMPTY! Input had ${nds.length} nodes, changes:`, meaningfulChanges);
        // Return original nodes to prevent empty state
        return nds;
      }
      
      return updatedNodes;
    });
  }, []);
  
  const onEdgesChange = useCallback((changes) => {
    setEdges((eds) => ReactFlowComponents.applyEdgeChanges(changes, eds));
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
  const [collapsedContainers, setCollapsedContainers] = useState({});
  
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

  // Use useRef to create a stable callback reference
  const handleContainerToggleRef = useRef();
  handleContainerToggleRef.current = (containerId) => {
    setCollapsedContainers(prev => {
      const newState = {
        ...prev,
        [containerId]: !prev[containerId]
      };
      return newState;
    });
  };
  
  // Create a stable callback that never changes
  const stableHandleContainerToggle = useCallback((containerId) => {
    if (handleContainerToggleRef.current) {
      handleContainerToggleRef.current(containerId);
    }
  }, []);

  // Add counters to track useEffect execution
  const mainEffectCount = useRef(0);
  const collapsedEffectCount = useRef(0);

  // Process graph data when ReactFlow is loaded and data changes
  useEffect(() => {
    mainEffectCount.current += 1;
    
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
        type: 'bezier',
        style: { strokeWidth: 2, stroke: '#666666' },
        markerEnd: { type: 'arrowclosed', width: 20, height: 20, color: '#666666' },
      }));

      // Apply ELK layout with hierarchical grouping (use empty collapsed containers for initial layout)
      const layoutResult = await applyHierarchicalLayout(processedNodes, processedEdges, currentLayout, locationData, colorPalette, {}, stableHandleContainerToggle, isDraggedRef);
      
      setNodes(layoutResult.nodes);
      setEdges(layoutResult.edges);
    };

    processData().catch(error => {
      console.error('🚨 MAIN EFFECT ERROR:', error);
    });
  }, [graphData, currentLayout, colorPalette, locationData, stableHandleContainerToggle]);

  // Separate useEffect to handle collapsed container changes without triggering full re-layout
  useEffect(() => {
    collapsedEffectCount.current += 1;
    
    if (Object.keys(collapsedContainers).length === 0) {
      return;
    }
    
    // Only re-run layout if we have data and some containers are actually collapsed
    if (graphData && ELK) {
      const processCollapsedContainersUpdate = async () => {
        // Convert nodes again
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

        // Convert edges again
        const processedEdges = (graphData.edges || []).map(edge => ({
          ...edge,
          type: 'bezier',
          style: { strokeWidth: 2, stroke: '#666666' },
          markerEnd: { type: 'arrowclosed', width: 20, height: 20, color: '#666666' },
        }));

        // Re-apply layout with new collapsed state
        const layoutResult = await applyHierarchicalLayout(processedNodes, processedEdges, currentLayout, locationData, colorPalette, collapsedContainers, stableHandleContainerToggle, isDraggedRef);
        
        setNodes(layoutResult.nodes);
        setEdges(layoutResult.edges);
      };
      
      processCollapsedContainersUpdate().catch(error => {
        console.error('🚨 COLLAPSED EFFECT ERROR:', error);
      });
    }
  }, [collapsedContainers, graphData, currentLayout, colorPalette, locationData, stableHandleContainerToggle]);

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
        onContainerToggle={stableHandleContainerToggle}
      />
    </div>
  );
}
