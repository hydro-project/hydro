import { ELK } from './externalLibraries';
import { generateLocationColor, generateLocationBorderColor } from './colorUtils';
import { generateHyperedges } from './hyperedgeUtils';

const elkLayouts = {
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

// Cache for expanded container dimensions - calculated once on initialization
let expandedContainerDimensionsCache = new Map();

// Initialize expanded container dimensions cache
const initializeExpandedContainerDimensions = async (nodes, locations, nodeMap) => {
  console.log('🔄 CALCULATING EXPANDED CONTAINER DIMENSIONS...');
  
  const locationGroups = new Map();
  
  // Group nodes by location
  nodes.forEach(node => {
    const locationId = node.data?.locationId;
    if (locationId !== undefined && locationId !== null) {
      if (!locationGroups.has(locationId)) {
        const location = locations.get(locationId);
        if (location) {
          locationGroups.set(locationId, { location, nodeIds: new Set() });
        }
      }
      
      const group = locationGroups.get(locationId);
      if (group) {
        group.nodeIds.add(node.id);
      }
    }
  });
  
  // Calculate dimensions for each location
  for (const [locationId, { location, nodeIds }] of locationGroups) {
    const childNodes = Array.from(nodeIds)
      .map(nodeId => nodeMap.get(nodeId))
      .filter(node => node);
      
    if (childNodes.length === 0) {
      expandedContainerDimensionsCache.set(locationId, { width: 200, height: 150 });
      continue;
    }
    
    // Create ELK layout for this container's children
    const childElkGraph = {
      id: `temp-container-${locationId}`,
      layoutOptions: {
        'elk.algorithm': 'layered',
        'elk.direction': 'DOWN',
        'elk.spacing.nodeNode': 20,
        'elk.padding': '[top=30,left=20,bottom=20,right=20]'
      },
      children: childNodes.map(node => ({
        id: node.id,
        width: node.measured?.width || parseFloat(node.style?.width) || 200,
        height: node.measured?.height || parseFloat(node.style?.height) || 60,
      }))
    };
    
    try {
      const elkResult = await ELK.layout(childElkGraph);
      const dimensions = {
        width: (elkResult.width || 300) + 40,
        height: (elkResult.height || 200) + 60
      };
      expandedContainerDimensionsCache.set(locationId, dimensions);
      console.log(`📋 Cached dimensions for location ${locationId}:`, dimensions);
    } catch (error) {
      console.error(`ELK sizing failed for location ${locationId}:`, error);
      expandedContainerDimensionsCache.set(locationId, { width: 300, height: 200 });
    }
  }
  
  console.log('✅ CONTAINER DIMENSIONS CACHE INITIALIZED');
};

export const applyHierarchicalLayout = async (nodes, edges, layoutType, locations, currentPalette, collapsedContainers = {}, handleContainerToggle, isDraggedRef, precomputedHyperedges = []) => {
  if (!ELK) {
    console.log(`🚨 HIERARCHICAL LAYOUT ABORT: ELK not available`);
    return { nodes, edges };
  }

  console.log('🎯 UNIFIED ELK LAYOUT START - ReactFlow v12 + ELK');
  console.log('🔍 COLLAPSED CONTAINERS STATE:', JSON.stringify(collapsedContainers, null, 2));
  
  const nodeMap = new Map(nodes.map(n => [n.id, n]));
  const locationGroups = new Map();
  const orphanNodeIds = new Set(nodes.map(n => n.id));

  // 1. Group nodes by location
  nodes.forEach(node => {
    const locationId = node.data?.locationId;
    if (locationId !== undefined && locationId !== null) {
      if (!locationGroups.has(locationId)) {
        const location = locations.get(locationId);
        if (location) {
          locationGroups.set(locationId, { location, nodeIds: new Set() });
        }
      }
      
      const group = locationGroups.get(locationId);
      if (group) {
        group.nodeIds.add(node.id);
        orphanNodeIds.delete(node.id);
      }
    }
  });

  // 2. Build ELK graph structure - let ELK calculate everything
  const elkChildren = [];
  
  // Process each location group
  for (const [locationId, { location, nodeIds }] of locationGroups) {
    const containerId = `container_${location.id}`;
    const isCollapsed = collapsedContainers[containerId];
    
    console.log(`🔍 CONTAINER SETUP - ${containerId}: isCollapsed = ${isCollapsed}, nodeCount = ${nodeIds.size}`);
    
    if (isCollapsed) {
      // Collapsed container: single ELK node with fixed size
      elkChildren.push({
        id: containerId,
        width: 250,
        height: 80,
        isCollapsed: true,
        originalNodeIds: Array.from(nodeIds)
      });
    } else {
      // Expanded container: include child nodes and let ELK size the container
      const childElkNodes = Array.from(nodeIds).map(nodeId => {
        const node = nodeMap.get(nodeId);
        const nodeWidth = node.measured?.width || parseFloat(node.style?.width) || 200;
        const nodeHeight = node.measured?.height || parseFloat(node.style?.height) || 60;
        
        console.log(`📋 ELK child node ${nodeId}: ${nodeWidth} x ${nodeHeight}`);
        
        return {
          id: node.id,
          width: nodeWidth,
          height: nodeHeight,
        };
      });

      console.log(`🏗️ Creating expanded container ${containerId}: ${childElkNodes.length} children`);
      
      elkChildren.push({
        id: containerId,
        children: childElkNodes,
        layoutOptions: {
          ...elkLayouts[layoutType],
          'elk.padding': '[top=35,left=20,bottom=20,right=20]',
          'elk.spacing.nodeNode': 15,
          'elk.nodeSize.constraints': 'NODE_LABELS',
          'elk.nodeSize.options': 'MINIMUM_SIZE',
          'elk.contentAlignment': 'V_TOP H_LEFT',
        }
      });
    }
  }

  // Add orphan nodes to ELK graph
  orphanNodeIds.forEach(nodeId => {
    const node = nodeMap.get(nodeId);
    elkChildren.push({ 
      id: node.id, 
      width: node.measured?.width || parseFloat(node.style?.width) || 200,
      height: node.measured?.height || parseFloat(node.style?.height) || 60
    });
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
  const elkEdgeMap = new Map();
  
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
      
      elkEdgeMap.set(edgeId, newEdge);
    }
  });
  
  // Convert Map values back to array
  elkEdgeMap.forEach(edge => validElkEdges.push(edge));

  // Use precomputed hyperedges if available, otherwise generate them
  const hyperedges = precomputedHyperedges.length > 0 ? precomputedHyperedges : generateHyperedges(nodes, edges);
  
  // Convert hyperedges to ELK format
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
    edges: allElkEdges
  };

  console.log('📤 SENDING TO ELK (UNIFIED SINGLE CALL):');
  console.log('Layout type:', layoutType);
  console.log('Children summary:', elkChildren.map(child => ({
    id: child.id,
    hasFixedSize: !!child.width,
    childrenCount: child.children?.length || 0,
    isCollapsed: !!child.isCollapsed
  })));
  console.log('Total edges to ELK:', allElkEdges.length);

  // 3. Apply ELK layout - SINGLE UNIFIED CALL
  const layoutedGraph = await ELK.layout(elkGraph);

  console.log('📥 RECEIVED FROM ELK (UNIFIED RESULT):');
  console.log('Root dimensions:', { width: layoutedGraph.width, height: layoutedGraph.height });
  console.log('Children after layout:', layoutedGraph.children.map(child => ({
    id: child.id,
    x: child.x,
    y: child.y,
    width: child.width,
    height: child.height,
    childCount: child.children ? child.children.length : 0,
    sampleChildPositions: child.children ? child.children.slice(0, 3).map(c => ({ id: c.id, x: c.x, y: c.y })) : []
  })));

  // 4. Process the layout result - USE ONLY ELK RESULTS
  const containerNodes = [];
  const childAndOrphanNodes = [];
  const layoutedNodeMap = new Map();

  // Build map of all layouted nodes
  layoutedGraph.children.forEach(elkNode => {
    layoutedNodeMap.set(elkNode.id, elkNode);
    if (elkNode.children) {
      elkNode.children.forEach(child => {
        layoutedNodeMap.set(child.id, child);
      });
    }
  });

  // Create container nodes - USE ELK SIZES ONLY
  layoutedGraph.children.forEach(elkNode => {
    if (elkNode.children || elkNode.isCollapsed) { // It's a container
      const locationId = parseInt(elkNode.id.replace('container_', ''), 10);
      const location = locations.get(locationId);
      const isCollapsed = !!elkNode.isCollapsed; // Use ELK's determination

      console.log(`🏗️ Processing container ${elkNode.id}:`, {
        isCollapsed,
        elkPosition: { x: elkNode.x, y: elkNode.y },
        elkSize: { width: elkNode.width, height: elkNode.height },
        childCount: elkNode.children ? elkNode.children.length : 0,
        sizeSource: 'ELK_RESULT_ONLY'
      });
      
      if (!location) {
        console.warn(`Could not find location metadata for container ${elkNode.id}`);
        return;
      }

      // Create container style - USE ELK DIMENSIONS DIRECTLY
      const containerStyle = {
        width: elkNode.width,   // ONLY from ELK result
        height: elkNode.height, // ONLY from ELK result
        backgroundColor: generateLocationColor(location.id, locations.size, currentPalette),
        borderRadius: '8px',
        zIndex: 1,
      };

      // Add visual indication for collapsed state
      if (isCollapsed) {
        containerStyle.opacity = 0.8;
        containerStyle.border = `2px dashed ${generateLocationBorderColor(location.id, locations.size, currentPalette)}`;
        containerStyle.backgroundColor = generateLocationColor(location.id, locations.size, currentPalette).replace('40', '60');
        
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
        type: 'container',
        position: { x: elkNode.x, y: elkNode.y },
        style: containerStyle,
        data: {
          label: isCollapsed ? `${location.label || `Location ${location.id}`} (${elkNode.originalNodeIds?.length || 0} nodes)` : location.label || `Location ${location.id}`,
          isContainer: true,
          locationId: location.id,
          isCollapsed: isCollapsed,
          nodeCount: elkNode.originalNodeIds?.length || 0,
          onContainerToggle: handleContainerToggle,
          isDraggedRef: isDraggedRef,
        },
        draggable: true,
        selectable: true,
        connectable: true,
      });

      console.log(`➡️ Created React Flow container ${elkNode.id}:`, {
        position: { x: elkNode.x, y: elkNode.y },
        size: { width: elkNode.width, height: elkNode.height },
        isCollapsed,
        sizeSource: 'ELK_RESULT_ONLY'
      });
    }
  });

  // Create child and orphan nodes
  nodes.forEach(originalNode => {
    const locationId = originalNode.data?.locationId;
    const isChild = locationId !== undefined && locationId !== null;
    const containerId = isChild ? `container_${locationId}` : null;
    
    // Skip if node is in a collapsed container
    if (isChild && containerId) {
      const elkContainer = layoutedGraph.children.find(c => c.id === containerId);
      if (elkContainer && elkContainer.isCollapsed) {
        return; // Skip nodes in collapsed containers
      }
    }

    const elkNode = layoutedNodeMap.get(originalNode.id);
    if (!elkNode) {
      console.warn(`Node ${originalNode.id} not found in ELK layout result.`);
      return;
    }

    if (isChild && containerId && containerNodes.some(c => c.id === containerId)) {
      // Child node - Use ELK coordinates directly, let ReactFlow handle viewport transforms
      
      // Comprehensive ReactFlow viewport debugging
      let viewportInfo = { scale: 1.0, translateX: 0, translateY: 0, transform: 'none' };
      try {
        const reactFlowViewport = document.querySelector('.react-flow__viewport');
        if (reactFlowViewport) {
          const transform = reactFlowViewport.style.transform;
          viewportInfo.transform = transform;
          
          // Extract transform components
          const transformMatch = transform.match(/translate\(([^,]+),([^)]+)\) scale\(([^)]+)\)/);
          if (transformMatch) {
            viewportInfo.translateX = parseFloat(transformMatch[1]);
            viewportInfo.translateY = parseFloat(transformMatch[2]);
            viewportInfo.scale = parseFloat(transformMatch[3]);
          }
        }
      } catch (error) {
        console.warn('Could not analyze ReactFlow viewport:', error);
      }
      
      // Apply inverse scale factor to compensate for ReactFlow viewport scaling
      const inverseScale = viewportInfo.scale !== 0 ? (1 / viewportInfo.scale) : 1;
      const finalX = elkNode.x * inverseScale;
      const finalY = elkNode.y * inverseScale;
      
      console.log(`🔍 CHILD COORDINATE - ${originalNode.id}:`, {
        containerId,
        elkOriginalCoords: { x: elkNode.x, y: elkNode.y },
        inverseScale: inverseScale.toFixed(4),
        finalCoords: { x: finalX, y: finalY },
        viewportInfo,
        coordinateSource: 'ELK_HIERARCHICAL_INVERSE_SCALED',
        note: 'Applying inverse scale to compensate for ReactFlow viewport scaling'
      });
      
      childAndOrphanNodes.push({
        ...originalNode,
        position: { x: finalX, y: finalY },
        parentNode: containerId,
        extent: 'parent',
        style: { ...originalNode.style, zIndex: 10 },
        connectable: true,
      });
    } else {
      // Orphan node - use absolute coordinates
      childAndOrphanNodes.push({
        ...originalNode,
        position: { x: elkNode.x, y: elkNode.y },
        connectable: true,
      });
    }
  });

  // Process edges with simplified logic
  const visibleNodeIds = new Set();
  childAndOrphanNodes.forEach(node => visibleNodeIds.add(node.id));
  containerNodes.forEach(node => visibleNodeIds.add(node.id));
  
  const finalEdgesResult = edges.map(originalEdge => {
    let sourceId = originalEdge.source;
    let targetId = originalEdge.target;
    
    // Redirect edges for collapsed containers
    const sourceNode = nodeMap.get(originalEdge.source);
    if (sourceNode?.data?.locationId !== undefined) {
      const sourceContainerId = `container_${sourceNode.data.locationId}`;
      if (collapsedContainers[sourceContainerId]) {
        sourceId = sourceContainerId;
      }
    }
    
    const targetNode = nodeMap.get(originalEdge.target);
    if (targetNode?.data?.locationId !== undefined) {
      const targetContainerId = `container_${targetNode.data.locationId}`;
      if (collapsedContainers[targetContainerId]) {
        targetId = targetContainerId;
      }
    }
    
    // Filter invalid edges
    if (!visibleNodeIds.has(sourceId) || !visibleNodeIds.has(targetId) || sourceId === targetId) {
      return null;
    }
    
    // Determine edge styling
    const visibleSourceNode = childAndOrphanNodes.find(n => n.id === sourceId) || containerNodes.find(c => c.id === sourceId);
    const visibleTargetNode = childAndOrphanNodes.find(n => n.id === targetId) || containerNodes.find(c => c.id === targetId);
    
    let isNetworkEdge = false;
    if (visibleSourceNode && visibleTargetNode) {
      const sourceLocationId = visibleSourceNode.data?.locationId || (visibleSourceNode.id && visibleSourceNode.id.startsWith('container_') ? parseInt(visibleSourceNode.id.replace('container_', '')) : null);
      const targetLocationId = visibleTargetNode.data?.locationId || (visibleTargetNode.id && visibleTargetNode.id.startsWith('container_') ? parseInt(visibleTargetNode.id.replace('container_', '')) : null);
      
      const isDifferentLocations = sourceLocationId !== null && targetLocationId !== null && sourceLocationId !== targetLocationId;
      const hasNetworkNode = (visibleSourceNode.data?.nodeType === 'Network') || (visibleTargetNode.data?.nodeType === 'Network');
      
      isNetworkEdge = isDifferentLocations || hasNetworkNode;
    }
    
    return {
      id: `${sourceId}_to_${targetId}`,
      source: sourceId,
      target: targetId,
      type: 'smoothstep',
      style: { 
        strokeWidth: 2, 
        stroke: '#666666',
        strokeDasharray: isNetworkEdge ? '5,5' : undefined,
      },
      markerEnd: { type: 'arrowclosed', width: 20, height: 20, color: '#666666' },
      animated: isNetworkEdge,
    };
  }).filter(edge => edge !== null);
  
  // Process hyperedges
  const hasCollapsedContainers = Object.values(collapsedContainers).some(Boolean);
  const hyperedgeResults = hasCollapsedContainers ? hyperedges.map(hyperedge => ({
    id: hyperedge.id,
    source: hyperedge.sources[0],
    target: hyperedge.targets[0],
    type: 'smoothstep',
    style: { 
      strokeWidth: 3,
      stroke: '#880088',
      strokeDasharray: '8,4',
    },
    markerEnd: { type: 'arrowclosed', width: 20, height: 20, color: '#880088' },
    animated: true,
    data: { isHyperedge: true }
  })) : [];

  // Combine containers and other nodes
  const finalNodesResult = [...containerNodes, ...childAndOrphanNodes];
  
  console.log('🎯 UNIFIED LAYOUT RESULT:');
  console.log('Total nodes:', finalNodesResult.length);
  console.log('Container nodes:', containerNodes.length);
  console.log('Child nodes:', childAndOrphanNodes.length);
  console.log('Total edges:', finalEdgesResult.length);

  // COORDINATE INVESTIGATION - Streamlined check
  console.log('🔍 CHECKING FOR COORDINATE DISCREPANCIES...');
  
  let overallLayoutBroken = false;
  
  containerNodes.forEach(container => {
    const childNodesInContainer = childAndOrphanNodes.filter(child => child.parentNode === container.id);
    if (childNodesInContainer.length > 0) {
      console.log(`\n📦 CONTAINER ${container.id}:`);
      
      // Step 1: Analyze actual child positions vs container bounds
      const childPositions = childNodesInContainer.map(child => child.position);
      const minX = Math.min(...childPositions.map(pos => pos.x));
      const maxX = Math.max(...childPositions.map(pos => pos.x));
      const minY = Math.min(...childPositions.map(pos => pos.y));
      const maxY = Math.max(...childPositions.map(pos => pos.y));
      
      // Step 2: Calculate required container size for child content
      const requiredWidth = Math.max(maxX + 220, 300); // Add node width estimate
      const requiredHeight = Math.max(maxY + 80, 200);  // Add node height estimate
      
      console.log(`  📍 CHILD NODES ANALYSIS:`);
      console.log(`    Child positions: ${childNodesInContainer.length} nodes`);
      console.log(`    X range: ${minX} to ${maxX} (span: ${maxX - minX})`);
      console.log(`    Y range: ${minY} to ${maxY} (span: ${maxY - minY})`);
      console.log(`    Required container size: ${requiredWidth} x ${requiredHeight}`);
      
      console.log(`  📦 CONTAINER ACTUAL:`);
      console.log(`    Position: (${container.position.x}, ${container.position.y})`);
      console.log(`    Size: ${container.style.width} x ${container.style.height}`);
      console.log(`    Container coordinate space: 0 to ${container.style.width} x 0 to ${container.style.height}`);
      
      // Step 3: RIGOROUS boundary violation checks
      const violatingNodes = childNodesInContainer.filter(child => {
        const nodeRight = child.position.x + 220; // Estimate node width
        const nodeBottom = child.position.y + 60;  // Estimate node height
        return child.position.x < 0 || 
               child.position.y < 0 || 
               nodeRight > container.style.width || 
               nodeBottom > container.style.height;
      });
      
      // Step 4: Size adequacy check
      const containerTooSmall = container.style.width < requiredWidth || container.style.height < requiredHeight;
      
      console.log(`  🚨 VISUAL LAYOUT PROBLEMS:`);
      if (violatingNodes.length > 0) {
        overallLayoutBroken = true;
        console.log(`    ❌ BOUNDARY VIOLATIONS: ${violatingNodes.length} nodes outside container`);
        violatingNodes.slice(0, 5).forEach(node => {
          const nodeRight = node.position.x + 220;
          const nodeBottom = node.position.y + 60;
          console.log(`      Node ${node.id}: (${node.position.x}, ${node.position.y}) extends to (${nodeRight}, ${nodeBottom})`);
          if (node.position.x < 0) console.log(`        ⚠️  X position ${node.position.x} < 0`);
          if (node.position.y < 0) console.log(`        ⚠️  Y position ${node.position.y} < 0`);
          if (nodeRight > container.style.width) console.log(`        ⚠️  Right edge ${nodeRight} > container width ${container.style.width}`);
          if (nodeBottom > container.style.height) console.log(`        ⚠️  Bottom edge ${nodeBottom} > container height ${container.style.height}`);
        });
      }
      
      if (containerTooSmall) {
        overallLayoutBroken = true;
        console.log(`    ❌ CONTAINER TOO SMALL:`);
        console.log(`      Current: ${container.style.width} x ${container.style.height}`);
        console.log(`      Required: ${requiredWidth} x ${requiredHeight}`);
        console.log(`      Deficit: ${requiredWidth - container.style.width} x ${requiredHeight - container.style.height}`);
      }
      
      if (violatingNodes.length === 0 && !containerTooSmall) {
        console.log(`    ✅ Container layout appears correct`);
      }
      
      // Step 5: ELK coordinate system analysis
      console.log(`  � COORDINATE SYSTEM DIAGNOSIS:`);
      console.log(`    ELK coordinate source: ELK_HIERARCHICAL_DIRECT (no conversion)`);
      console.log(`    ELK hierarchyHandling: INCLUDE_CHILDREN`);
      console.log(`    Expected: ELK provides relative coordinates within parent container`);
      
      if (violatingNodes.length > 0 || containerTooSmall) {
        console.log(`    💡 LIKELY ISSUES:`);
        console.log(`      1. ELK not respecting container padding/constraints`);
        console.log(`      2. ELK hierarchical layout incorrectly configured`);
        console.log(`      3. Child node size estimates wrong in ELK input`);
        console.log(`      4. Container size calculation broken`);
      }
    }
  });
  
  console.log(`\n🎯 OVERALL LAYOUT STATUS:`);
  console.log(`⏳ Checking DOM positions in 1 second to compare with JavaScript coordinates...`);
  
  // DOM REALITY CHECK - Compare actual rendered positions vs JavaScript coordinates
  setTimeout(() => {
    console.log('\n🌐 DOM REALITY CHECK - CHECKING ACTUAL RENDERED POSITIONS:');
    let realLayoutBroken = false;
    
    // First, analyze ReactFlow's coordinate system behavior
    console.log('\n🔧 REACTFLOW COORDINATE SYSTEM ANALYSIS:');
    const reactFlowInstance = document.querySelector('.react-flow');
    const reactFlowViewport = document.querySelector('.react-flow__viewport');
    
    if (reactFlowInstance && reactFlowViewport) {
      const instanceRect = reactFlowInstance.getBoundingClientRect();
      const viewportTransform = reactFlowViewport.style.transform;
      
      console.log(`📏 ReactFlow Instance: ${instanceRect.width.toFixed(1)} x ${instanceRect.height.toFixed(1)} at (${instanceRect.left.toFixed(1)}, ${instanceRect.top.toFixed(1)})`);
      console.log(`🔄 Viewport Transform: ${viewportTransform}`);
      
      // Parse transform
      const transformMatch = viewportTransform.match(/translate\(([^,]+),([^)]+)\) scale\(([^)]+)\)/);
      if (transformMatch) {
        const translateX = parseFloat(transformMatch[1]);
        const translateY = parseFloat(transformMatch[2]);
        const scale = parseFloat(transformMatch[3]);
        
        console.log(`📐 Transform Components:`);
        console.log(`  Translation: (${translateX.toFixed(1)}, ${translateY.toFixed(1)})`);
        console.log(`  Scale: ${scale.toFixed(6)}`);
        console.log(`  Scale percentage: ${(scale * 100).toFixed(2)}%`);
        
        if (scale !== 1.0) {
          console.log(`⚠️  NON-UNITY SCALE DETECTED: This affects all coordinate calculations!`);
        }
      }
    }
    
    containerNodes.forEach(container => {
      const childNodesInContainer = childAndOrphanNodes.filter(child => child.parentNode === container.id);
      if (childNodesInContainer.length > 0) {
        console.log(`\n📦 DOM CONTAINER ${container.id}:`);
        
        // Find the actual container DOM element
        const containerElement = document.querySelector(`[data-id="${container.id}"]`);
        if (containerElement) {
          const containerRect = containerElement.getBoundingClientRect();
          
          // Calculate container position relative to ReactFlow instance
          let containerRelativeToInstance = { x: 0, y: 0 };
          if (reactFlowInstance) {
            const instanceRect = reactFlowInstance.getBoundingClientRect();
            containerRelativeToInstance.x = containerRect.left - instanceRect.left;
            containerRelativeToInstance.y = containerRect.top - instanceRect.top;
          }
          
          console.log(`  📦 DOM CONTAINER BOUNDS:`);
          console.log(`    Screen position: (${containerRect.left.toFixed(1)}, ${containerRect.top.toFixed(1)})`);
          console.log(`    Screen size: ${containerRect.width.toFixed(1)} x ${containerRect.height.toFixed(1)}`);
          console.log(`    Relative to ReactFlow: (${containerRelativeToInstance.x.toFixed(1)}, ${containerRelativeToInstance.y.toFixed(1)})`);
          console.log(`    JS position: (${container.position.x}, ${container.position.y})`);
          console.log(`    JS size: ${container.style.width} x ${container.style.height}`);
          
          // Calculate scale factors
          const widthScale = containerRect.width / container.style.width;
          const heightScale = containerRect.height / container.style.height;
          console.log(`    Size scaling: width ${widthScale.toFixed(4)}x, height ${heightScale.toFixed(4)}x`);
        }
        
        // Check each child node's actual DOM position vs JS position
        let domViolations = 0;
        childNodesInContainer.slice(0, 5).forEach(child => {
          const childElement = document.querySelector(`[data-id="${child.id}"]`);
          if (childElement && containerElement) {
            const childRect = childElement.getBoundingClientRect();
            const containerRect = containerElement.getBoundingClientRect();
            
            // Calculate various coordinate representations
            const domRelativeToContainer = {
              x: childRect.left - containerRect.left,
              y: childRect.top - containerRect.top
            };
            
            const domRelativeToInstance = reactFlowInstance ? {
              x: childRect.left - reactFlowInstance.getBoundingClientRect().left,
              y: childRect.top - reactFlowInstance.getBoundingClientRect().top
            } : { x: 0, y: 0 };
            
            console.log(`\n  📍 NODE ${child.id} COORDINATE ANALYSIS:`);
            console.log(`    JS relative position: (${child.position.x}, ${child.position.y})`);
            console.log(`    DOM screen position: (${childRect.left.toFixed(1)}, ${childRect.top.toFixed(1)})`);
            console.log(`    DOM relative to container: (${domRelativeToContainer.x.toFixed(1)}, ${domRelativeToContainer.y.toFixed(1)})`);
            console.log(`    DOM relative to ReactFlow: (${domRelativeToInstance.x.toFixed(1)}, ${domRelativeToInstance.y.toFixed(1)})`);
            console.log(`    DOM node size: ${childRect.width.toFixed(1)} x ${childRect.height.toFixed(1)}`);
            
            // Calculate coordinate discrepancy
            const discrepancyX = domRelativeToContainer.x - child.position.x;
            const discrepancyY = domRelativeToContainer.y - child.position.y;
            console.log(`    Coordinate discrepancy: (${discrepancyX.toFixed(1)}, ${discrepancyY.toFixed(1)})`);
            
            // Analyze potential causes of discrepancy
            if (Math.abs(discrepancyX) > 5 || Math.abs(discrepancyY) > 5) {
              console.log(`    ⚠️  SIGNIFICANT COORDINATE MISMATCH detected!`);
              
              // Check if discrepancy matches scale factor
              const scaleInfo = reactFlowViewport ? reactFlowViewport.style.transform.match(/scale\(([^)]+)\)/) : null;
              if (scaleInfo) {
                const scale = parseFloat(scaleInfo[1]);
                const scaledJSX = child.position.x * scale;
                const scaledJSY = child.position.y * scale;
                const scaleDiscrepancyX = domRelativeToContainer.x - scaledJSX;
                const scaleDiscrepancyY = domRelativeToContainer.y - scaledJSY;
                
                console.log(`    📐 Scale analysis (scale=${scale.toFixed(4)}):`);
                console.log(`      If JS coords were scaled: (${scaledJSX.toFixed(1)}, ${scaledJSY.toFixed(1)})`);
                console.log(`      Scaled discrepancy: (${scaleDiscrepancyX.toFixed(1)}, ${scaleDiscrepancyY.toFixed(1)})`);
                
                if (Math.abs(scaleDiscrepancyX) < Math.abs(discrepancyX) && Math.abs(scaleDiscrepancyY) < Math.abs(discrepancyY)) {
                  console.log(`    💡 SCALING APPEARS TO BE THE ISSUE!`);
                }
              }
            }
            
            // Check if child is actually outside container in DOM
            const outsideLeft = childRect.left < containerRect.left;
            const outsideRight = childRect.right > containerRect.right;
            const outsideTop = childRect.top < containerRect.top;
            const outsideBottom = childRect.bottom > containerRect.bottom;
            
            if (outsideLeft || outsideRight || outsideTop || outsideBottom) {
              domViolations++;
              realLayoutBroken = true;
              console.log(`    ❌ DOM BOUNDARY VIOLATION:`);
              if (outsideLeft) console.log(`      ⚠️  Child left ${childRect.left.toFixed(1)} < container left ${containerRect.left.toFixed(1)}`);
              if (outsideRight) console.log(`      ⚠️  Child right ${childRect.right.toFixed(1)} > container right ${containerRect.right.toFixed(1)}`);
              if (outsideTop) console.log(`      ⚠️  Child top ${childRect.top.toFixed(1)} < container top ${containerRect.top.toFixed(1)}`);
              if (outsideBottom) console.log(`      ⚠️  Child bottom ${childRect.bottom.toFixed(1)} > container bottom ${containerRect.bottom.toFixed(1)}`);
            } else {
              console.log(`    ✅ DOM position within container bounds`);
            }
          } else {
            console.log(`    ⚠️  Could not find DOM element for ${child.id || container.id}`);
          }
        });
        
        if (domViolations > 0) {
          console.log(`  ❌ ${domViolations} nodes actually outside container in DOM`);
        } else {
          console.log(`  ✅ All nodes within container bounds in DOM`);
        }
      }
    });
    
    console.log(`\n🎯 FINAL REALITY CHECK:`);
    if (realLayoutBroken) {
      console.log(`❌ CONFIRMED: Layout is visually broken in DOM - nodes outside containers`);
      console.log(`🔧 ROOT CAUSE ANALYSIS:`);
      
      // Analyze the most likely causes
      const scaleInfo = reactFlowViewport ? reactFlowViewport.style.transform.match(/scale\(([^)]+)\)/) : null;
      const scale = scaleInfo ? parseFloat(scaleInfo[1]) : 1.0;
      
      console.log(`📊 DIAGNOSTIC SUMMARY:`);
      console.log(`  Current ReactFlow scale: ${scale.toFixed(6)}`);
      console.log(`  Scale deviation from 1.0: ${Math.abs(scale - 1.0).toFixed(6)}`);
      console.log(`  Coordinate approach: ELK_HIERARCHICAL_DIRECT (no manual scaling)`);
      
      if (Math.abs(scale - 1.0) > 0.01) {
        console.log(`💡 PRIMARY HYPOTHESIS: ReactFlow scale factor ${scale.toFixed(4)} is causing coordinate mismatch`);
        console.log(`� POTENTIAL SOLUTIONS:`);
        console.log(`   A. Apply inverse scale factor to child coordinates: multiply by ${(1/scale).toFixed(4)}`);
        console.log(`   B. Force ReactFlow scale to 1.0 before layout calculation`);
        console.log(`   C. Use ReactFlow's coordinate transformation utilities`);
        console.log(`   D. Investigate parentNode + extent:'parent' behavior with scaled viewports`);
      } else {
        console.log(`💡 SCALE IS NEAR 1.0 - other factors at play:`);
        console.log(`🔧 INVESTIGATE:`);
        console.log(`   A. Container padding/border offsets`);
        console.log(`   B. ReactFlow internal coordinate transformations`);
        console.log(`   C. CSS transforms or positioning issues`);
        console.log(`   D. extent:'parent' implementation quirks`);
      }
      
      // Provide specific next steps
      console.log(`\n� RECOMMENDED NEXT STEPS:`);
      console.log(`   1. Test with ReactFlow scale forced to 1.0`);
      console.log(`   2. Try applying inverse scale factor: position * (1/scale)`);
      console.log(`   3. Check if removing extent:'parent' fixes positioning`);
      console.log(`   4. Examine ReactFlow v12 documentation for parentNode coordinate handling`);
      
    } else {
      console.log(`✅ DOM layout is actually correct - coordinate system working properly!`);
      console.log(`🎉 SUCCESS: ELK coordinates + ReactFlow parentNode system working as intended`);
    }
  }, 1000);

  if (finalNodesResult.length === 0) {
    console.error(`🚨 LAYOUT RETURNING EMPTY NODES!`);
    return { nodes, edges };
  }

  // Only include hyperedges when containers are collapsed
  const finalEdges = hasCollapsedContainers ? [...finalEdgesResult, ...hyperedgeResults] : finalEdgesResult;
  
  return { nodes: finalNodesResult, edges: finalEdges };
};
