/**
 * Clean Hydro Graph Visualizer
 * 
 * A simplified, flat graph visualizer using ReactFlow v12 and ELK layout.
 */

import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { 
  useNodesState, 
  useEdgesState
} from '@xyflow/react';
import { applyLayout, applyLayoutForCollapsedContainers } from './utils/layout.js';
import { LayoutControls } from './components/LayoutControls.js';
import { Legend } from './components/Legend.js';
import { ReactFlowInner } from './components/ReactFlowInner.js';
import { processGraphData } from './utils/reactFlowConfig.js';
import { useCollapsedContainers } from './containers/useCollapsedContainers.js';
import { processCollapsedContainers, rerouteEdgesForCollapsedContainers } from './containers/containerLogic.js';
import { isValidGraphData, getUniqueNodesById } from './utils/constants.js';
import styles from '../../pages/visualizer.module.css';

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
  const [fitViewDisabled, setFitViewDisabled] = useState(false); // Track if Fit View button should be disabled
  const [lastFitTimestamp, setLastFitTimestamp] = useState(0); // Track when fit was last applied
  
    // Suppress ResizeObserver errors globally for this component
  useEffect(() => {
    const originalError = window.onerror;
    const originalUnhandledRejection = window.onunhandledrejection;
    const originalConsoleError = console.error;
    
    window.onerror = (message, source, lineno, colno, error) => {
      if (message && typeof message === 'string' && 
          (message.includes('ResizeObserver loop completed with undelivered notifications') ||
           message.includes('ResizeObserver'))) {
        return true; // Suppress the error
      }
      if (originalError) {
        return originalError(message, source, lineno, colno, error);
      }
      return false;
    };
    
    window.onunhandledrejection = (event) => {
      if (event.reason && event.reason.message && event.reason.message.includes('ResizeObserver')) {
        event.preventDefault(); // Suppress the error
        return;
      }
      if (originalUnhandledRejection) {
        originalUnhandledRejection(event);
      }
    };
    
    // Also suppress ResizeObserver console errors
    console.error = (...args) => {
      const message = args.join(' ');
      if (message.includes('ResizeObserver') || message.includes('loop completed with undelivered notifications')) {
        return; // Suppress ResizeObserver errors
      }
      originalConsoleError.apply(console, args);
    };
    
    return () => {
      window.onerror = originalError;
      window.onunhandledrejection = originalUnhandledRejection;
      console.error = originalConsoleError;
    };
  }, []);
  
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
    setFitViewDisabled(true); // Disable Fit View button temporarily

    // Add a delay to ensure ReactFlow is stable
    setTimeout(() => {
      try {
        if (window.reactFlowInstance) {
          // Get node bounds for viewport calculation
          const nodes = window.reactFlowInstance.getNodes();
          if (nodes.length > 0) {
            const bounds = nodes.reduce((acc, node) => {
              const x = node.position.x;
              const y = node.position.y;
              const width = node.measured?.width || node.style?.width || 100;
              const height = node.measured?.height || node.style?.height || 100;
              
              return {
                minX: Math.min(acc.minX, x),
                minY: Math.min(acc.minY, y),
                maxX: Math.max(acc.maxX, x + width),
                maxY: Math.max(acc.maxY, y + height)
              };
            }, { minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity });
            
            // Manual viewport calculation as alternative to fitView
            const graphWidth = bounds.maxX - bounds.minX;
            const graphHeight = bounds.maxY - bounds.minY;
            
            // Get the ReactFlow container dimensions
            const container = document.querySelector('.reactflowWrapper') || document.querySelector('[data-testid="rf__wrapper"]');
            if (container) {
              const containerRect = container.getBoundingClientRect();
              const containerWidth = containerRect.width;
              const containerHeight = containerRect.height;
              
              // Calculate zoom to fit with padding
              const padding = 0.05; // 5% padding
              const scaleX = (containerWidth * (1 - padding * 2)) / graphWidth;
              const scaleY = (containerHeight * (1 - padding * 2)) / graphHeight;
              const scale = Math.min(scaleX, scaleY, 2.0); // Cap at max zoom
              const finalScale = Math.max(scale, 0.2); // Ensure minimum zoom
              
              // Calculate center position
              const centerX = (bounds.minX + bounds.maxX) / 2;
              const centerY = (bounds.minY + bounds.maxY) / 2;
              
              // Calculate viewport position to center the graph
              const x = containerWidth / 2 - centerX * finalScale;
              const y = containerHeight / 2 - centerY * finalScale;
              
              // Try fitView first, but fall back to manual setViewport
              try {
                window.reactFlowInstance.fitView({ 
                  padding: 0.05,
                  duration: duration,
                  minZoom: 0.2,
                  maxZoom: 2.0
                });
              } catch (fitViewError) {
                // Fallback to manual viewport setting
                window.reactFlowInstance.setViewport({ x, y, zoom: finalScale }, { duration });
              }
            }
          }
        }
      } catch (outerError) {
        // Suppress ResizeObserver errors and other harmless layout errors
        if (outerError.name === 'ResizeObserver' || 
            outerError.message?.includes('ResizeObserver') ||
            outerError.message?.includes('loop completed with undelivered notifications')) {
          return;
        }
        console.warn(`[Visualizer] Error in fitViewport:`, outerError);
      }
    }, 500); // 500ms delay to let ReactFlow stabilize
  }, [autoFit]);

  // Helper function to mark that changes occurred (enables Fit View button)
  const markChangesOccurred = useCallback(() => {
    if (autoFit) {
      setFitViewDisabled(true); // Keep disabled when autoFit is on
    } else {
      setFitViewDisabled(false); // Enable when autoFit is off and changes occur
    }
  }, [autoFit]);

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

  // Auto-collapse all containers on initial load
  useEffect(() => {
    if (nodes.length > 0 && !hasAutoCollapsed && childNodesByParent.size > 0 && !isLayouting) {
      const groupNodes = nodes.filter(node => node.type === 'group');
      if (groupNodes.length > 0) {
        setHasAutoCollapsed(true);
        
        performLayoutOperation(async (opId) => {
          // Instead of calling collapseAll() and waiting, directly compute the collapsed state
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
          markChangesOccurred();
          
          // Deterministic viewport fitting after layout is complete
          fitViewport(300, 'auto-collapse');
        }, 'auto-collapse');
      }
    }
  }, [nodes.length, hasAutoCollapsed, childNodesByParent.size, isLayouting, performLayoutOperation, collapseAll, nodes, edges, currentLayout, setNodes, markChangesOccurred]);

  // Reset auto-collapse flag when graph data changes
  useEffect(() => {
    setHasAutoCollapsed(false);
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
  }, []);

  // Handle manual fit view button click
  const handleFitView = useCallback(() => {
    fitViewport(300, 'manual', true); // Force autofit even if disabled
  }, [fitViewport]);

  // Handle auto-fit toggle
  const handleAutoFitToggle = useCallback((enabled) => {
    setAutoFit(enabled);
    
    if (enabled) {
      // When auto-fit is enabled, apply fit immediately and keep button disabled
      setFitViewDisabled(true);
      fitViewport(300, 'auto-fit-enabled', true);
    } else {
      // When auto-fit is disabled, enable the Fit View button
      setFitViewDisabled(false);
    }
  }, [fitViewport]);

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
        markChangesOccurred();
        
        // Deterministic viewport fitting after layout is complete
        fitViewport(300, 'layout change');
      }, 'layout-change');
    }
  }, [hasCollapsedContainers, nodes, edges, collapsedContainers, setNodes, performLayoutOperation, markChangesOccurred]);

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
      markChangesOccurred();
      
      // Deterministic viewport fitting after layout is complete
      fitViewport(300, 'collapse all');
    }, 'collapse-all');
  }, [collapseAll, nodes, edges, currentLayout, setNodes, performLayoutOperation, markChangesOccurred]);

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
      markChangesOccurred();
      
      // Deterministic viewport fitting after layout is complete
      fitViewport(300, 'expand all');
    }, 'expand-all');
  }, [expandAll, nodes, edges, currentLayout, setNodes, performLayoutOperation, markChangesOccurred]);

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
        markChangesOccurred();
        
        // Deterministic viewport fitting after layout is complete
        fitViewport(300, 'container toggle');
      }, `toggle-container-${node.id}`);
    }
  }, [toggleContainer, nodes, edges, currentLayout, setNodes, collapsedContainers, performLayoutOperation, markChangesOccurred]);

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
          fitViewDisabled={fitViewDisabled}
        />
      );
      onControlsReady(controls);
    }
  }, [
    currentLayout, colorPalette, hasCollapsedContainers, hierarchyChoices, 
    currentGrouping, autoFit, fitViewDisabled, onControlsReady,
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
        markChangesOccurred();
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
  }, [graphData, currentLayout, colorPalette, currentGrouping, setNodes, setEdges, markChangesOccurred]); // Remove isLayouting dependency

  // Process collapsed containers as derived state using useMemo
  const { displayNodes, displayEdges } = useMemo(() => {
    if (nodes.length === 0) {
      return { displayNodes: [], displayEdges: [] };
    }
    
    const collapsedArray = Array.from(collapsedContainers);
    const processedNodes = processCollapsedContainers(nodes, collapsedArray);
    
    // Also process edges to reroute them for collapsed containers
    const processedEdges = rerouteEdgesForCollapsedContainers(
      edges,
      processedNodes,
      childNodesByParent,
      collapsedArray
    );
    
    return {
      displayNodes: processedNodes,
      displayEdges: processedEdges
    };
  }, [nodes, edges, collapsedContainers, childNodesByParent]);

  // Note: Removed ReactFlow updateNode/updateNodeInternals calls as they are not necessary
  // and were causing errors. ReactFlow v12 handles node updates automatically.

  if (isLoading) {
    return <div className={styles.loading}>Laying out graph...</div>;
  }

  return (
    <div className={styles.visualizationWrapper}>
      <Legend colorPalette={colorPalette} />
      
      <ReactFlowInner
        nodes={displayNodes}
        edges={displayEdges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeClick={handleNodeClick}
        colorPalette={colorPalette}
      />
    </div>
  );
}
