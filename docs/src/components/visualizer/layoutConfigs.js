/**
 * Enhanced Layout Configurations for ELK.js with ReactFlow v12
 * 
 * Optimized for ReactFlow v12's measured dimensions and sub-flow capabilities
 * All configurations leverage ELK's full potential for hierarchical layouts
 */

// Enhanced ELK layout configurations optimized for ReactFlow v12
export const elkLayouts = {
  layered: {
    'elk.algorithm': 'layered',
    'elk.layered.spacing.nodeNodeBetweenLayers': 40,
    'elk.spacing.nodeNode': 25,
    'elk.spacing.componentComponent': 30,
    'elk.direction': 'RIGHT',
    'elk.layered.thoroughness': 7,
    'elk.hierarchyHandling': 'SEPARATE_CHILDREN',
    // ReactFlow v12: Better node size handling
    'elk.nodeSize.constraints': 'NODE_LABELS',
    'elk.nodeSize.options': 'DEFAULT_MINIMUM_SIZE COMPUTE_NODE_LABELS',
    // Enhanced edge routing
    'elk.edgeRouting': 'ORTHOGONAL',
    'elk.layered.edgeRouting.selfLoopDistribution': 'EQUALLY',
  },
  mrtree: {
    'elk.algorithm': 'mrtree',
    'elk.mrtree.searchOrder': 'DFS',
    'elk.spacing.nodeNode': 30,
    'elk.spacing.componentComponent': 35,
    'elk.hierarchyHandling': 'SEPARATE_CHILDREN',
    'elk.nodeSize.constraints': 'NODE_LABELS',
    'elk.mrtree.weighting': 'DESCENDANTS'
  },
  force: {
    'elk.algorithm': 'force',
    'elk.force.repulsivePower': 0.6,
    'elk.force.temperature': 0.3,
    'elk.spacing.nodeNode': 40,
    'elk.spacing.componentComponent': 45,
    'elk.hierarchyHandling': 'SEPARATE_CHILDREN',
    'elk.nodeSize.constraints': 'NODE_LABELS'
  },
  stress: {
    'elk.algorithm': 'stress',
    'elk.stress.desiredEdgeLength': 50,
    'elk.stress.dimension': 'XY',
    'elk.spacing.nodeNode': 30,
    'elk.spacing.componentComponent': 35,
    'elk.hierarchyHandling': 'SEPARATE_CHILDREN',
    'elk.nodeSize.constraints': 'NODE_LABELS'
  },
  radial: {
    'elk.algorithm': 'radial',
    'elk.radial.radius': 120,
    'elk.radial.compactor': 'WEDGE_COMPACTION',
    'elk.spacing.nodeNode': 25,
    'elk.spacing.componentComponent': 30,
    'elk.hierarchyHandling': 'SEPARATE_CHILDREN'
  },
  disco: {
    'elk.algorithm': 'disco',
    'elk.disco.componentCompaction.strategy': 'POLYOMINO',
    'elk.spacing.nodeNode': 30,
    'elk.hierarchyHandling': 'INCLUDE_CHILDREN'
  }
};
