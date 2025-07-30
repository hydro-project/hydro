/**
 * Clean Hydro Graph Visualizer
 * 
 * A simplified, flat graph visualizer using ReactFlow v12 and ELK layout.
 */

import React, { useState, useEffect, useCallback, useRef } from 'react';
import { 
  useNodesState, 
  useEdgesState,
  applyNodeChanges, 
  applyEdgeChanges 
} from '@xyflow/react';
import { applyLayout } from './utils/layout.js';
import { LayoutControls } from './components/LayoutControls.js';
import { Legend } from './components/Legend.js';
import { ReactFlowInner } from './components/ReactFlowInner.js';
import { processGraphData } from './utils/reactFlowConfig.js';
import { useCollapsedContainers } from './containers/useCollapsedContainers.js';
import { processCollapsedContainers, rerouteEdgesForCollapsedContainers } from './containers/containerLogic.js';
import styles from '../../pages/visualizer.module.css';

export function Visualizer({ graphData }) {
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [currentLayout, setCurrentLayout] = useState('mrtree');
  const [colorPalette, setColorPalette] = useState('Set3');
  const [isLoading, setIsLoading] = useState(false);
  
  // Collapsed containers state
  const {
    collapsedContainers,
    toggleContainer,
    isCollapsed,
    childNodesByParent,
    collapseAll,
    expandAll,
    hasCollapsedContainers,
  } = useCollapsedContainers(nodes);

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

  // Handle node clicks for expanding/collapsing containers
  const handleNodeClick = useCallback((event, node) => {
    console.log('Node clicked:', node.id, node.type);
    if (node.type === 'group' || node.type === 'collapsedContainer') {
      console.log('Toggling container:', node.id);
      event.stopPropagation();
      toggleContainer(node.id);
    }
  }, [toggleContainer]);

  // Process graph data and apply layout with proper memoization
  useEffect(() => {
    console.log('[DEBUG] useEffect triggered for graphData change');
    
    let isCancelled = false;
    
    const processGraph = async () => {
      if (!graphData || !graphData.nodes || graphData.nodes.length === 0) {
        if (!isCancelled) {
          setNodes([]);
          setEdges([]);
        }
        return;
      }
      
      setIsLoading(true);

      try {
        const result = await processGraphData(graphData, colorPalette, currentLayout, applyLayout);
        
        if (isCancelled) return; // Don't update state if component unmounted or effect cancelled
        
        // Ensure nodes have unique IDs to prevent ReactFlow duplication
        const uniqueNodes = result.nodes.filter((node, index, array) => 
          array.findIndex(n => n.id === node.id) === index
        );
        
        console.log(`[DEBUG] Deduplication: ${result.nodes.length} -> ${uniqueNodes.length} nodes`);
        
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
  }, [graphData, currentLayout, colorPalette, setNodes, setEdges]); // Remove collapsed containers dependencies

  // Handle collapsed container changes by updating the nodes state directly
  useEffect(() => {
    if (nodes.length === 0) return; // Don't process if no nodes
    
    console.log('Applying collapsed container changes');
    
    const collapsedArray = Array.from(collapsedContainers);
    const processedNodes = processCollapsedContainers(nodes, collapsedArray);
    
    // Also process edges to reroute them for collapsed containers
    const processedEdges = rerouteEdgesForCollapsedContainers(
      edges,
      processedNodes,
      childNodesByParent,
      collapsedArray
    );
    
    // Only update if there are actual changes - use a more robust comparison
    const hasNodeChanges = processedNodes.some((node, i) => {
      const original = nodes[i];
      return !original || 
             node.hidden !== original.hidden || 
             node.type !== original.type ||
             node.width !== original.width ||
             node.height !== original.height;
    }) || processedNodes.length !== nodes.length;
    
    const hasEdgeChanges = processedEdges.some((edge, i) => {
      const original = edges[i];
      return !original || 
             edge.hidden !== original.hidden || 
             edge.source !== original.source ||
             edge.target !== original.target;
    }) || processedEdges.length !== edges.length;
    
    if (hasNodeChanges || hasEdgeChanges) {
      console.log('Updating nodes/edges with collapsed container changes');
      if (hasNodeChanges) {
        setNodes(processedNodes);
      }
      if (hasEdgeChanges) {
        setEdges(processedEdges);
      }
      
      // Force ReactFlow to update node internals (dimensions, etc.)
      setTimeout(() => {
        if (window.reactFlowInstance) {
          console.log('Forcing ReactFlow to update node internals');
          processedNodes.forEach(node => {
            if (node.type === 'collapsedContainer') {
              window.reactFlowInstance.updateNodeInternals(node.id);
            }
          });
        }
      }, 100);
    }
  }, [collapsedContainers, childNodesByParent, nodes.length, edges.length]); // Use lengths instead of full arrays

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
    console.log(`[DEBUG] Node types:`, nodeTypeCounts);
  }

  return (
    <div className={styles.visualizationWrapper}>
      <LayoutControls 
        currentLayout={currentLayout}
        onLayoutChange={setCurrentLayout}
        colorPalette={colorPalette}
        onPaletteChange={setColorPalette}
        hasCollapsedContainers={hasCollapsedContainers}
        onCollapseAll={collapseAll}
        onExpandAll={expandAll}
      />
      
      <Legend colorPalette={colorPalette} />
      
      <ReactFlowInner
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeClick={handleNodeClick}
        colorPalette={colorPalette}
      />
    </div>
  );
}
