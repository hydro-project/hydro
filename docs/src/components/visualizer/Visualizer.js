/**
 * Clean Hydro Graph Visualizer
 * 
 * A simplified, flat graph visualizer using ReactFlow v12 and ELK layout.
 */

import React, { useState, useEffect, useCallback } from 'react';
import { 
  useNodesState, 
  useEdgesState
} from '@xyflow/react';
import { applyLayout } from './utils/layout.js';
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
  }, [graphData, currentGrouping]);

  // Handle grouping change
  const handleGroupingChange = useCallback((newGrouping) => {
    setCurrentGrouping(newGrouping);
  }, []);

  // Handle node clicks for expanding/collapsing containers
  const handleNodeClick = useCallback((event, node) => {
    if (node.type === 'group' || node.type === 'collapsedContainer') {
      event.stopPropagation();
      toggleContainer(node.id);
    }
  }, [toggleContainer]);

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
  }, [graphData, currentLayout, colorPalette, currentGrouping, setNodes, setEdges]); // Add currentGrouping to dependencies

  // Handle collapsed container changes by updating the nodes state directly
  useEffect(() => {
    if (nodes.length === 0) return; // Don't process if no nodes
    
    
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
      if (hasNodeChanges) {
        setNodes(processedNodes);
      }
      if (hasEdgeChanges) {
        setEdges(processedEdges);
      }
      
      // Force ReactFlow to update node internals (dimensions, etc.)
      setTimeout(() => {
        if (window.reactFlowInstance) {
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
        hierarchyChoices={hierarchyChoices}
        currentGrouping={currentGrouping}
        onGroupingChange={handleGroupingChange}
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
