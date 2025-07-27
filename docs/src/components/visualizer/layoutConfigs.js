/**
 * Layout Configurations for ELK.js
 * 
 * Contains all ELK layout algorithm configurations with compact spacing
 * to prevent oversized containers
 */

// ELK layout configurations with VERY COMPACT spacing to prevent huge containers
export const elkLayouts = {
  layered: {
    'elk.algorithm': 'layered',
    'elk.layered.spacing.nodeNodeBetweenLayers': 30, // Reduced from 80
    'elk.spacing.nodeNode': 20, // Reduced from 60
    'elk.spacing.componentComponent': 20, // Reduced from 40
    'elk.direction': 'RIGHT',
    'elk.layered.thoroughness': 7,
    'elk.hierarchyHandling': 'SEPARATE_CHILDREN'
  },
  mrtree: {
    'elk.algorithm': 'mrtree',
    'elk.mrtree.searchOrder': 'DFS',
    'elk.spacing.nodeNode': 20, // Reduced from 60
    'elk.spacing.componentComponent': 20, // Reduced from 40
    'elk.hierarchyHandling': 'SEPARATE_CHILDREN'
  },
  force: {
    'elk.algorithm': 'force',
    'elk.force.repulsivePower': 0.5,
    'elk.spacing.nodeNode': 30, // Reduced from 80
    'elk.spacing.componentComponent': 25, // Reduced from 50
    'elk.hierarchyHandling': 'SEPARATE_CHILDREN'
  },
  stress: {
    'elk.algorithm': 'stress',
    'elk.stress.desiredEdgeLength': 30, // Reduced from 80
    'elk.spacing.nodeNode': 20, // Reduced from 60
    'elk.spacing.componentComponent': 20, // Reduced from 40
    'elk.hierarchyHandling': 'SEPARATE_CHILDREN'
  },
  radial: {
    'elk.algorithm': 'radial',
    'elk.radial.radius': 100, // Reduced from 150
    'elk.spacing.nodeNode': 20, // Reduced from 60
    'elk.spacing.componentComponent': 20, // Reduced from 40
    'elk.hierarchyHandling': 'SEPARATE_CHILDREN'
  },
  disco: {
    'elk.algorithm': 'disco',
    'elk.disco.componentCompaction.strategy': 'POLYOMINO',
    'elk.spacing.nodeNode': 25, // Reduced from 50
    'elk.hierarchyHandling': 'INCLUDE_CHILDREN'
  }
};
