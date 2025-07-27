/**
 * Layout Algorithms
 * 
 * Contains the hierarchical layout algorithm and helper functions
 * for positioning nodes using ELK.js
 */

import { elkLayouts } from './layoutConfigs.js';
import { generateLocationColor, generateLocationBorderColor } from './colorUtils.js';
import { ELK } from './externalLibraries.js';

// This function is now unused in the hierarchical approach but kept for potential simple layouts.
export const applyElkLayout = async (nodes, edges, layoutType = 'layered') => {
  if (!ELK) return nodes;
  
  const graph = {
    id: 'root',
    layoutOptions: elkLayouts[layoutType] || elkLayouts.layered,
    children: nodes.map(node => ({
      id: node.id,
      width: 200,
      height: 60,
    })),
    edges: edges.map(edge => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target]
    }))
  };
  
  try {
    const elkResult = await ELK.layout(graph);
    return nodes.map(node => {
      const elkNode = elkResult.children?.find(n => n.id === node.id);
      if (elkNode) {
        return {
          ...node,
          position: { x: elkNode.x || 0, y: elkNode.y || 0 }
        };
      }
      return node;
    });
  } catch (error) {
    console.error('ELK layout failed:', error);
    return nodes;
  }
};

// NEW HIERARCHICAL LAYOUT APPROACH
export const applyHierarchicalLayout = async (nodes, edges, layoutType, locations, currentPalette, collapsedContainers = {}, handleContainerToggle, isDraggedRef) => {
  if (!ELK) {
    console.log(`🚨 HIERARCHICAL LAYOUT ABORT: ELK not available`);
    return { nodes, edges };
  }

  const nodeMap = new Map(nodes.map(n => [n.id, n]));
  const locationGroups = new Map();
  const orphanNodeIds = new Set(nodes.map(n => n.id));

  // 1. Group nodes by location, using the passed-in 'locations' map.
  // This is more robust than iterating over location.nodes.
  nodes.forEach(node => {
    const locationId = node.data?.locationId;
    if (locationId !== undefined && locationId !== null) {
      if (!locationGroups.has(locationId)) {
        const location = locations.get(locationId);
        if (location) {
          locationGroups.set(locationId, { location, nodeIds: new Set() });
        } else {
          console.warn(`Could not find location metadata for locationId: ${locationId}`);
        }
      }
      
      const group = locationGroups.get(locationId);
      if (group) {
        group.nodeIds.add(node.id);
        orphanNodeIds.delete(node.id);
      }
    }
  });

  // Build the set of all node IDs that will exist in the ELK graph
  const elkChildren = [];
  
  // Add container nodes to ELK graph
  locationGroups.forEach(({ location, nodeIds }) => {
    const containerId = `container_${location.id}`;
    const isCollapsed = collapsedContainers[containerId];
    
    if (isCollapsed) {
      // If collapsed, treat the container as a single node
      elkChildren.push({
        id: containerId,
        width: 200, // Standard collapsed container size
        height: 60,
        // Mark as collapsed for later processing
        isCollapsed: true,
        originalNodeIds: Array.from(nodeIds)
      });
    } else {
      // If expanded, include all child nodes (no label nodes in ELK)
      const childElkNodes = Array.from(nodeIds).map(nodeId => {
        const node = nodeMap.get(nodeId);
        return {
          id: node.id,
          width: parseFloat(node.style.width),
          height: parseFloat(node.style.height)
        };
      });

      elkChildren.push({
        id: containerId,
        children: childElkNodes,
        layoutOptions: {
          'elk.padding': '[top=50,left=30,bottom=30,right=30]',
          ...elkLayouts[layoutType]
        }
      });
    }
  });

  // Add orphan nodes to ELK graph
  orphanNodeIds.forEach(nodeId => {
    const node = nodeMap.get(nodeId);
    elkChildren.push({ id: node.id, width: node.style.width, height: node.style.height });
  });

  // Build the set of all node IDs that will exist in the ELK graph
  const existingNodeIds = new Set();
  elkChildren.forEach(child => {
    existingNodeIds.add(child.id);
    if (child.children) {
      child.children.forEach(subchild => {
        existingNodeIds.add(subchild.id);
      });
    }
  });
  
  // Filter and reroute edges to only reference existing nodes
  const validElkEdges = [];
  edges.forEach(edge => {
    let sourceId = edge.source;
    let targetId = edge.target;
    
    // Check if source node is in a collapsed container
    const sourceNode = nodeMap.get(edge.source);
    if (sourceNode?.data?.locationId !== undefined) {
      const sourceContainerId = `container_${sourceNode.data.locationId}`;
      if (collapsedContainers[sourceContainerId]) {
        sourceId = sourceContainerId;
      }
    }
    
    // Check if target node is in a collapsed container
    const targetNode = nodeMap.get(edge.target);
    if (targetNode?.data?.locationId !== undefined) {
      const targetContainerId = `container_${targetNode.data.locationId}`;
      if (collapsedContainers[targetContainerId]) {
        targetId = targetContainerId;
      }
    }
    
    // Only add edge if both endpoints exist in the ELK graph and aren't the same
    if (existingNodeIds.has(sourceId) && existingNodeIds.has(targetId) && sourceId !== targetId) {
      const newEdge = {
        id: `${sourceId}_to_${targetId}`,
        sources: [sourceId],
        targets: [targetId]
      };
      validElkEdges.push(newEdge);
    }
  });

  const elkGraph = {
    id: 'root',
    layoutOptions: {
      ...(elkLayouts[layoutType] || elkLayouts.mrtree),
      'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
    },
    children: elkChildren,
    edges: validElkEdges
  };

  // 3. Apply ELK layout
  const layoutedGraph = await ELK.layout(elkGraph);

  // 4. Process the layout result to create React Flow nodes
  const finalNodes = [];
  const layoutedNodeMap = new Map();
  const containerNodes = [];
  const childAndOrphanNodes = [];

  // First pass: process layouted graph to establish a map of all nodes and their positions
  layoutedGraph.children.forEach(elkNode => {
    layoutedNodeMap.set(elkNode.id, elkNode);
    if (elkNode.children) {
      elkNode.children.forEach(child => {
        // Pass parent's absolute position to children for relative calculation
        child.parentX = elkNode.x;
        child.parentY = elkNode.y;
        layoutedNodeMap.set(child.id, child);
      });
    }
  });

  // Second pass: Create all container nodes first
  layoutedGraph.children.forEach(elkNode => {
    if (elkNode.children || elkNode.isCollapsed) { // It's a container (expanded or collapsed)
      const locationId = parseInt(elkNode.id.replace('container_', ''), 10);
      const location = locations.get(locationId);
      const isCollapsed = collapsedContainers[elkNode.id];

      if (!location) {
        console.warn(`Could not find location metadata for container ${elkNode.id}. This might be due to a mismatch in location IDs. Skipping container rendering.`);
        // Even if we skip the container, we should still process its children as orphans.
        if (elkNode.children) {
          elkNode.children.forEach(child => {
            layoutedNodeMap.set(child.id, { ...child, isOrphan: true });
          });
        }
        return;
      }

      // Create container node with appropriate styling
      const containerStyle = {
        width: elkNode.width,
        height: elkNode.height,
        backgroundColor: generateLocationColor(location.id, locations.size, currentPalette),
        borderRadius: '8px',
        zIndex: 1,
      };

      // Add visual indication for collapsed state
      if (isCollapsed) {
        containerStyle.opacity = 0.8;
        containerStyle.border = `2px dashed ${generateLocationBorderColor(location.id, locations.size, currentPalette)}`;
        containerStyle.backgroundColor = generateLocationColor(location.id, locations.size, currentPalette).replace('40', '60'); // More opaque
        
        // Add content display styles for collapsed containers
        containerStyle.display = 'flex';
        containerStyle.alignItems = 'center';
        containerStyle.justifyContent = 'center';
        containerStyle.color = '#333';
        containerStyle.fontSize = '12px';
        containerStyle.fontWeight = '500';
        containerStyle.textAlign = 'center';
        containerStyle.padding = '10px';
      } else {
        containerStyle.border = `2px solid ${generateLocationBorderColor(location.id, locations.size, currentPalette)}`;
      }

      containerNodes.push({
        id: elkNode.id,
        type: 'container', // Use the new custom container node type
        position: { x: elkNode.x, y: elkNode.y },
        style: containerStyle,
        data: {
          label: isCollapsed ? `${location.label || `Location ${location.id}`} (${elkNode.originalNodeIds?.length || 0} nodes)` : location.label || `Location ${location.id}`,
          isContainer: true,
          locationId: location.id,
          isCollapsed: isCollapsed,
          nodeCount: elkNode.originalNodeIds?.length || 0,
          onContainerToggle: handleContainerToggle, // Pass the stable handler directly
          isDraggedRef: isDraggedRef, // Pass the drag tracking ref
        },
        draggable: true,
        selectable: true, // CHANGED: Make selectable to see if it helps with click detection
        connectable: true,
      });
    }
  });

  const validContainerIds = new Set(containerNodes.map(n => n.id));

  // Third pass: Create all child and orphan nodes, including ELK-positioned labels
  nodes.forEach(originalNode => {
    const locationId = originalNode.data?.locationId;
    const isChild = locationId !== undefined && locationId !== null;
    const containerId = isChild ? `container_${locationId}` : null;
    const isContainerCollapsed = containerId && collapsedContainers[containerId];

    // Skip processing nodes that are in collapsed containers entirely
    if (isChild && isContainerCollapsed) {
      return;
    }

    const elkNode = layoutedNodeMap.get(originalNode.id);
    if (!elkNode) {
      console.warn(`Node ${originalNode.id} not found in ELK layout result.`);
      return;
    }

    if (isChild && validContainerIds.has(containerId)) {
      // It's a child of a valid, existing container
      childAndOrphanNodes.push({
        ...originalNode,
        position: {
          x: elkNode.x, // Position is relative to parent
          y: elkNode.y,
        },
        parentNode: containerId,
        extent: 'parent',
        style: { ...originalNode.style, zIndex: 10 },
        connectable: false, // Regular nodes should not be connectable
      });
    } else {
      // It's an orphan (or its parent container was invalid and not created)
      childAndOrphanNodes.push({
        ...originalNode,
        position: {
          x: elkNode.x, // For orphans from invalid containers, position is absolute
          y: elkNode.y,
        },
        connectable: false, // Regular nodes should not be connectable
      });
    }
  });

  // Process labels for expanded containers - position at center-top using ELK container dimensions
  const labelNodes = [];
  containerNodes.forEach(containerNode => {
    if (!containerNode.data.isCollapsed) {
      const labelText = containerNode.data.label || '';
      const containerWidth = containerNode.style.width;
      
      // Calculate label width for centering
      const avgCharWidth = 6.5; // 11px bold font
      const horizontalPadding = 8; // 4px left + 4px right
      const borderWidth = 2; // 1px left + 1px right  
      const labelWidth = (labelText.length * avgCharWidth) + horizontalPadding + borderWidth;
      
      // Center horizontally within container, position at top
      const centerX = (containerWidth - labelWidth) / 2;
      
      labelNodes.push({
        id: `label-${containerNode.id}`,
        type: 'label',
        position: { 
          x: Math.max(10, centerX), // Center horizontally with minimum margin
          y: 10 // Position near top of container
        },
        data: { 
          label: containerNode.data.label 
        },
        parentNode: containerNode.id,
        extent: 'parent',
        draggable: false,
        selectable: false,
        connectable: false,
        focusable: false,
        deletable: false
      });
    }
  });

  // Use the edges that were already processed during ELK layout
  // Convert them back to the ReactFlow format
  const finalEdgesResult = validElkEdges.map(elkEdge => {
    const sourceId = elkEdge.sources[0];
    const targetId = elkEdge.targets[0];
    
    // Determine if this edge crosses between locations (network edge)
    let isNetworkEdge = false;
    
    // Check if source and target are in different locations
    // First check regular nodes
    let sourceNode = nodeMap.get(sourceId);
    let targetNode = nodeMap.get(targetId);
    
    // If not found in regular nodes, check container nodes
    if (!sourceNode) {
      sourceNode = containerNodes.find(c => c.id === sourceId);
    }
    if (!targetNode) {
      targetNode = containerNodes.find(c => c.id === targetId);
    }
    
    if (sourceNode && targetNode) {
      // Get location IDs for both nodes
      const sourceLocationId = sourceNode.data?.locationId || (sourceNode.id && sourceNode.id.startsWith('container_') ? parseInt(sourceNode.id.replace('container_', '')) : null);
      const targetLocationId = targetNode.data?.locationId || (targetNode.id && targetNode.id.startsWith('container_') ? parseInt(targetNode.id.replace('container_', '')) : null);
      
      // An edge is a network edge if:
      // 1. It connects nodes in different locations, OR
      // 2. Either endpoint is a network node type (regardless of location)
      const isDifferentLocations = sourceLocationId !== null && targetLocationId !== null && sourceLocationId !== targetLocationId;
      const hasNetworkNode = (sourceNode.data?.nodeType === 'Network') || (targetNode.data?.nodeType === 'Network');
      
      isNetworkEdge = isDifferentLocations || hasNetworkNode;
    }
    
    return {
      id: elkEdge.id,
      source: sourceId,
      target: targetId,
      type: 'bezier', // Use bezier curves for smooth edges
      style: { 
        strokeWidth: 2, 
        stroke: '#666666',
        strokeDasharray: isNetworkEdge ? '5,5' : undefined, // Dashed lines for network edges
      },
      markerEnd: { type: 'arrowclosed', width: 20, height: 20, color: '#666666' },
      animated: isNetworkEdge, // Animate network edges
    };
  });
  
  // Combine containers and other nodes, ensuring containers come first.
  const finalNodesResult = [...containerNodes, ...childAndOrphanNodes, ...labelNodes];
  
  if (finalNodesResult.length === 0) {
    console.error(`🚨 HIERARCHICAL LAYOUT RETURNING EMPTY NODES!`);
    console.error(`  Input: ${nodes.length} nodes, ${edges.length} edges`);
    console.error(`  Locations: ${locations.size} locations`);
  }
  
  return { nodes: finalNodesResult, edges: finalEdgesResult };
};
