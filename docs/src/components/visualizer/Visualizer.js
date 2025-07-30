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
  const [lastFitTimestamp, setLastFitTimestamp] = useState(0); // Track when fit was last applied
  
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

    // Use ReactFlow's built-in fitView method without manual DOM calculations
    // Debounce to prevent cascading resize events
    const timeoutId = setTimeout(() => {
      if (window.reactFlowInstance) {
        try {
          // Use ReactFlow's fitView which handles ResizeObserver properly
          window.reactFlowInstance.fitView({ 
            padding: 0.1, // 10% padding
            duration: duration,
            minZoom: 0.1,
            maxZoom: 1.5
          });
        } catch (error) {
          console.warn(`[Visualizer] fitView failed for ${operationName}:`, error);
        }
      }
    }, 100); // Small delay to let DOM settle

    // Cleanup timeout if component unmounts
    return () => clearTimeout(timeoutId);
  }, [autoFit]);

  // Debounced fitViewport to prevent ResizeObserver loops
  const debouncedFitViewport = useCallback((duration = 300, operationName = 'layout', forceAutoFit = false) => {
    // Clear any existing timeout
    if (window.fitViewportTimeout) {
      clearTimeout(window.fitViewportTimeout);
    }
    
    // Set new timeout
    window.fitViewportTimeout = setTimeout(() => {
      fitViewport(duration, operationName, forceAutoFit);
    }, 150); // Debounce for 150ms
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
          
          // Deterministic viewport fitting after layout is complete
          debouncedFitViewport(300, 'auto-collapse');
        }, 'auto-collapse');
      }
    }
  }, [nodes.length, hasAutoCollapsed, childNodesByParent.size, isLayouting, performLayoutOperation, collapseAll, nodes, edges, currentLayout, setNodes]);

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
      <Legend colorPalette={colorPalette} graphData={graphData} />
      
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
