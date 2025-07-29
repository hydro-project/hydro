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
  console.log('[applyLayout] Starting layout with:', { nodeCount: nodes.length, edgeCount: edges.length, layoutType });
  // Remove the detailed logging for cleaner output
  
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
      const elkContainer = {
        ...container,  // PRESERVE all original properties (style, data, type, etc.)
        id: container.id,
        width: container.style?.width || 300, // Use original width or fallback
        height: container.style?.height || 200, // Use original height or fallback
        layoutOptions: {
          ...ELK_LAYOUT_CONFIGS[layoutType], // Use the selected layout algorithm!
          'elk.padding': '[top=15,left=20,bottom=15,right=20]',
          'elk.spacing.nodeNode': 25,
          'elk.spacing.edgeNode': 15,
          'elk.spacing.edgeEdge': 10,
          'elk.spacing.borderNode': 20,
        },
        children: childElkNodes,
      };
      children.push(elkContainer);
    });
    // Add regular nodes at this level
    const levelNodes = regularNodes.filter(node => node.parentId === parentId);
    levelNodes.forEach(node => {
      const elkNode = {
        ...node,  // PRESERVE all original properties (style, data, type, etc.)
        id: node.id,
        width: node.style?.width || 200,
        height: node.style?.height || 60,
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
      'elk.spacing.edgeNode': 20, // Space between edges and nodes at root level
      'elk.spacing.edgeEdge': 15, // Space between parallel edges at root level
    },  
    children: buildElkHierarchy(null), // Start with no parent (top level)
    edges: edges.map(edge => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target],
    })),
  };

  // Debug: Log the ELK graph structure to see where nodes are placed
  console.log('[ELK] Graph structure:');
  console.log('  Root children:', elkGraph.children.map(c => `${c.id} (${c.children?.length || 0} children)`));
  console.log('  Edges:', elkGraph.edges.map(e => `${e.sources[0]} -> ${e.targets[0]}`));
  
  // Helper function to get all node IDs from hierarchy
  function getAllNodeIds(nodes) {
    const ids = [];
    nodes.forEach(node => {
      ids.push(node.id);
      if (node.children) {
        ids.push(...getAllNodeIds(node.children));
      }
    });
    return ids;
  }
  
  // Only log missing nodes, not the full list
  elkGraph.edges.forEach(edge => {
    const allNodeIds = getAllNodeIds(elkGraph.children);
    const sourceExists = allNodeIds.includes(edge.sources[0]);
    const targetExists = allNodeIds.includes(edge.targets[0]);
    if (!sourceExists || !targetExists) {
      console.error(`[ELK] Missing node for edge ${edge.sources[0]} -> ${edge.targets[0]}. Source exists: ${sourceExists}, Target exists: ${targetExists}`);
    }
  });

  try {
    const layoutResult = await elk.layout(elkGraph);

    // Apply positions back to nodes using a recursive function
    function applyPositions(elkNodes, depth = 0) {
      const layoutedNodes = [];
      elkNodes.forEach(elkNode => {
        const reactFlowNode = nodes.find(n => n.id === elkNode.id);
        if (reactFlowNode) {
          // For containers, calculate tighter bounds based on actual child positions
          if (reactFlowNode.type === 'group' && elkNode.children && elkNode.children.length > 0) {
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
            const padding = 40;
            const tightWidth = (childBounds.maxX - childBounds.minX) + padding;
            const tightHeight = (childBounds.maxY - childBounds.minY) + padding;
            elkNode.width = Math.max(tightWidth, elkNode.width * 0.6);
            elkNode.height = Math.max(tightHeight, elkNode.height * 0.6);
          }
          let width = elkNode.width;
          let height = elkNode.height;
          if (typeof width === 'undefined' || typeof height === 'undefined') {
            console.warn(`[ReactFlow] WARNING: Node ${elkNode.id} missing width/height from ELK! width=${width}, height=${height}`);
            width = width || 300;
            height = height || 200;
          }
          
          const processedNode = {
            ...reactFlowNode,
            width,
            height,
            position: {
              x: elkNode.x || 0,
              y: elkNode.y || 0,
            },
            // CRITICAL: For ReactFlow v12, custom data must be in the `data` prop.
            // We also merge in the original style to preserve gradients, etc.
            data: {
              ...reactFlowNode.data,
              nodeStyle: {
                ...reactFlowNode.style,
                width,
                height,
              },
            },
            style: {
              // This top-level style is still useful for ReactFlow's internal calculations
              width,
              height,
            },
            extent: reactFlowNode.parentId ? 'parent' : undefined,
          };
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
      
      // Shift all nodes to start from (20, 20) instead of having large offsets
      const offsetX = Math.max(0, bbox.minX - 20);
      const offsetY = Math.max(0, bbox.minY - 20);
      
      if (offsetX > 0 || offsetY > 0) {
        sortedNodes.forEach(node => {
          if (node.position) {
            node.position.x -= offsetX;
            node.position.y -= offsetY;
          }
        });
      }
    }

    console.log('[applyLayout] Layout complete');
    // Remove detailed node logging for cleaner output
    
    return {
      nodes: sortedNodes,
      edges: edges,
    };

  } catch (error) {
    console.error('ELK layout failed:', error);
    throw error; // Let the error bubble up instead of silently falling back
  }
}
