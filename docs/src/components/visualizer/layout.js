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
      const elkContainer = {
        ...container,  // PRESERVE all original properties (style, data, type, etc.)
        id: container.id,
        // Let ELK calculate container size - temporarily remove padding to test
        layoutOptions: {
          ...ELK_LAYOUT_CONFIGS[layoutType], // Use the selected layout algorithm!
          'elk.spacing.nodeNode': 20,
          'elk.spacing.edgeNode': 15,
          'elk.spacing.edgeEdge': 10,
        },
        children: childElkNodes,
      };
      children.push(elkContainer);
    });
    // Add regular nodes at this level - let ELK handle sizing
    const levelNodes = regularNodes.filter(node => node.parentId === parentId);
    levelNodes.forEach(node => {
      const elkNode = {
        ...node,  // PRESERVE all original properties (style, data, type, etc.)
        id: node.id,
        // Use existing dimensions if available, otherwise let ELK decide
        width: node.style?.width || node.width || 200,
        height: node.style?.height || node.height || 60,
      };
      children.push(elkNode);
    });
    return children;
  }

  // Build the ELK graph with hierarchy - let ELK handle all sizing and positioning
  const elkGraph = {
    id: 'root',
    layoutOptions: {
      ...ELK_LAYOUT_CONFIGS[layoutType],
      'elk.padding': '[top=20,left=20,bottom=20,right=20]', // Root level padding for canvas margins
      'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
      'elk.spacing.nodeNode': 40, // Spacing between top-level containers
      'elk.spacing.edgeNode': 20,
      'elk.spacing.edgeEdge': 15,
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

    // Apply positions back to nodes - debug coordinate systems
    function applyPositions(elkNodes, depth = 0) {
      const layoutedNodes = [];
      elkNodes.forEach(elkNode => {
        const reactFlowNode = nodes.find(n => n.id === elkNode.id);
        if (reactFlowNode) {
          // Debug: Log positions for child nodes to see the offset
          if (reactFlowNode.parentId) {
            console.log(`[DEBUG] Child node ${elkNode.id}: ELK pos=(${elkNode.x}, ${elkNode.y}), parent=${reactFlowNode.parentId}`);
          }
          
          // Use ELK's results exactly as calculated - no adjustments
          const processedNode = {
            ...reactFlowNode,
            width: elkNode.width,
            height: elkNode.height,
            position: {
              x: elkNode.x || 0,
              y: elkNode.y || 0,
            },
            data: {
              ...reactFlowNode.data,
              nodeStyle: {
                ...reactFlowNode.style,
                width: elkNode.width,
                height: elkNode.height,
              },
            },
            style: {
              width: elkNode.width,
              height: elkNode.height,
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

    console.log('[applyLayout] Layout complete');
    
    return {
      nodes: sortedNodes,
      edges: edges,
    };

  } catch (error) {
    console.error('ELK layout failed:', error);
    throw error; // Let the error bubble up instead of silently falling back
  }
}
