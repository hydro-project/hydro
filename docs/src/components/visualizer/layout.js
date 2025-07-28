/**
 * Simple ELK Layout Integration
 * 
 * Provides flat graph layout using ELK algorithms with shared configuration
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

  console.log('=== ELK LAYOUT DEBUG START ===');
  console.log('Input nodes to ELK:', nodes.map(n => ({
    id: n.id,
    type: n.type,
    parentId: n.parentId, // FIXED: ReactFlow v12 uses parentId
    position: n.position,
    style: n.style
  })));

  // Separate hierarchy nodes (group nodes) from regular nodes
  const hierarchyNodes = nodes.filter(node => node.type === 'group');
  const regularNodes = nodes.filter(node => node.type !== 'group');
  
  console.log('Hierarchy nodes:', hierarchyNodes.map(n => ({
    id: n.id,
    parentId: n.parentId, // FIXED: ReactFlow v12 uses parentId
    position: n.position
  })));
  console.log('Regular nodes:', regularNodes.map(n => ({
    id: n.id,
    parentId: n.parentId, // FIXED: ReactFlow v12 uses parentId
    position: n.position
  })));

  // CRITICAL: Check parent-child relationships
  console.log('=== PARENT-CHILD RELATIONSHIP DEBUG ===');
  hierarchyNodes.forEach(node => {
    const children = hierarchyNodes.filter(child => child.parentId === node.id);
    const parent = hierarchyNodes.find(p => p.id === node.parentId);
    console.log(`Node ${node.id}:`, {
      parentId: node.parentId, // FIXED: ReactFlow v12 uses parentId
      parentExists: !!parent,
      parentName: parent?.data?.label,
      childCount: children.length,
      childrenIds: children.map(c => c.id),
      hasValidParentChild: node.parentId === null || !!parent
    });
  });
  
  // Check for orphaned or incorrectly parented nodes
  const nodeIds = new Set(hierarchyNodes.map(n => n.id));
  const orphanedNodes = hierarchyNodes.filter(node => 
    node.parentId !== null && !nodeIds.has(node.parentId)
  );
  
  if (orphanedNodes.length > 0) {
    console.error('⚠️ ORPHANED NODES DETECTED - these nodes reference non-existent parents:', 
      orphanedNodes.map(n => ({id: n.id, parentId: n.parentId}))
    );
  }

  // Build ELK hierarchy structure
  function buildElkHierarchy(parentId = null) {
    const children = [];
    
    // Add hierarchy containers at this level
    const containers = hierarchyNodes.filter(node => node.parentId === parentId);
    console.log(`Building ELK hierarchy for parentId=${parentId}, found containers:`, containers.map(c => c.id));
    
    containers.forEach(container => {
      // Recursively build children for this container
      const childrenArray = buildElkHierarchy(container.id);
      
      // Create container with children
      const elkContainer = {
        id: container.id,
        width: 400, // Will be resized by ELK
        height: 250, // Will be resized by ELK
        layoutOptions: {
          'elk.padding': '[top=50,left=40,bottom=40,right=40]',
          'elk.spacing.nodeNode': 30,
          'elk.algorithm': 'layered',
          'elk.layered.spacing.nodeNodeBetweenLayers': 40,
        },
        children: childrenArray,
      };
      console.log(`Created ELK container:`, elkContainer);
      children.push(elkContainer);
    });
    
    // Add regular nodes at this level
    const levelNodes = regularNodes.filter(node => node.parentId === parentId);
    console.log(`Adding regular nodes for parentId=${parentId}:`, levelNodes.map(n => n.id));
    
    levelNodes.forEach(node => {
      const elkNode = {
        id: node.id,
        width: 200,
        height: 60,
      };
      console.log(`Created ELK node:`, elkNode);
      children.push(elkNode);
    });
    
    console.log(`Built hierarchy level for parentId=${parentId}, children:`, children.map(c => c.id));
    return children;
  }

  // Build the ELK graph with hierarchy
  const elkGraph = {
    id: 'root',
    layoutOptions: {
      ...ELK_LAYOUT_CONFIGS[layoutType],
      'elk.padding': '[top=40,left=40,bottom=40,right=40]', // More generous root padding
      'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
      'elk.spacing.nodeNode': 60, // More space between top-level containers
    },
    children: buildElkHierarchy(null), // Start with no parent (top level)
    edges: edges.map(edge => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target],
    })),
  };

  console.log('ELK Input Graph:', JSON.stringify(elkGraph, null, 2));

  try {
    const layoutResult = await elk.layout(elkGraph);
    console.log('ELK Output Result:', JSON.stringify(layoutResult, null, 2));
    
    // Apply positions back to nodes using a recursive function
    function applyPositions(elkNodes, depth = 0) {
      const layoutedNodes = [];
      const indent = '  '.repeat(depth);
      
      console.log(`${indent}Applying positions at depth ${depth}`);
      
      elkNodes.forEach(elkNode => {
        const reactFlowNode = nodes.find(n => n.id === elkNode.id);
        console.log(`${indent}Processing ELK node:`, {
          id: elkNode.id,
          x: elkNode.x,
          y: elkNode.y,
          width: elkNode.width,
          height: elkNode.height,
          hasChildren: !!elkNode.children
        });
        
        if (reactFlowNode) {
          const processedNode = {
            ...reactFlowNode,
            position: {
              x: elkNode.x || 0,
              y: elkNode.y || 0,
            },
            style: reactFlowNode.type === 'group' ? {
              ...reactFlowNode.style,
              width: elkNode.width,
              height: elkNode.height,
            } : reactFlowNode.style,
            // CRITICAL: Set extent to 'parent' for child nodes to enable proper nesting
            extent: reactFlowNode.parentId ? 'parent' : undefined,
            // For group nodes, ensure they expand to contain children
            expandParent: reactFlowNode.type === 'group',
          };
          
          console.log(`${indent}Created ReactFlow node:`, {
            id: processedNode.id,
            type: processedNode.type,
            position: processedNode.position,
            parentId: processedNode.parentId, // FIXED: ReactFlow v12 uses parentId
            extent: processedNode.extent,
            expandParent: processedNode.expandParent,
            hasStyle: !!processedNode.style,
            styleDimensions: processedNode.style?.width ? { width: processedNode.style.width, height: processedNode.style.height } : undefined,
          });
          
          layoutedNodes.push(processedNode);
        }
        
        // Recursively apply positions to children
        if (elkNode.children) {
          console.log(`${indent}Processing ${elkNode.children.length} children of ${elkNode.id}`);
          layoutedNodes.push(...applyPositions(elkNode.children, depth + 1));
        }
      });
      
      return layoutedNodes;
    }

    const layoutedNodes = applyPositions(layoutResult.children || []);
    
    console.log('=== FINAL REACTFLOW NODES DEBUG ===');
    console.log('Final layouted nodes:', layoutedNodes.map(n => ({
      id: n.id,
      type: n.type,
      position: n.position,
      parentId: n.parentId, // FIXED: ReactFlow v12 uses parentId
    })));
    
    // CRITICAL: Validate ReactFlow parent-child relationships
    console.log('=== REACTFLOW PARENT-CHILD VALIDATION ===');
    const finalGroupNodes = layoutedNodes.filter(n => n.type === 'group');
    const finalRegularNodes = layoutedNodes.filter(n => n.type !== 'group');
    
    finalGroupNodes.forEach(node => {
      const parent = layoutedNodes.find(p => p.id === node.parentId);
      const children = layoutedNodes.filter(child => child.parentId === node.id);
      
      console.log(`Final node ${node.id} (${node.data?.label}):`, {
        position: node.position,
        parentId: node.parentId, // FIXED: ReactFlow v12 uses parentId
        parentExists: !!parent,
        parentLabel: parent?.data?.label,
        childCount: children.length,
        childrenLabels: children.map(c => c.data?.label || c.id),
        style: {
          width: node.style?.width,
          height: node.style?.height,
          background: node.style?.background,
        }
      });
    });
    
    // Check if ReactFlow will understand the hierarchy
    const reactFlowNodeIds = new Set(layoutedNodes.map(n => n.id));
    const brokenParentRefs = layoutedNodes.filter(node => 
      node.parentId && !reactFlowNodeIds.has(node.parentId)
    );
    
    if (brokenParentRefs.length > 0) {
      console.error('⚠️ BROKEN PARENT REFERENCES IN FINAL REACTFLOW DATA:', 
        brokenParentRefs.map(n => ({id: n.id, parentId: n.parentId}))
      );
    }

    console.log('=== ELK LAYOUT DEBUG END ===');

    // CRITICAL: Sort nodes so parents come before children (ReactFlow v12 requirement)
    // This ensures proper hierarchy processing in ReactFlow
    const sortedNodes = [];
    const nodeMap = new Map(layoutedNodes.map(node => [node.id, node]));
    const visited = new Set();
    
    function addNodeAndParents(nodeId) {
      if (visited.has(nodeId)) return;
      
      const node = nodeMap.get(nodeId);
      if (!node) return;
      
      // First add the parent (if any)
      if (node.parentId && !visited.has(node.parentId)) {
        addNodeAndParents(node.parentId);
      }
      
      // Then add this node
      visited.add(nodeId);
      sortedNodes.push(node);
    }
    
    // Add all nodes, ensuring parents come first
    layoutedNodes.forEach(node => addNodeAndParents(node.id));
    
    console.log('Node ordering check - parents before children:');
    sortedNodes.forEach((node, index) => {
      if (node.parentId) {
        const parentIndex = sortedNodes.findIndex(n => n.id === node.parentId);
        console.log(`${node.id} (parent: ${node.parentId}, parentIndex: ${parentIndex}, thisIndex: ${index})`);
        if (parentIndex > index) {
          console.error(`❌ ORDERING ERROR: ${node.id} appears before its parent ${node.parentId}`);
        }
      }
    });

    return {
      nodes: sortedNodes,
      edges: edges,
    };

  } catch (error) {
    console.error('ELK layout failed:', error);
    throw error; // Let the error bubble up instead of silently falling back
  }
}
