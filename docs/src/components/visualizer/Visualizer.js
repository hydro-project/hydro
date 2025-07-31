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
import { applyLayout, applyLayoutForCollapsedContainers } from './utils/layout.js';
import { LayoutControls } from './components/LayoutControls.js';
import { InfoPanel } from './components/InfoPanel.js';
import { ReactFlowInner } from './components/ReactFlowInner.js';
import { processGraphData } from './utils/reactFlowConfig.js';
import { useCollapsedContainers } from './containers/useCollapsedContainers.js';
import { processCollapsedContainers, rerouteEdgesForCollapsedContainers } from './containers/containerLogic.js';
import { isValidGraphData, getUniqueNodesById, COMPONENT_COLORS } from './utils/constants.js';
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
  
  // Collapsed containers state
  const {
    collapsedContainers,
    toggleContainer,
    isCollapsed,
    childNodesByParent,
    collapseAll,
    expandAll,
    hasCollapsedContainers,
    lastChangedContainer,
  } = useCollapsedContainers(nodes);

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
            padding: 0.1,
            duration: duration,
            minZoom: 0.1,
            maxZoom: 1.5,
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
  // Robust staged initialization with retries and stuck overlay
  useEffect(() => {
    if (nodes.length > 0 && !hasAutoCollapsed && childNodesByParent.size > 0 && !isLayouting && !initFailed) {
      const groupNodes = nodes.filter(node => node.type === 'group');
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

        // Staged/idle initialization
        const doInit = () => {
          performLayoutOperation(async (opId) => {
            await new Promise(resolve => setTimeout(resolve, 100));
            const allCollapsedArray = groupNodes.map(node => node.id);
            collapseAll();
            const currentDisplayNodes = processCollapsedContainers(nodes, allCollapsedArray);
            const result = await applyLayoutForCollapsedContainers(currentDisplayNodes, edges, currentLayout);
            const updatedNodes = nodes.map(baseNode => {
              const displayNode = result.nodes.find(dn => dn.id === baseNode.id);
              if (displayNode && (displayNode.type === 'group' || displayNode.type === 'collapsedContainer')) {
                return {
                  ...baseNode,
                  position: displayNode.position
                };
              }
              return baseNode;
            });
            setNodes(updatedNodes);
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
  }, [nodes.length, hasAutoCollapsed, childNodesByParent.size, isLayouting, performLayoutOperation, collapseAll, nodes, edges, currentLayout, setNodes, debouncedFitViewport, initAttempts, initFailed]);

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
  }, []);

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

  // Handle layout change with collapsed container awareness
  const handleLayoutChange = useCallback(async (newLayout) => {
    setCurrentLayout(newLayout);
    
    // If we have collapsed containers, we need to re-apply layout with proper sizing
    if (hasCollapsedContainers && nodes.length > 0) {
      performLayoutOperation(async (opId) => {
        const collapsedArray = Array.from(collapsedContainers);
        const currentDisplayNodes = processCollapsedContainers(nodes, collapsedArray);
        
        const result = await applyLayoutForCollapsedContainers(currentDisplayNodes, edges, newLayout);
        
        const updatedNodes = nodes.map(baseNode => {
          const displayNode = result.nodes.find(dn => dn.id === baseNode.id);
          if (displayNode && (displayNode.type === 'group' || displayNode.type === 'collapsedContainer')) {
            return {
              ...baseNode,
              position: displayNode.position
            };
          }
          return baseNode;
        });
        
        setNodes(updatedNodes);
        
        // Deterministic viewport fitting after layout is complete
        debouncedFitViewport(300, 'layout change');
      }, 'layout-change');
    }
  }, [hasCollapsedContainers, nodes, edges, collapsedContainers, setNodes, performLayoutOperation]);

  // Wrapper for collapseAll with layout readjustment
  const handleCollapseAll = useCallback(async () => {
    return performLayoutOperation(async (opId) => {
      // Compute the collapsed state directly instead of waiting for state update
      const groupNodes = nodes.filter(node => node.type === 'group');
      const allCollapsedArray = groupNodes.map(node => node.id);
      
      // Call collapseAll() for state consistency but don't wait for it
      collapseAll();
      
      // Use the computed collapsed state immediately
      const currentDisplayNodes = processCollapsedContainers(nodes, allCollapsedArray);
      
      const result = await applyLayoutForCollapsedContainers(currentDisplayNodes, edges, currentLayout);
      
      const updatedNodes = nodes.map(baseNode => {
        const displayNode = result.nodes.find(dn => dn.id === baseNode.id);
        if (displayNode && (displayNode.type === 'group' || displayNode.type === 'collapsedContainer')) {
          return {
            ...baseNode,
            position: displayNode.position
          };
        }
        return baseNode;
      });
      
      setNodes(updatedNodes);
      
      // Deterministic viewport fitting after layout is complete
      debouncedFitViewport(300, 'collapse all');
    }, 'collapse-all');
  }, [collapseAll, nodes, edges, currentLayout, setNodes, performLayoutOperation]);

  // Wrapper for expandAll with layout readjustment
  const handleExpandAll = useCallback(async () => {
    return performLayoutOperation(async (opId) => {
      // Call expandAll() for state consistency but don't wait for it
      expandAll();
      
      // Use empty collapsed array immediately (all expanded)
      const currentDisplayNodes = processCollapsedContainers(nodes, []);
      
      const result = await applyLayoutForCollapsedContainers(currentDisplayNodes, edges, currentLayout);
      
      const updatedNodes = nodes.map(baseNode => {
        const displayNode = result.nodes.find(dn => dn.id === baseNode.id);
        if (displayNode && (displayNode.type === 'group' || displayNode.type === 'collapsedContainer')) {
          return {
            ...baseNode,
            position: displayNode.position
          };
        }
        return baseNode;
      });
      
      setNodes(updatedNodes);
      
      // Deterministic viewport fitting after layout is complete
      debouncedFitViewport(300, 'expand all');
    }, 'expand-all');
  }, [expandAll, nodes, edges, currentLayout, setNodes, performLayoutOperation]);

  // Handle node clicks for expanding/collapsing containers
  const handleNodeClick = useCallback(async (event, node) => {
    if (node.type === 'group' || node.type === 'collapsedContainer') {
      event.stopPropagation();
      
      return performLayoutOperation(async (opId) => {
        // Compute the new collapsed state directly instead of waiting for state update
        const collapsedArray = Array.from(collapsedContainers);
        
        // Toggle the specific container in our computed array
        if (collapsedArray.includes(node.id)) {
          const index = collapsedArray.indexOf(node.id);
          collapsedArray.splice(index, 1);
        } else {
          collapsedArray.push(node.id);
        }
        
        // Call toggleContainer() for state consistency but don't wait for it
        toggleContainer(node.id);
        
        // Use the computed collapsed state immediately
        const currentDisplayNodes = processCollapsedContainers(nodes, collapsedArray);
        
        const result = await applyLayoutForCollapsedContainers(currentDisplayNodes, edges, currentLayout, node.id);
        
        const updatedNodes = nodes.map(baseNode => {
          const displayNode = result.nodes.find(dn => dn.id === baseNode.id);
          if (displayNode && (displayNode.type === 'group' || displayNode.type === 'collapsedContainer')) {
            return {
              ...baseNode,
              position: displayNode.position
            };
          }
          return baseNode;
        });
        
        setNodes(updatedNodes);
        
        // Deterministic viewport fitting after layout is complete
        debouncedFitViewport(300, 'container toggle');
      }, `toggle-container-${node.id}`);
    }
  }, [toggleContainer, nodes, edges, currentLayout, setNodes, collapsedContainers, performLayoutOperation]);

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
          hierarchyChoices={hierarchyChoices}
          currentGrouping={currentGrouping}
          onGroupingChange={handleGroupingChange}
          autoFit={autoFit}
          onAutoFitToggle={handleAutoFitToggle}
          onFitView={handleFitView}
        />
      );
      onControlsReady(controls);
    }
  }, [
    currentLayout, colorPalette, hasCollapsedContainers, hierarchyChoices, 
    currentGrouping, autoFit, onControlsReady,
    handleLayoutChange, handleCollapseAll, handleExpandAll, 
    handleGroupingChange, handleAutoFitToggle, handleFitView
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
  }, [graphData, currentLayout, colorPalette, currentGrouping, setNodes, setEdges]); // Remove isLayouting dependency

  // Process collapsed containers as derived state using useMemo
  const { displayNodes, displayEdges } = useMemo(() => {
    if (nodes.length === 0) {
      return { displayNodes: [], displayEdges: [] };
    }
    
    // Skip container processing during initialization to prevent race condition
    // where edges are processed before auto-collapse completes
    if (isInitializing) {
      return { displayNodes: nodes, displayEdges: edges };
    }
    
    const collapsedArray = Array.from(collapsedContainers);
    const processedNodes = processCollapsedContainers(nodes, collapsedArray);
    
    // CRITICAL: Use the same processedNodes for edge processing to ensure consistency
    // This prevents race conditions where edges are calculated with different node visibility
    const processedEdges = rerouteEdgesForCollapsedContainers(
      edges,
      processedNodes, // Use processedNodes instead of original nodes
      childNodesByParent,
      collapsedArray
    );
    
    return {
      displayNodes: processedNodes,
      displayEdges: processedEdges
    };
  }, [nodes, edges, collapsedContainers, childNodesByParent, isInitializing]);

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
