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

export function Visualizer({ graphData }) {
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [currentLayout, setCurrentLayout] = useState('mrtree');
  const [colorPalette, setColorPalette] = useState('Set3');
  const [isLoading, setIsLoading] = useState(false);
  const [hasAutoCollapsed, setHasAutoCollapsed] = useState(false);
  const [isLayouting, setIsLayouting] = useState(false);
  const [layoutOperationId, setLayoutOperationId] = useState(0);
  
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

  // Safe layout operation wrapper to prevent race conditions
  const performLayoutOperation = useCallback(async (operationFn, operationName) => {
    if (isLayouting) {
      console.warn(`[Visualizer] Skipping ${operationName} - layout operation already in progress`);
      return false;
    }

    const currentOpId = layoutOperationId + 1;
    setLayoutOperationId(currentOpId);
    setIsLayouting(true);
    
    console.log(`[Visualizer] Starting layout operation: ${operationName} (id: ${currentOpId})`);

    try {
      await operationFn(currentOpId);
      console.log(`[Visualizer] Completed layout operation: ${operationName} (id: ${currentOpId})`);
      return true;
    } catch (error) {
      console.error(`[Visualizer] Failed ${operationName}:`, error);
      
      // Attempt recovery by resetting layout state
      if (error.name === 'ResizeObserver' || error.message?.includes('ResizeObserver')) {
        console.warn('[Visualizer] ResizeObserver error detected, attempting recovery...');
        // Force a clean re-render after a delay
        setTimeout(() => {
          if (window.reactFlowInstance) {
            try {
              window.reactFlowInstance.fitView({ padding: 0.1, duration: 0 });
            } catch (recoveryError) {
              console.error('[Visualizer] Recovery attempt failed:', recoveryError);
            }
          }
        }, 100);
      }
      
      return false;
    } finally {
      // Always clear layouting state - remove the condition that could cause it to stick
      console.log(`[Visualizer] Clearing layout state for operation: ${operationName} (id: ${currentOpId})`);
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
        }, 'auto-collapse');
      }
    }
  }, [nodes.length, hasAutoCollapsed, childNodesByParent.size, isLayouting, performLayoutOperation, collapseAll, nodes, edges, currentLayout, setNodes]);

  // Reset auto-collapse flag when graph data changes
  useEffect(() => {
    setHasAutoCollapsed(false);
  }, [graphData]);

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

  // Handle layout change with collapsed container awareness
  const handleLayoutChange = useCallback(async (newLayout) => {
    console.log('[Visualizer] Layout change requested:', newLayout);
    setCurrentLayout(newLayout);
    
    // If we have collapsed containers, we need to re-apply layout with proper sizing
    if (hasCollapsedContainers && nodes.length > 0) {
      console.log('[Visualizer] Re-applying layout for collapsed containers');
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
      }, 'layout-change');
    } else {
      console.log('[Visualizer] No collapsed containers, normal layout change');
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
    }, 'expand-all');
  }, [expandAll, nodes, edges, currentLayout, setNodes, performLayoutOperation]);

  // Handle node clicks for expanding/collapsing containers
  const handleNodeClick = useCallback(async (event, node) => {
    console.log('[Visualizer] Node clicked:', node.type, node.id);
    
    if (node.type === 'group' || node.type === 'collapsedContainer') {
      event.stopPropagation();
      
      console.log('[Visualizer] Attempting to toggle container:', node.id);
      
      return performLayoutOperation(async (opId) => {
        // Compute the new collapsed state directly instead of waiting for state update
        const collapsedArray = Array.from(collapsedContainers);
        
        // Toggle the specific container in our computed array
        if (collapsedArray.includes(node.id)) {
          const index = collapsedArray.indexOf(node.id);
          collapsedArray.splice(index, 1);
          console.log('[Visualizer] Expanding container:', node.id);
        } else {
          collapsedArray.push(node.id);
          console.log('[Visualizer] Collapsing container:', node.id);
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
      }, `toggle-container-${node.id}`);
    }
  }, [toggleContainer, nodes, edges, currentLayout, setNodes, collapsedContainers, performLayoutOperation]);

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

  // Update ReactFlow internals when collapsed containers change
  useEffect(() => {
    if (window.reactFlowInstance && displayNodes.length > 0) {
      // Use requestAnimationFrame to ensure DOM updates are complete
      requestAnimationFrame(() => {
        if (window.reactFlowInstance) {
          displayNodes.forEach(node => {
            if (node.type === 'collapsedContainer') {
              try {
                window.reactFlowInstance.updateNodeInternals(node.id);
              } catch (error) {
                console.warn('[Visualizer] Failed to update node internals:', error);
              }
            }
          });
        }
      });
    }
  }, [displayNodes]);

  // Trigger re-render after layout readjustments
  useEffect(() => {
    if (window.reactFlowInstance && nodes.length > 0) {
      // Use requestAnimationFrame to ensure layout updates are complete
      requestAnimationFrame(() => {
        if (window.reactFlowInstance) {
          try {
            window.reactFlowInstance.fitView({ padding: 0.1, duration: 200 });
          } catch (error) {
            console.warn('[Visualizer] Failed to fit view after layout change:', error);
          }
        }
      });
    }
  }, [lastChangedContainer]);

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
  }

  return (
    <div className={styles.visualizationWrapper}>
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
      />
      
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
