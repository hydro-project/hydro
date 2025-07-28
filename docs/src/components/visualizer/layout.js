/**
 * Simple ELK Layout Integration
 * 
 * Provides        layoutOptions: {
          'elk.spacing.nodeNode': 30,
          'elk.algorithm': 'layered',
          'elk.layered.spacing.nodeNodeBetweenLayers': 40,
          'elk.layered.spacing.borderToNode': 20,
        },
        children: buildElkHierarchy(container.id),
      };aph layout using ELK algorithms with shared configuration
 */

import { ELK_LAYOUT_CONFIGS } from './reactFlowConfig.js';

let ELK = null;

// Load ELK dynamically
async function loadELK() {
  if (ELK) return ELK;
  
  try {
    const elkModule = await import('elkjs');
    ELK = new elkModule.default();
    return ELK;
  } catch (error) {
    console.error('Failed to load ELK:', error);
    return null;
  }
}

export async function applyLayout(nodes, edges, layoutType = 'mrtree') {
  const elk = await loadELK();
  
  if (!elk) {
    console.error('ELK not available - this is a critical error');
    throw new Error('ELK layout engine failed to load');
  }

  const hierarchyNodes = nodes.filter(node => node.type === 'group');
  const regularNodes = nodes.filter(node => node.type !== 'group');

  // Build ELK hierarchy structure
  function buildElkHierarchy(parentId = null) {
    const children = [];
    // Add hierarchy containers at this level
    const containers = hierarchyNodes.filter(node => node.parentId === parentId);
    containers.forEach(container => {
      // Recursively build children for this container
      const childElkNodes = buildElkHierarchy(container.id);
      // Diagnostic: log child count and sum of child widths
      const childWidths = childElkNodes
        .filter(n => n.width)
        .map(n => n.width);
      const sumChildWidths = childWidths.reduce((a, b) => a + b, 0);
      console.log(`[DIAG] Container ${container.id} has ${childElkNodes.length} children, sumChildWidths=${sumChildWidths}`);
      console.log(`[DIAG] Container ${container.id} children:`, childElkNodes.map(n => ({ id: n.id, type: n.type, width: n.width, height: n.height })));
      console.log(`[DIAG] Container ${container.id} layoutOptions:`, {
        'elk.padding': '[top=15,left=20,bottom=15,right=20]',
        'elk.spacing.nodeNode': 25,
        'elk.algorithm': 'mrtree',
        'elk.direction': 'DOWN',
      });
      const elkContainer = {
        id: container.id,
        // CRITICAL: Add explicit width/height for containers so ELK knows their size
        width: 300, // Provide initial size hint
        height: 200, // Provide initial size hint
        layoutOptions: {
          'elk.padding': '[top=15,left=20,bottom=15,right=20]', // More breathing room
          'elk.spacing.nodeNode': 25, // Increased from 10
          // Use the SAME algorithm as the root for consistency
          'elk.algorithm': 'mrtree', // Match the root algorithm
          'elk.direction': 'DOWN',
        },
        children: childElkNodes,
      };
      children.push(elkContainer);
    });
    // Add regular nodes at this level
    const levelNodes = regularNodes.filter(node => node.parentId === parentId);
    levelNodes.forEach(node => {
      const elkNode = {
        id: node.id,
        width: 200,
        height: 60,
      };
      children.push(elkNode);
    });
    return children;
  }

  // Build the ELK graph with hierarchy
  const elkGraph = {
    id: 'root',
    layoutOptions: {
      ...ELK_LAYOUT_CONFIGS[layoutType],
      'elk.padding': '[top=30,left=30,bottom=30,right=30]', // More breathing room for root
      'elk.hierarchyHandling': 'INCLUDE_CHILDREN', // Back to INCLUDE_CHILDREN - this is more canonical
      'elk.spacing.nodeNode': 50, // Increased spacing between top-level containers
    },
    children: buildElkHierarchy(null), // Start with no parent (top level)
    edges: edges.map(edge => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target],
    })),
  };
  console.log('[DIAG] ELK input graph:', JSON.stringify(elkGraph, null, 2));

  try {
    const layoutResult = await elk.layout(elkGraph);
    console.log('[DIAG] ELK layout result:', JSON.stringify(layoutResult, null, 2));

    // Apply positions back to nodes using a recursive function
    function applyPositions(elkNodes, depth = 0) {
      const layoutedNodes = [];
      elkNodes.forEach(elkNode => {
        const reactFlowNode = nodes.find(n => n.id === elkNode.id);
        if (reactFlowNode) {
          // Log ELK's calculated dimensions for containers
          if (reactFlowNode.type === 'group') {
            console.log(`[ELK] Container ${elkNode.id} dimensions: width=${elkNode.width}, height=${elkNode.height}, x=${elkNode.x}, y=${elkNode.y}`);
            
            // For containers, calculate tighter bounds based on actual child positions
            if (elkNode.children && elkNode.children.length > 0) {
              const childBounds = elkNode.children.reduce((bounds, child) => {
                const right = (child.x || 0) + (child.width || 0);
                const bottom = (child.y || 0) + (child.height || 0);
                return {
                  minX: Math.min(bounds.minX, child.x || 0),
                  minY: Math.min(bounds.minY, child.y || 0),
                  maxX: Math.max(bounds.maxX, right),
                  maxY: Math.max(bounds.maxY, bottom),
                };
              }, { minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity });
              
              // Add padding to the tight bounds
              const padding = 40; // Updated to match new padding: 20 left/right + 15 top/bottom
              const tightWidth = (childBounds.maxX - childBounds.minX) + padding;
              const tightHeight = (childBounds.maxY - childBounds.minY) + padding;
              
              console.log(`[TIGHT] Container ${elkNode.id} tight bounds: width=${tightWidth}, height=${tightHeight} (vs ELK: ${elkNode.width}x${elkNode.height})`);
              
              elkNode.width = Math.max(tightWidth, elkNode.width * 0.6); // Use tighter bounds but not smaller than 60% of ELK's calculation
              elkNode.height = Math.max(tightHeight, elkNode.height * 0.6);
            }
          }
          const processedNode = {
            ...reactFlowNode,
            position: {
              x: elkNode.x || 0,
              y: elkNode.y || 0,
            },
            style: reactFlowNode.type === 'group' ? {
              ...reactFlowNode.style,
              width: elkNode.width || 300,
              height: elkNode.height || 200,
            } : reactFlowNode.style,
            extent: reactFlowNode.parentId ? 'parent' : undefined,
            expandParent: reactFlowNode.type === 'group',
          };
          // Log dimensions being applied to ReactFlow containers
          if (processedNode.type === 'group') {
            console.log(`[ReactFlow] Applying to container ${processedNode.id}: width=${processedNode.style.width}, height=${processedNode.style.height}, x=${processedNode.position.x}, y=${processedNode.position.y}`);
          }
          layoutedNodes.push(processedNode);
        }
        // Recursively apply positions to children
        if (elkNode.children) {
          layoutedNodes.push(...applyPositions(elkNode.children, depth + 1));
        }
      });
      return layoutedNodes;
    }
    const layoutedNodes = applyPositions(layoutResult.children || []);

    // CRITICAL: Sort nodes so parents come before children (ReactFlow v12 requirement)
    const sortedNodes = [];
    const nodeMap = new Map(layoutedNodes.map(node => [node.id, node]));
    const visited = new Set();
    
    function addNodeAndParents(nodeId) {
      if (visited.has(nodeId)) return;
      
      const node = nodeMap.get(nodeId);
      if (!node) return;
      
      if (node.parentId && !visited.has(node.parentId)) {
        addNodeAndParents(node.parentId);
      }
      
      visited.add(nodeId);
      sortedNodes.push(node);
    }
    
    layoutedNodes.forEach(node => addNodeAndParents(node.id));

    // Post-layout optimization: Calculate tight bounding box and adjust positions
    const allNodes = sortedNodes.filter(n => n.position);
    if (allNodes.length > 0) {
      const bbox = allNodes.reduce((bounds, node) => {
        const nodeRight = node.position.x + (node.style?.width || 200);
        const nodeBottom = node.position.y + (node.style?.height || 60);
        return {
          minX: Math.min(bounds.minX, node.position.x),
          minY: Math.min(bounds.minY, node.position.y),
          maxX: Math.max(bounds.maxX, nodeRight),
          maxY: Math.max(bounds.maxY, nodeBottom),
        };
      }, { minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity });

      console.log(`[BBOX] Layout bounding box: minX=${bbox.minX}, minY=${bbox.minY}, maxX=${bbox.maxX}, maxY=${bbox.maxY}`);
      
      // Shift all nodes to start from (20, 20) instead of having large offsets
      const offsetX = Math.max(0, bbox.minX - 20);
      const offsetY = Math.max(0, bbox.minY - 20);
      
      if (offsetX > 0 || offsetY > 0) {
        console.log(`[BBOX] Applying offset: x=-${offsetX}, y=-${offsetY}`);
        sortedNodes.forEach(node => {
          if (node.position) {
            node.position.x -= offsetX;
            node.position.y -= offsetY;
          }
        });
      }
    }

    return {
      nodes: sortedNodes,
      edges: edges,
    };

  } catch (error) {
    console.error('ELK layout failed:', error);
    throw error; // Let the error bubble up instead of silently falling back
  }
}
