/**
 * Layout Algorithms
 * 
 * Contains the hierarchical layout algorithm and helper functions
 * for positioning nodes using ELK.js
 */

import { elkLayouts } from './layoutConfigs.js';
import { generateLocationColor, generateLocationBorderColor } from './colorUtils.js';
import { ELK } from './externalLibraries.js';
import { generateHyperedges, routeEdgesForCollapsedContainers, createChildNodeMapping } from './hyperedgeUtils.js';

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
export const applyHierarchicalLayout = async (nodes, edges, layoutType, locations, currentPalette, collapsedContainers = {}, handleContainerToggle, isDraggedRef, precomputedHyperedges = []) => {
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

  // Calculate actual container dimensions before ELK layout
  const calculateContainerDimensions = (nodeIds, isCollapsed) => {
    if (isCollapsed) {
      return { width: 250, height: 80 };
    }
    
    // For expanded containers, estimate the space needed based on child nodes
    const childNodes = Array.from(nodeIds).map(nodeId => nodeMap.get(nodeId));
    const totalChildArea = childNodes.reduce((area, node) => {
      const width = parseFloat(node.style.width) || 100;
      const height = parseFloat(node.style.height) || 40;
      return area + (width * height);
    }, 0);
    
    // Estimate container dimensions based on total child area plus padding
    const padding = 50; // top + bottom + left + right padding
    const aspectRatio = 1.5; // prefer wider containers
    
    // Calculate dimensions that can contain all children with some spacing
    const estimatedHeight = Math.sqrt(totalChildArea / aspectRatio) + padding;
    const estimatedWidth = totalChildArea / estimatedHeight + padding;
    
    // Apply reasonable bounds
    return {
      width: Math.max(200, Math.min(estimatedWidth, 800)),
      height: Math.max(150, Math.min(estimatedHeight, 600))
    };
  };

  // Build the set of all node IDs that will exist in the ELK graph
  const elkChildren = [];
  
  // Add container nodes to ELK graph
  locationGroups.forEach(({ location, nodeIds }) => {
    const containerId = `container_${location.id}`;
    const isCollapsed = collapsedContainers[containerId];
    const containerDims = calculateContainerDimensions(nodeIds, isCollapsed);
    
    if (isCollapsed) {
      // If collapsed, use calculated dimensions as a single node
      elkChildren.push({
        id: containerId,
        width: containerDims.width,
        height: containerDims.height,
        // Mark as collapsed for later processing
        isCollapsed: true,
        originalNodeIds: Array.from(nodeIds)
      });
    } else {
      // If expanded, provide both child nodes AND the estimated container dimensions
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
        width: containerDims.width,  // Give ELK the estimated container size
        height: containerDims.height,
        children: childElkNodes,
        layoutOptions: {
          'elk.padding': '[top=40,left=25,bottom=25,right=25]', // Reduced padding
          'elk.spacing.nodeNode': 15, // Tighter internal spacing
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
  const elkEdgeMap = new Map(); // Use Map to deduplicate edges by ID
  
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
      const edgeId = `${sourceId}_to_${targetId}`;
      const newEdge = {
        id: edgeId,
        sources: [sourceId],
        targets: [targetId]
      };
      
      // Use Map to automatically deduplicate by edge ID
      elkEdgeMap.set(edgeId, newEdge);
    }
  });
  
  // Convert Map values back to array
  elkEdgeMap.forEach(edge => validElkEdges.push(edge));

  // Use precomputed hyperedges if available, otherwise generate them
  const hyperedges = precomputedHyperedges.length > 0 ? precomputedHyperedges : generateHyperedges(nodes, edges);
  
  // Convert hyperedges to ELK format and include them in ELK's edge set
  // This ensures ELK knows about all the connections that will be visible in ReactFlow
  const elkHyperedges = hyperedges.map(hyperedge => ({
    id: hyperedge.id,
    sources: hyperedge.sources,
    targets: hyperedge.targets
  }));
  
  // Combine regular edges with hyperedges for ELK
  const allElkEdges = [...validElkEdges, ...elkHyperedges];

  const elkGraph = {
    id: 'root',
    layoutOptions: {
      ...(elkLayouts[layoutType] || elkLayouts.mrtree),
      'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
    },
    children: elkChildren,
    edges: allElkEdges // Give ELK the complete edge picture
  };

  // DEBUG: Log what we're sending to ELK
  console.log('📤 SENDING TO ELK:');
  console.log('Layout type:', layoutType);
  console.log('Root layout options:', elkGraph.layoutOptions);
  console.log('Children summary:', elkChildren.map(child => ({
    id: child.id,
    width: child.width,
    height: child.height,
    isCollapsed: child.isCollapsed,
    childCount: child.children ? child.children.length : 0,
    layoutOptions: child.layoutOptions
  })));
  console.log('Regular edges:', validElkEdges.length);
  console.log('Hyperedges:', elkHyperedges.length);
  console.log('Total edges to ELK:', allElkEdges.length);

  // 3. Apply ELK layout
  const layoutedGraph = await ELK.layout(elkGraph);

  // DEBUG: Log what ELK returned
  console.log('📥 RECEIVED FROM ELK:');
  console.log('Root dimensions:', { width: layoutedGraph.width, height: layoutedGraph.height });
  console.log('Children after layout:', layoutedGraph.children.map(child => ({
    id: child.id,
    x: child.x,
    y: child.y,
    width: child.width,
    height: child.height,
    childCount: child.children ? child.children.length : 0
  })));

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

      // DEBUG: Log container processing
      console.log(`🏗️ Processing container ${elkNode.id}:`, {
        isCollapsed,
        elkPosition: { x: elkNode.x, y: elkNode.y },
        elkSize: { width: elkNode.width, height: elkNode.height },
        hasChildren: !!elkNode.children,
        childCount: elkNode.children ? elkNode.children.length : 0
      });

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

      // DEBUG: Log the React Flow container node we just created
      console.log(`➡️ Created React Flow container ${elkNode.id}:`, {
        position: { x: elkNode.x, y: elkNode.y },
        style: { width: containerStyle.width, height: containerStyle.height },
        isCollapsed,
        label: isCollapsed ? `${location.label || `Location ${location.id}`} (${elkNode.originalNodeIds?.length || 0} nodes)` : location.label || `Location ${location.id}`
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
        connectable: true, // FIXED: Make nodes connectable so edges can attach
      });
    } else {
      // It's an orphan (or its parent container was invalid and not created)
      childAndOrphanNodes.push({
        ...originalNode,
        position: {
          x: elkNode.x, // For orphans from invalid containers, position is absolute
          y: elkNode.y,
        },
        connectable: true, // FIXED: Make nodes connectable so edges can attach
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

  // Build set of all visible node IDs (includes both child nodes and container nodes)
  const visibleNodeIds = new Set();
  childAndOrphanNodes.forEach(node => visibleNodeIds.add(node.id));
  containerNodes.forEach(node => visibleNodeIds.add(node.id));
  
  // Process ALL original edges, not just ELK edges, to include internal container edges
  console.log('🔄 PROCESSING ORIGINAL EDGES:');
  console.log('Input edges:', edges.length);
  
  const finalEdgesResult = edges.map((originalEdge, index) => {
    let sourceId = originalEdge.source;
    let targetId = originalEdge.target;
    
    // Check if source node is in a collapsed container and redirect edge
    const sourceNode = nodeMap.get(originalEdge.source);
    if (sourceNode?.data?.locationId !== undefined) {
      const sourceContainerId = `container_${sourceNode.data.locationId}`;
      if (collapsedContainers[sourceContainerId]) {
        sourceId = sourceContainerId;
      }
    }
    
    // Check if target node is in a collapsed container and redirect edge
    const targetNode = nodeMap.get(originalEdge.target);
    if (targetNode?.data?.locationId !== undefined) {
      const targetContainerId = `container_${targetNode.data.locationId}`;
      if (collapsedContainers[targetContainerId]) {
        targetId = targetContainerId;
      }
    }
    
    // Only include edge if both endpoints are visible and different
    if (!visibleNodeIds.has(sourceId) || !visibleNodeIds.has(targetId) || sourceId === targetId) {
      // DEBUG: Log why this edge was filtered out
      if (index < 5) { // Only log first few to avoid spam
        console.log(`Edge ${index} filtered out: ${originalEdge.source} -> ${originalEdge.target}`, {
          sourceVisible: visibleNodeIds.has(sourceId),
          targetVisible: visibleNodeIds.has(targetId),
          sameSourceTarget: sourceId === targetId,
          redirectedSource: sourceId !== originalEdge.source ? sourceId : 'no redirect',
          redirectedTarget: targetId !== originalEdge.target ? targetId : 'no redirect'
        });
      }
      return null; // Filter out invalid edges
    }
    
    // Look up the actual visible nodes to determine their types/locations
    const visibleSourceNode = childAndOrphanNodes.find(n => n.id === sourceId) || containerNodes.find(c => c.id === sourceId);
    const visibleTargetNode = childAndOrphanNodes.find(n => n.id === targetId) || containerNodes.find(c => c.id === targetId);
    
    // An edge is internal if both its source and target are child nodes within the same container.
    const isInternalEdge = 
      visibleSourceNode?.parentNode &&
      visibleTargetNode?.parentNode &&
      visibleSourceNode.parentNode === visibleTargetNode.parentNode;

    // Determine if this is a network edge based on node types or locations
    let isNetworkEdge = false;
    if (visibleSourceNode && visibleTargetNode) {
      // Get location IDs for both nodes
      const sourceLocationId = visibleSourceNode.data?.locationId || (visibleSourceNode.id && visibleSourceNode.id.startsWith('container_') ? parseInt(visibleSourceNode.id.replace('container_', '')) : null);
      const targetLocationId = visibleTargetNode.data?.locationId || (visibleTargetNode.id && visibleTargetNode.id.startsWith('container_') ? parseInt(visibleTargetNode.id.replace('container_', '')) : null);
      
      // An edge is a network edge if:
      // 1. It connects nodes in different locations, OR
      // 2. Either endpoint is a network node type (regardless of location)
      const isDifferentLocations = sourceLocationId !== null && targetLocationId !== null && sourceLocationId !== targetLocationId;
      const hasNetworkNode = (visibleSourceNode.data?.nodeType === 'Network') || (visibleTargetNode.data?.nodeType === 'Network');
      
      isNetworkEdge = isDifferentLocations || hasNetworkNode;
    }
    
    const resultEdge = {
      id: `${sourceId}_to_${targetId}`, // Use consistent edge ID format
      source: sourceId,
      target: targetId,
      type: isInternalEdge ? 'custom' : 'bezier', // Use custom type for internal edges
      style: { 
        strokeWidth: 2, 
        stroke: '#666666',
        strokeDasharray: isNetworkEdge ? '5,5' : undefined, // Dashed lines for network edges
      },
      markerEnd: { type: 'arrowclosed', width: 20, height: 20, color: '#666666' },
      animated: isNetworkEdge, // Animate network edges
    };
    
    // DEBUG: Log first few internal edges being created
    if (index < 5 && !sourceId.startsWith('container_') && !targetId.startsWith('container_')) {
      console.log(`Creating internal edge ${index}:`, {
        original: `${originalEdge.source} -> ${originalEdge.target}`,
        final: `${sourceId} -> ${targetId}`,
        id: resultEdge.id,
        isNetworkEdge
      });
    }
    
    return resultEdge;
  }).filter(edge => edge !== null); // Remove null entries
  
  console.log('Processed edges result:', finalEdgesResult.length);

  // Process hyperedges for ReactFlow format (container-to-container edges)
  // Only include hyperedges if there are actually collapsed containers
  const hasCollapsedContainers = Object.values(collapsedContainers).some(Boolean);
  const hyperedgeResults = hasCollapsedContainers ? hyperedges.map(hyperedge => ({
    id: hyperedge.id,
    source: hyperedge.sources[0],
    target: hyperedge.targets[0],
    type: 'bezier',
    style: { 
      strokeWidth: 3, // Thicker for hyperedges
      stroke: '#880088', // Purple for container-to-container connections
      strokeDasharray: '8,4', // Distinctive dashed pattern
    },
    markerEnd: { type: 'arrowclosed', width: 20, height: 20, color: '#880088' },
    animated: true, // Always animate hyperedges
    data: { isHyperedge: true } // Mark as hyperedge for easy identification
  })) : [];
  
  // Combine containers and other nodes, ensuring containers come first.
  const finalNodesResult = [...containerNodes, ...childAndOrphanNodes, ...labelNodes];
  
  // DEBUG: Log final result summary
  console.log('🎯 FINAL LAYOUT RESULT:');
  console.log('Total nodes:', finalNodesResult.length);
  console.log('Container nodes:', containerNodes.map(node => ({
    id: node.id,
    position: node.position,
    size: { width: node.style.width, height: node.style.height },
    isCollapsed: node.data.isCollapsed
  })));
  console.log('Child nodes:', childAndOrphanNodes.length);
  console.log('Total edges:', finalEdgesResult.length);
  
  if (finalNodesResult.length === 0) {
    console.error(`🚨 HIERARCHICAL LAYOUT RETURNING EMPTY NODES!`);
    console.error(`  Input: ${nodes.length} nodes, ${edges.length} edges`);
    console.error(`  Locations: ${locations.size} locations`);
  }
  
  // Only include precomputed hyperedges when NO containers are collapsed
  // When containers are collapsed, edge routing will create the appropriate container-to-container edges
  const finalEdges = hasCollapsedContainers ? finalEdgesResult : [...finalEdgesResult, ...hyperedgeResults];
  
  // CRITICAL: Deduplicate final edges to prevent React warnings and rendering issues
  console.log('🔄 DEDUPLICATION PROCESS:');
  console.log('Before deduplication - finalEdges:', finalEdges.length);
  
  const finalEdgeMap = new Map();
  const duplicatesFound = [];
  
  finalEdges.forEach((edge, index) => {
    if (edge && edge.id) {
      if (finalEdgeMap.has(edge.id)) {
        duplicatesFound.push({
          id: edge.id,
          firstIndex: Array.from(finalEdgeMap.keys()).indexOf(edge.id),
          duplicateIndex: index
        });
      }
      finalEdgeMap.set(edge.id, edge);
    } else {
      console.warn('Edge without ID found at index', index, edge);
    }
  });
  
  if (duplicatesFound.length > 0) {
    console.log('Duplicates found during deduplication:', duplicatesFound.slice(0, 5));
  }
  
  const deduplicatedFinalEdges = Array.from(finalEdgeMap.values());
  console.log('After deduplication - deduplicatedFinalEdges:', deduplicatedFinalEdges.length);
  
  // Check how many internal edges survived deduplication
  const survivingInternalEdges = deduplicatedFinalEdges.filter(edge => 
    !edge.source.startsWith('container_') && !edge.target.startsWith('container_')
  );
  console.log('Internal edges surviving deduplication:', survivingInternalEdges.length);
  
  // DEBUG: Log edge composition
  console.log('🔗 EDGE COMPOSITION:');
  console.log('Original edges:', edges.length);
  console.log('finalEdgesResult:', finalEdgesResult.length, 'edges');
  console.log('hasCollapsedContainers:', hasCollapsedContainers);
  console.log('hyperedgeResults:', hyperedgeResults.length, 'edges');
  console.log('Using finalEdges:', hasCollapsedContainers ? 'finalEdgesResult only' : 'finalEdgesResult + hyperedgeResults');
  console.log('Before deduplication:', finalEdges.length);
  console.log('After deduplication:', deduplicatedFinalEdges.length);
  
  // DEBUG: Show sample internal edges with more detail
  const internalEdges = finalEdgesResult.filter(edge => {
    const sourceIsContainer = edge.source.startsWith('container_');
    const targetIsContainer = edge.target.startsWith('container_');
    return !sourceIsContainer && !targetIsContainer; // Both are regular nodes
  });
  console.log('Internal container edges:', internalEdges.length);
  if (internalEdges.length > 0) {
    console.log('Sample internal edges:', internalEdges.slice(0, 5).map(e => `${e.source} -> ${e.target}`));
    console.log('Sample internal edge objects:', internalEdges.slice(0, 2));
    
    // Check if the source and target nodes exist in the visible nodes
    const firstInternalEdge = internalEdges[0];
    if (firstInternalEdge) {
      const sourceNode = finalNodesResult.find(n => n.id === firstInternalEdge.source);
      const targetNode = finalNodesResult.find(n => n.id === firstInternalEdge.target);
      console.log('First internal edge source node:', sourceNode ? 'FOUND' : 'NOT FOUND');
      console.log('First internal edge target node:', targetNode ? 'FOUND' : 'NOT FOUND');
      if (sourceNode && targetNode) {
        console.log('Source connectable:', sourceNode.connectable);
        console.log('Target connectable:', targetNode.connectable);
        console.log('Source parentNode:', sourceNode.parentNode);
        console.log('Target parentNode:', targetNode.parentNode);
      }
    }
  }
  
  // DEBUG: Comprehensive edge analysis
  console.log('🔍 DETAILED EDGE ANALYSIS:');
  console.log('finalEdgesResult breakdown:');
  const edgeCategories = {
    containerToContainer: finalEdgesResult.filter(e => e.source.startsWith('container_') && e.target.startsWith('container_')),
    containerToNode: finalEdgesResult.filter(e => e.source.startsWith('container_') && !e.target.startsWith('container_')),
    nodeToContainer: finalEdgesResult.filter(e => !e.source.startsWith('container_') && e.target.startsWith('container_')),
    nodeToNode: finalEdgesResult.filter(e => !e.source.startsWith('container_') && !e.target.startsWith('container_'))
  };
  
  console.log('Container->Container edges:', edgeCategories.containerToContainer.length);
  console.log('Container->Node edges:', edgeCategories.containerToNode.length);
  console.log('Node->Container edges:', edgeCategories.nodeToContainer.length);
  console.log('Node->Node edges (internal):', edgeCategories.nodeToNode.length);
  
  // Show first few of each category
  Object.entries(edgeCategories).forEach(([category, edges]) => {
    if (edges.length > 0) {
      console.log(`${category} sample:`, edges.slice(0, 2).map(e => `${e.source} -> ${e.target}`));
    }
  });
  
  // DEBUG: Check deduplicatedFinalEdges content
  console.log('🔍 FINAL EDGES PASSED TO REACTFLOW:');
  console.log('Total deduplicatedFinalEdges:', deduplicatedFinalEdges.length);
  
  const finalEdgeCategories = {
    containerToContainer: deduplicatedFinalEdges.filter(e => e.source.startsWith('container_') && e.target.startsWith('container_')),
    containerToNode: deduplicatedFinalEdges.filter(e => e.source.startsWith('container_') && !e.target.startsWith('container_')),
    nodeToContainer: deduplicatedFinalEdges.filter(e => !e.source.startsWith('container_') && e.target.startsWith('container_')),
    nodeToNode: deduplicatedFinalEdges.filter(e => !e.source.startsWith('container_') && !e.target.startsWith('container_'))
  };
  
  console.log('Final Container->Container edges:', finalEdgeCategories.containerToContainer.length);
  console.log('Final Container->Node edges:', finalEdgeCategories.containerToNode.length);
  console.log('Final Node->Container edges:', finalEdgeCategories.nodeToContainer.length);
  console.log('Final Node->Node edges (internal):', finalEdgeCategories.nodeToNode.length);
  
  // Show actual internal edge objects being passed to ReactFlow
  if (finalEdgeCategories.nodeToNode.length > 0) {
    console.log('First 3 internal edges passed to ReactFlow:');
    finalEdgeCategories.nodeToNode.slice(0, 3).forEach((edge, index) => {
      console.log(`Edge ${index + 1}:`, {
        id: edge.id,
        source: edge.source,
        target: edge.target,
        type: edge.type,
        style: edge.style,
        markerEnd: edge.markerEnd,
        animated: edge.animated
      });
    });
  } else {
    console.error('🚨 NO INTERNAL EDGES IN FINAL RESULT!');
  }
  
  // DEBUG: Check for duplicate edge IDs in final result
  const finalEdgeIds = deduplicatedFinalEdges.map(edge => edge.id);
  const finalDuplicateIds = finalEdgeIds.filter((id, index) => finalEdgeIds.indexOf(id) !== index);
  if (finalDuplicateIds.length > 0) {
    console.error('🚨 DUPLICATE EDGE IDs STILL FOUND AFTER DEDUPLICATION:', finalDuplicateIds);
  } else {
    console.log('✅ No duplicate edge IDs found after deduplication');
  }
  
  return { 
    nodes: finalNodesResult, 
    edges: deduplicatedFinalEdges,
    hyperedges: hyperedges // Return hyperedges separately for reuse
  };
};
