/**
 * Simple ELK Layout Integration
 * 
 * Provides flat graph layout using ELK algorithms
 */

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

const layoutConfigs = {
  mrtree: {
    'elk.algorithm': 'mrtree',
    'elk.direction': 'DOWN',
    'elk.spacing.nodeNode': 50,
    'elk.spacing.edgeNode': 20,
  },
  layered: {
    'elk.algorithm': 'layered',
    'elk.direction': 'DOWN',
    'elk.spacing.nodeNode': 30,
    'elk.layered.spacing.nodeNodeBetweenLayers': 50,
  },
  force: {
    'elk.algorithm': 'force',
    'elk.spacing.nodeNode': 100,
  },
  stress: {
    'elk.algorithm': 'stress',
    'elk.spacing.nodeNode': 100,
  },
  radial: {
    'elk.algorithm': 'radial',
    'elk.spacing.nodeNode': 100,
  },
};

export async function applyLayout(nodes, edges, layoutType = 'mrtree') {
  const elk = await loadELK();
  
  if (!elk) {
    console.warn('ELK not available, using default positions');
    return { nodes, edges };
  }

  // Convert to ELK format
  const elkNodes = nodes.map(node => ({
    id: node.id,
    width: 200,
    height: 60,
  }));

  const elkEdges = edges.map(edge => ({
    id: edge.id,
    sources: [edge.source],
    targets: [edge.target],
  }));

  const elkGraph = {
    id: 'root',
    layoutOptions: {
      ...layoutConfigs[layoutType],
      'elk.padding': '[top=20,left=20,bottom=20,right=20]',
    },
    children: elkNodes,
    edges: elkEdges,
  };

  try {
    const layoutResult = await elk.layout(elkGraph);
    
    // Apply positions back to nodes
    const layoutedNodes = nodes.map(node => {
      const elkNode = layoutResult.children?.find(n => n.id === node.id);
      
      if (elkNode) {
        return {
          ...node,
          position: {
            x: elkNode.x || 0,
            y: elkNode.y || 0,
          },
        };
      }
      
      return node;
    });

    return {
      nodes: layoutedNodes,
      edges: edges, // Edges unchanged
    };

  } catch (error) {
    console.error('ELK layout failed:', error);
    return { nodes, edges };
  }
}
