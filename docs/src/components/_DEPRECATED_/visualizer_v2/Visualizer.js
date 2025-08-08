/**
 * Clean Hydro Graph Visualizer
 * 
 * A simplified, flat graph visualizer using ReactFlow v12 and ELK layout.
 */

import React, { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { 
  useNodesState, 
  useEdgesState
} from '@xyflow/react';
import { applyLayout, layoutVisualElements, clearContainerDimensionsCache, createVisualStateFromGraph, createVisualFilters, VisualState } from './utils/layout.js';
import { createReactFlowStateManager } from './utils/reactFlowStateManager.js';
import { LayoutControls } from './components/LayoutControls.js';
import { InfoPanel } from './components/InfoPanel.js';
import { ReactFlowInner } from './components/ReactFlowInner.js';
import { processGraphData, FIT_VIEW_CONFIG } from './utils/reactFlowConfig.js';
import { isValidGraphData, getUniqueNodesById, COMPONENT_COLORS, filterNodesByType } from './utils/constants.js';
import styles from '../../pages/visualizer.module.css';

// Initialization retry constants
const MAX_INIT_ATTEMPTS = 4;
const INIT_RETRY_DELAY = 800; // ms between retries
const INIT_TIMEOUT = 3000; // ms to consider an attempt stuck

export function Visualizer({ graphData, onControlsReady }) {
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [currentLayout, setCurrentLayout] = useState('mrtree');
  const [colorPalette, setColorPalette] = useState('Set3');
  const [isLoading, setIsLoading] = useState(false);
  const [hasAutoCollapsed, setHasAutoCollapsed] = useState(false);
  const [isLayouting, setIsLayouting] = useState(false);
  const [layoutOperationId, setLayoutOperationId] = useState(0);
  const [autoFit, setAutoFit] = useState(true); // Default to auto-fit enabled
  const [lastFitTimestamp, setLastFitTimestamp] = useState(0); // Track when fit was last applied
  const [isInitializing, setIsInitializing] = useState(false); // Track initialization overlay
  const [initAttempts, setInitAttempts] = useState(0);
  const [initFailed, setInitFailed] = useState(false);
  const initTimeoutRef = useRef(null);
  
  // Panel positions state
  const [panelPositions, setPanelPositions] = useState({
    info: { position: 'top-right', docked: true }
  });
  
  // Grouping hierarchy state
  const [hierarchyChoices, setHierarchyChoices] = useState([]);
  const [currentGrouping, setCurrentGrouping] = useState('');
  
  // Central Visual State Management
  const [visualState, setVisualState] = useState(null);
  
  // ReactFlow State Manager - wraps all ReactFlow interactions with VisualState as source of truth
  const reactFlowStateManager = useMemo(() => {
    return createReactFlowStateManager(setNodes, setEdges);
  }, [setNodes, setEdges]);
  
  // Get all child node IDs for each parent container (computed from nodes)
  const childNodesByParent = useMemo(() => {
    const map = new Map();
    nodes.forEach(node => {
      if (node.parentId) {
        if (!map.has(node.parentId)) {
          map.set(node.parentId, new Set());
        }
        map.get(node.parentId).add(node.id);
      }
    });
    return map;
  }, [nodes]);
  
  // Initialize visual state when nodes/edges change
  useEffect(() => {
    if (nodes.length > 0) {
      setVisualState(createVisualStateFromGraph(nodes, edges));
    }
  }, [nodes.length > 0 ? nodes.map(n => n.id).join(',') : '', edges.length]);

  // Container state management functions
  const toggleContainer = useCallback((containerId) => {
    if (!visualState) return;
    
    const newState = new VisualState();
    // Copy all existing state
    visualState.containers.forEach((state, id) => {
      newState.setContainerState(id, state);
    });
    visualState.nodes.forEach((state, id) => {
      newState.setNodeState(id, state);
    });
    visualState.edges.forEach((state, id) => {
      newState.setEdgeState(id, state);
    });
    
    // Toggle the specific container
    const currentState = visualState.getContainerState(containerId);
    const newContainerState = currentState === 'collapsed' ? 'expanded' : 'collapsed';
    newState.setContainerState(containerId, newContainerState);
    
    setVisualState(newState);
  }, [visualState]);

  const collapseAll = useCallback(() => {
    if (!visualState) return;
    
    const newState = new VisualState();
    // Copy node and edge state
    visualState.nodes.forEach((state, id) => {
      newState.setNodeState(id, state);
    });
    visualState.edges.forEach((state, id) => {
      newState.setEdgeState(id, state);
    });
    
    // Set all containers to collapsed
    const groupNodes = filterNodesByType(nodes, 'group');
    groupNodes.forEach(container => {
      newState.setContainerState(container.id, 'collapsed');
    });
    
    setVisualState(newState);
  }, [visualState, nodes]);

  const expandAll = useCallback(() => {
    if (!visualState) return;
    
    const newState = new VisualState();
    // Copy node and edge state
    visualState.nodes.forEach((state, id) => {
      newState.setNodeState(id, state);
    });
    visualState.edges.forEach((state, id) => {
      newState.setEdgeState(id, state);
    });
    
    // Set all containers to expanded
    const groupNodes = filterNodesByType(nodes, 'group');
    groupNodes.forEach(container => {
      newState.setContainerState(container.id, 'expanded');
    });
    
    setVisualState(newState);
  }, [visualState, nodes]);

  const isCollapsed = useCallback((containerId) => {
    return visualState?.getContainerState(containerId) === 'collapsed';
  }, [visualState]);

  const hasCollapsedContainers = useMemo(() => {
    if (!visualState) return false;
    for (const [_, state] of visualState.containers) {
      if (state === 'collapsed') return true;
    }
    return false;
  }, [visualState]);

  // Get collapsed containers as array for backward compatibility
  const collapsedContainers = useMemo(() => {
    if (!visualState) return new Set();
    const collapsed = new Set();
    visualState.containers.forEach((state, id) => {
      if (state === 'collapsed') {
        collapsed.add(id);
      }
    });
    return collapsed;
  }, [visualState]);

  // Helper function to handle viewport fitting after layout operations
  const fitViewport = useCallback((duration = 300, operationName = 'layout', forceAutoFit = false) => {
    // Skip auto-fitting if autoFit is disabled and this isn't a forced fit
    if (!autoFit && !forceAutoFit) {
      return;
    }

    // Track that we're applying fit
    const timestamp = Date.now();
    setLastFitTimestamp(timestamp);

    // Use a more robust approach to prevent ResizeObserver loops
    const scheduleFitView = () => {
      // Clear any existing timeout first
      if (window.fitViewTimeout) {
        clearTimeout(window.fitViewTimeout);
      }

      // Schedule the fitView call for the next frame cycle
      window.fitViewTimeout = setTimeout(() => {
        window.dispatchEvent(new CustomEvent('fitViewRequest', {
          detail: {
            padding: FIT_VIEW_CONFIG.padding,
            duration: duration,
            minZoom: FIT_VIEW_CONFIG.minZoom,
            maxZoom: FIT_VIEW_CONFIG.maxZoom,
            operationName,
            timestamp
          }
        }));
      }, 200); // Increased delay to ensure DOM stability
    };

    // Use requestIdleCallback if available, otherwise fallback to setTimeout
    if (window.requestIdleCallback) {
      window.requestIdleCallback(scheduleFitView, { timeout: 500 });
    } else {
      scheduleFitView();
    }

    // Cleanup function
    return () => {
      if (window.fitViewTimeout) {
        clearTimeout(window.fitViewTimeout);
        window.fitViewTimeout = null;
      }
    };
  }, [autoFit]);

  // Debounced fitViewport to prevent ResizeObserver loops
  const debouncedFitViewport = useCallback((duration = 300, operationName = 'layout', forceAutoFit = false) => {
    // Clear any existing timeout
    if (window.fitViewportTimeout) {
      clearTimeout(window.fitViewportTimeout);
    }
    
    // Use longer debounce during initialization or auto-collapse operations
    const isInitialization = operationName === 'auto-collapse' || operationName === 'layout';
    const debounceDelay = isInitialization ? 500 : 150; // Longer delay for initialization
    
    // Set new timeout
    window.fitViewportTimeout = setTimeout(() => {
      fitViewport(duration, operationName, forceAutoFit);
    }, debounceDelay);
  }, [fitViewport]);

  // Safe layout operation wrapper to prevent race conditions
  const performLayoutOperation = useCallback(async (operationFn, operationName) => {
    if (isLayouting) {
      return false;
    }

    const currentOpId = layoutOperationId + 1;
    setLayoutOperationId(currentOpId);
    setIsLayouting(true);

    try {
      await operationFn(currentOpId);
      return true;
    } catch (error) {
      console.error(`[Visualizer] Failed ${operationName}:`, error);
      
      // Attempt recovery by resetting layout state
      if (error.name === 'ResizeObserver' || error.message?.includes('ResizeObserver')) {
        // ResizeObserver errors are usually harmless
      }
      
      return false;
    } finally {
      setIsLayouting(false);
    }
  }, [isLayouting, layoutOperationId]);

  // Auto-collapse all containers on initial load with staged initialization
  // Using VisualState API for centralized state management
  useEffect(() => {
    if (nodes.length > 0 && !hasAutoCollapsed && childNodesByParent.size > 0 && !isLayouting && !initFailed && !isLoading && visualState) {
      const groupNodes = filterNodesByType(nodes, 'group');
      if (groupNodes.length > 0) {
        setHasAutoCollapsed(true);
        setIsInitializing(true);
        setInitAttempts((prev) => prev + 1);

        // Timeout: if initialization takes too long, trigger retry or stuck overlay
        if (initTimeoutRef.current) clearTimeout(initTimeoutRef.current);
        initTimeoutRef.current = setTimeout(() => {
          if (initAttempts + 1 >= MAX_INIT_ATTEMPTS) {
            setIsInitializing(false);
            setInitFailed(true);
          } else {
            setHasAutoCollapsed(false); // allow retry
          }
        }, INIT_TIMEOUT);

        // Staged/idle initialization using VisualState
        const doInit = async () => {
          performLayoutOperation(async (opId) => {
            await new Promise(resolve => setTimeout(resolve, 100));
            
            // STEP 1: First run full layout to populate dimension cache
            console.log('[Visualizer] 🚀 INIT: Running full layout to populate dimension cache...');
            const fullLayoutResult = await applyLayout(nodes, edges, currentLayout);
            
            // STEP 2: Create collapsed visual state
            console.log('[Visualizer] 🚀 INIT: Creating collapsed visual state...');
            const initState = createVisualStateFromGraph(fullLayoutResult.nodes, edges);
            groupNodes.forEach(container => {
              initState.setContainerState(container.id, 'collapsed');
            });
            
            // STEP 3: Apply collapsed layout using ReactFlow state manager
            console.log('[Visualizer] 🚀 INIT: Applying collapsed layout via ReactFlow state manager...');
            const collapsedResult = await reactFlowStateManager.initializeReactFlow(
              initState, 
              fullLayoutResult.nodes, 
              edges, 
              currentLayout, 
              'auto-collapse'
            );
            
            setVisualState(initState); // Update visual state
            
            await new Promise(resolve => setTimeout(resolve, 200));
            setTimeout(() => {
              debouncedFitViewport(300, 'auto-collapse');
              setTimeout(() => {
                setIsInitializing(false);
                if (initTimeoutRef.current) clearTimeout(initTimeoutRef.current);
              }, 400);
            }, 300);
          }, 'auto-collapse');
        };
        if (window.requestIdleCallback) {
          window.requestIdleCallback(doInit, { timeout: 500 });
        } else {
          setTimeout(doInit, 100);
        }
      }
    }
    // Cleanup timeout on unmount
    return () => {
      if (initTimeoutRef.current) clearTimeout(initTimeoutRef.current);
    };
  }, [nodes.length, hasAutoCollapsed, childNodesByParent.size, isLayouting, performLayoutOperation, nodes, edges, currentLayout, debouncedFitViewport, initAttempts, initFailed, visualState]);

  // Reset auto-collapse flag when graph data changes
  useEffect(() => {
    setHasAutoCollapsed(false);
    setIsInitializing(false);
    setInitAttempts(0);
    setInitFailed(false);
    if (initTimeoutRef.current) clearTimeout(initTimeoutRef.current);
  }, [graphData]);

  // Validate group nodes have proper styling
  useEffect(() => {
    if (Array.isArray(nodes) && nodes.length > 0) {
      nodes.forEach(n => {
        if (n.type === 'group' && (!n.style || typeof n.style.width === 'undefined' || typeof n.style.height === 'undefined' || !n.id)) {
          console.warn('[Visualizer] Invalid group node detected:', n.id);
        }
      });
    }
  }, [nodes]);

  // Extract hierarchy choices from graph data and set default
  useEffect(() => {
    if (isValidGraphData(graphData) && graphData.hierarchyChoices) {
      setHierarchyChoices(graphData.hierarchyChoices);
      // Set default grouping to the first available choice
      if (graphData.hierarchyChoices.length > 0 && !currentGrouping) {
        setCurrentGrouping(graphData.hierarchyChoices[0].id);
      }
    } else {
      setHierarchyChoices([]);
      setCurrentGrouping('');
    }
  }, [graphData]); // Remove currentGrouping to prevent infinite loops

  // Handle grouping change
  const handleGroupingChange = useCallback((newGrouping) => {
    setCurrentGrouping(newGrouping);
    // Reset auto-collapse flag so it will trigger again for the new grouping
    setHasAutoCollapsed(false);
    // Temporarily disable auto-collapse to prevent race condition during grouping change
    setIsLoading(true);
  }, [currentGrouping]);

  // Handle manual fit view button click
  const handleFitView = useCallback(() => {
    fitViewport(300, 'manual', true); // Force autofit even if disabled
  }, [fitViewport]);

  // Handle auto-fit toggle
  const handleAutoFitToggle = useCallback((enabled) => {
    setAutoFit(enabled);
    
    if (enabled) {
      // When auto-fit is enabled, apply fit immediately
      debouncedFitViewport(300, 'auto-fit-enabled', true);
    }
    // When auto-fit is disabled, no action needed - button is always available
  }, [debouncedFitViewport]);

  // Handle layout change with VisualState-based layout
  const handleLayoutChange = useCallback(async (newLayout) => {
    setCurrentLayout(newLayout);
    
    // If we have containers, re-apply layout with current visual state
    if (visualState && nodes.length > 0) {
      performLayoutOperation(async (opId) => {
        const result = await reactFlowStateManager.updateLayout(
          visualState, 
          nodes, 
          edges, 
          newLayout, 
          'layout-change'
        );
        
        // Deterministic viewport fitting after layout is complete
        debouncedFitViewport(300, 'layout change');
      }, 'layout-change');
    }
  }, [visualState, nodes, edges, performLayoutOperation, debouncedFitViewport]);

  // Wrapper for collapseAll with VisualState-based layout
  const handleCollapseAll = useCallback(async () => {
    if (!visualState) return;
    
    return performLayoutOperation(async (opId) => {
      // Create new visual state with all containers collapsed
      const newState = createVisualStateFromGraph(nodes, edges);
      const groupNodes = filterNodesByType(nodes, 'group');
      groupNodes.forEach(container => {
        newState.setContainerState(container.id, 'collapsed');
      });
      
      // Apply layout with new state via ReactFlow state manager
      const result = await reactFlowStateManager.updateContainerStates(
        newState, 
        nodes, 
        edges, 
        currentLayout, 
        'collapse-all'
      );
      
      setVisualState(newState); // Update state
      
      // Deterministic viewport fitting after layout is complete
      debouncedFitViewport(300, 'collapse all');
    }, 'collapse-all');
  }, [visualState, nodes, edges, currentLayout, performLayoutOperation, debouncedFitViewport, reactFlowStateManager]);

  // Wrapper for expandAll with VisualState-based layout
  const handleExpandAll = useCallback(async () => {
    if (!visualState) return;
    
    return performLayoutOperation(async (opId) => {
      // Create new visual state with all containers expanded
      const newState = createVisualStateFromGraph(nodes, edges);
      const groupNodes = filterNodesByType(nodes, 'group');
      groupNodes.forEach(container => {
        newState.setContainerState(container.id, 'expanded');
      });
      
      // Apply layout with new state via ReactFlow state manager
      const result = await reactFlowStateManager.updateContainerStates(
        newState, 
        nodes, 
        edges, 
        currentLayout, 
        'expand-all'
      );
      
      setVisualState(newState); // Update state
      
      // Deterministic viewport fitting after layout is complete
      debouncedFitViewport(300, 'expand all');
    }, 'expand-all');
  }, [visualState, nodes, edges, currentLayout, performLayoutOperation, debouncedFitViewport, reactFlowStateManager]);

  // Handle node clicks for expanding/collapsing containers
  const handleNodeClick = useCallback(async (event, node) => {
    if (node.type === 'group' || node.type === 'collapsedContainer') {
      event.stopPropagation();
      
      if (!visualState) return;
      
      return performLayoutOperation(async (opId) => {
        // Create new visual state with toggled container
        const newState = new VisualState();
        // Copy all existing state
        visualState.containers.forEach((state, id) => {
          newState.setContainerState(id, state);
        });
        visualState.nodes.forEach((state, id) => {
          newState.setNodeState(id, state);
        });
        visualState.edges.forEach((state, id) => {
          newState.setEdgeState(id, state);
        });
        
        // Toggle the specific container
        const currentState = visualState.getContainerState(node.id);
        const newContainerState = currentState === 'collapsed' ? 'expanded' : 'collapsed';
        newState.setContainerState(node.id, newContainerState);
        
        console.log(`[Visualizer] 🎯 VISUAL STATE: Container ${node.id} toggled to ${newContainerState}`);
        
        // Use the ReactFlow state manager to apply the new state
        const result = await reactFlowStateManager.updateContainerStates(
          newState, 
          nodes, 
          edges, 
          currentLayout, 
          `toggle-container-${node.id}`
        );
        
        setVisualState(newState); // Update the visual state
        
        // Deterministic viewport fitting after layout is complete
        debouncedFitViewport(300, 'container toggle');
      }, `toggle-container-${node.id}`);
    }
  }, [visualState, nodes, edges, currentLayout, performLayoutOperation, debouncedFitViewport, reactFlowStateManager]);

  // Handle container toggle from hierarchy tree
  const handleHierarchyToggle = useCallback(async (containerId) => {
    // Create a mock event and node object for compatibility with handleNodeClick
    const mockEvent = { stopPropagation: () => {} };
    const mockNode = { 
      id: containerId, 
      type: collapsedContainers.has(containerId) ? 'collapsedContainer' : 'group'
    };
    
    return handleNodeClick(mockEvent, mockNode);
  }, [handleNodeClick, collapsedContainers]);

  // Handle panel position changes
  const handlePanelPositionChange = useCallback((panelId, position, docked) => {
    setPanelPositions(prev => ({
      ...prev,
      [panelId]: { position, docked }
    }));
  }, []);

  // Create toolbar controls and pass them to parent
  useEffect(() => {
    if (onControlsReady) {
      const controls = (
        <LayoutControls 
          currentLayout={currentLayout}
          onLayoutChange={handleLayoutChange}
          colorPalette={colorPalette}
          onPaletteChange={setColorPalette}
          hasCollapsedContainers={hasCollapsedContainers}
          onCollapseAll={handleCollapseAll}
          onExpandAll={handleExpandAll}
          autoFit={autoFit}
          onAutoFitToggle={handleAutoFitToggle}
          onFitView={handleFitView}
        />
      );
      onControlsReady(controls);
    }
  }, [
    currentLayout, colorPalette, hasCollapsedContainers, autoFit, onControlsReady,
    handleLayoutChange, handleCollapseAll, handleExpandAll, 
    handleAutoFitToggle, handleFitView
  ]);

  // Process graph data and apply layout with proper memoization
  useEffect(() => {
    let isCancelled = false;
    
    const processGraph = async () => {
      if (!isValidGraphData(graphData)) {
        if (!isCancelled) {
          setNodes([]);
          setEdges([]);
        }
        return;
      }
      
      setIsLoading(true);

      try {
        // Pass the current grouping to the graph processing
        const result = await processGraphData(graphData, colorPalette, currentLayout, applyLayout, currentGrouping);
        
        if (isCancelled) return; // Don't update state if component unmounted or effect cancelled
        
        // Ensure nodes have unique IDs to prevent ReactFlow duplication
        const uniqueNodes = getUniqueNodesById(result.nodes);
        
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
  }, [graphData, currentLayout, colorPalette, currentGrouping]); // Remove setNodes, setEdges dependencies

  // Process visual state as derived state using useMemo
  const { displayNodes, displayEdges } = useMemo(() => {
    if (nodes.length === 0 || !visualState) {
      return { displayNodes: [], displayEdges: [] };
    }
    
    // Skip container processing during initialization to prevent race condition
    if (isInitializing) {
      return { displayNodes: nodes, displayEdges: edges };
    }
    
    // The layout function now handles all visual state filtering and edge rerouting
    // We just use the nodes and edges exactly as returned by the layout
    return {
      displayNodes: nodes,
      displayEdges: edges
    };
  }, [nodes, edges, visualState, isInitializing]);

  // Note: Removed ReactFlow updateNode/updateNodeInternals calls as they are not necessary
  // and were causing errors. ReactFlow v12 handles node updates automatically.

  if (isLoading) {
    return <div className={styles.loading}>Laying out graph...</div>;
  }

  return (
    <div className={styles.visualizationWrapper}>
      <InfoPanel 
        colorPalette={colorPalette} 
        graphData={graphData}
        nodes={nodes}
        collapsedContainers={collapsedContainers}
        onToggleContainer={handleHierarchyToggle}
        childNodesByParent={childNodesByParent}
        onPositionChange={(position, docked) => handlePanelPositionChange('info', position, docked)}
        hierarchyChoices={hierarchyChoices}
        currentGrouping={currentGrouping}
        onGroupingChange={handleGroupingChange}
      />

      <ReactFlowInner
        nodes={displayNodes}
        edges={displayEdges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeClick={handleNodeClick}
        colorPalette={colorPalette}
      />

      {/* Initialization overlay to hide viewport during setup */}
      {isInitializing && (
        <div className={styles.initializationOverlay}>
          <div className={styles.initializationContent}>
            <div className={styles.spinner}></div>
            <span>Preparing graph...</span>
          </div>
        </div>
      )}
      {/* Stuck overlay if all attempts fail */}
      {initFailed && (
        <div className={styles.initializationOverlay}>
          <div className={styles.initializationContent}>
            <div className={styles.spinner}></div>
            <span style={{color: COMPONENT_COLORS.STATUS_ERROR}}>Graph failed to initialize after several attempts.</span>
            <button
              className={styles.clearButton}
              style={{marginTop: 16}}
              onClick={() => {
                setInitFailed(false);
                setInitAttempts(0);
                setHasAutoCollapsed(false);
                setIsInitializing(false);
              }}
            >Retry Initialization</button>
          </div>
        </div>
      )}
    </div>
  );
}
