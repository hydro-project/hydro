/**
 * Collapsed Container Wrapper
 * Handles collapsed container processing without causing infinite loops
 */

import React, { useMemo, useEffect } from 'react';
import { processCollapsedContainers, rerouteEdgesForCollapsedContainers } from './containerLogic.js';

export function CollapsedContainerWrapper({ nodes, edges, collapsedContainers, childNodesByParent, children }) {
  // Process nodes and edges based on collapsed state
  const { processedNodes, processedEdges } = useMemo(() => {
    if (!nodes || nodes.length === 0) {
      return { processedNodes: [], processedEdges: [] };
    }
    
    // If no containers are collapsed, return original data
    if (collapsedContainers.size === 0) {
      return { processedNodes: nodes, processedEdges: edges || [] };
    }
    
    try {
      const collapsedArray = Array.from(collapsedContainers);
      const processedNodes = processCollapsedContainers(nodes, collapsedArray);
      
      // For now, just pass through original edges to avoid the null handle ID errors
      // TODO: Implement proper edge rerouting once basic collapse/expand works
      const processedEdges = edges || [];
      
      return { processedNodes, processedEdges };
    } catch (error) {
      console.error('Error processing collapsed containers:', error);
      return { processedNodes: nodes, processedEdges: edges || [] };
    }
  }, [nodes, edges, collapsedContainers, childNodesByParent]);

  // Pass the processed nodes and edges to children
  return children({ nodes: processedNodes, edges: processedEdges });
}
