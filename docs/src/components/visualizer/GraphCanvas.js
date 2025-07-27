/**
 * Graph Canvas Component
 * 
 * Main component that manages graph state, layout, and rendering
 */

import React, { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { ELK, ReactFlowComponents } from './externalLibraries.js';
import { generateNodeColors } from './colorUtils.js';
import { applyHierarchicalLayout } from './layoutAlgorithms.js';
import { generateHyperedges, routeEdgesForCollapsedContainers, createChildNodeMapping } from './hyperedgeUtils.js';
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
  
  // Calculate initial collapsed state based on container sizes
  const calculateInitialCollapsedState = useCallback((graphData) => {
    if (!graphData?.nodes) return {};
    
    const NODE_THRESHOLD = maxVisibleNodes;
    
    // Count nodes per location/container
    const containerSizes = new Map();
    graphData.nodes.forEach(node => {
      if (node.data?.locationId !== undefined) {
        const locationId = node.data.locationId;
        const containerId = `container_${locationId}`;
        containerSizes.set(containerId, (containerSizes.get(containerId) || 0) + 1);
      }
    });
    
    // If no containers or very few nodes, expand everything
    if (containerSizes.size === 0 || graphData.nodes.length <= NODE_THRESHOLD) {
      const result = {};
      containerSizes.forEach((_, containerId) => {
        result[containerId] = false; // expanded
      });
      return result;
    }
    
    // Sort containers by size (largest first)
    const sortedContainers = Array.from(containerSizes.entries())
      .sort(([,sizeA], [,sizeB]) => sizeB - sizeA);
    
    // Expand containers in order of size until we hit the threshold
    const collapsedState = {};
    let visibleNodes = 0;
    
    for (const [containerId, size] of sortedContainers) {
      if (visibleNodes + size <= NODE_THRESHOLD) {
        // Keep this container expanded
        collapsedState[containerId] = false;
        visibleNodes += size;
      } else {
        // Collapse this container
        collapsedState[containerId] = true;
      }
    }
    
    return collapsedState;
  }, [maxVisibleNodes]);
  
  const [collapsedContainers, setCollapsedContainers] = useState(() => 
    calculateInitialCollapsedState(graphData)
  );
  const [hyperedges, setHyperedges] = useState([]);
  
  // Reset collapsed state when graph data changes
  useEffect(() => {
    setCollapsedContainers(calculateInitialCollapsedState(graphData));
  }, [graphData, calculateInitialCollapsedState]);
  
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

      // Precompute hyperedges once from the initial graph structure
      const computedHyperedges = generateHyperedges(processedNodes, processedEdges);
      setHyperedges(computedHyperedges);

      // Apply ELK layout with hierarchical grouping (use empty collapsed containers for initial layout)
      const layoutResult = await applyHierarchicalLayout(processedNodes, processedEdges, currentLayout, locationData, colorPalette, {}, stableHandleContainerToggle, isDraggedRef, computedHyperedges);
      
      // For initial layout, we don't need edge routing since no containers are collapsed yet
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

        // Re-apply layout with new collapsed state, using precomputed hyperedges
        const layoutResult = await applyHierarchicalLayout(processedNodes, processedEdges, currentLayout, locationData, colorPalette, collapsedContainers, stableHandleContainerToggle, isDraggedRef, hyperedges);
        
        // Apply edge routing for collapsed containers
        const childNodeMapping = createChildNodeMapping(layoutResult.nodes);
        
        // Convert collapsedContainers format from container_${id} -> boolean to locationId -> boolean
        // Also create a simpler mapping: any container that's collapsed should be checked
        const collapsedLocationMapping = {};
        const collapsedContainerIds = new Set();
        
        Object.entries(collapsedContainers).forEach(([containerId, isCollapsed]) => {
          if (containerId.startsWith('container_') && isCollapsed) {
            const locationId = containerId.replace('container_', '');
            collapsedLocationMapping[locationId] = isCollapsed;
            collapsedContainerIds.add(containerId);
          }
        });
        
        const routedEdges = routeEdgesForCollapsedContainers(layoutResult.edges, collapsedLocationMapping, childNodeMapping, collapsedContainerIds);
        
        setNodes(layoutResult.nodes);
        setEdges(routedEdges);
      };
      
      processCollapsedContainersUpdate().catch(error => {
        console.error('🚨 COLLAPSED EFFECT ERROR:', error);
      });
    }
  }, [collapsedContainers, graphData, currentLayout, colorPalette, locationData, stableHandleContainerToggle, hyperedges]);

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
