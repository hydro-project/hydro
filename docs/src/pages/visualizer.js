import React, { useState, useCallback, useEffect, useRef } from 'react';
import Layout from '@theme/Layout';
import { useLocation } from '@docusaurus/router';
import styles from './visualizer.module.css';

// We'll use CDN approach compatible with React 18 like the docs infrastructure
let ReactFlowComponents = null;
let ELK = null;

const loadExternalLibraries = async () => {
  // Ensure React/ReactDOM are available globally for ReactFlow
  if (!window.React) {
    window.React = React;
  }
  
  if (!window.ReactDOM) {
    const ReactDOM = await import('react-dom');
    window.ReactDOM = ReactDOM;
  }
  
  // Load ReactFlow (compatible with React 18)
  if (!window.ReactFlow) {
    const reactFlowScript = document.createElement('script');
    reactFlowScript.src = 'https://unpkg.com/reactflow@11.11.4/dist/umd/index.js';
    document.head.appendChild(reactFlowScript);
    
    await new Promise((resolve, reject) => {
      reactFlowScript.onload = resolve;
      reactFlowScript.onerror = reject;
    });
  }
  
  // Load ELK.js for advanced layouts
  if (!window.ELK) {
    const elkScript = document.createElement('script');
    elkScript.src = 'https://unpkg.com/elkjs@0.8.2/lib/elk.bundled.js';
    document.head.appendChild(elkScript);
    
    await new Promise((resolve, reject) => {
      elkScript.onload = resolve;
      elkScript.onerror = reject;
    });
    
    ELK = new window.ELK();
  } else {
    ELK = new window.ELK();
  }
  
  // Load CSS
  if (!document.querySelector('link[href*="reactflow"]')) {
    const link = document.createElement('link');
    link.rel = 'stylesheet';
    link.href = 'https://unpkg.com/reactflow@11.11.4/dist/style.css';
    document.head.appendChild(link);
  }
  
  // Extract ReactFlow components
  const ReactFlowLib = window.ReactFlow;
  const { 
    default: ReactFlowComponent, 
    Controls, 
    MiniMap, 
    Background, 
    useNodesState, 
    useEdgesState, 
    addEdge, 
    applyNodeChanges, 
    applyEdgeChanges 
  } = ReactFlowLib;
  
  ReactFlowComponents = {
    ReactFlow: ReactFlowComponent,
    Controls,
    MiniMap,
    Background,
    useNodesState,
    useEdgesState,
    addEdge,
    applyNodeChanges,
    applyEdgeChanges
  };
  
  return { ReactFlowComponents, ELK };
};

// ELK layout configurations with VERY COMPACT spacing to prevent huge containers
const elkLayouts = {
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

// Expanded color palettes from template.html
const colorPalettes = {
  // Qualitative palettes
  'Set3': ['#8dd3c7', '#ffffb3', '#bebada', '#fb8072', '#80b1d3', '#fdb462', '#b3de69'],
  'Pastel1': ['#fbb4ae', '#b3cde3', '#ccebc5', '#decbe4', '#fed9a6', '#ffffcc', '#e5d8bd'],
  'Pastel2': ['#b3e2cd', '#fdcdac', '#cbd5e8', '#f4cae4', '#e6f5c9', '#fff2ae', '#f1e2cc'],
  'Set1': ['#e41a1c', '#377eb8', '#4daf4a', '#984ea3', '#ff7f00', '#ffff33', '#a65628'],
  'Set2': ['#66c2a5', '#fc8d62', '#8da0cb', '#e78ac3', '#a6d854', '#ffd92f', '#e5c494'],
  'Dark2': ['#1b9e77', '#d95f02', '#7570b3', '#e7298a', '#66a61e', '#e6ab02', '#a6761d'],
  'Accent': ['#7fc97f', '#beaed4', '#fdc086', '#ffff99', '#386cb0', '#f0027f', '#bf5b17'],
  'Paired': ['#a6cee3', '#1f78b4', '#b2df8a', '#33a02c', '#fb9a99', '#e31a1c', '#fdbf6f'],
  
  // Sequential palettes
  'Blues': ['#f7fbff', '#deebf7', '#c6dbef', '#9ecae1', '#6baed6', '#4292c6', '#2171b5'],
  'Greens': ['#f7fcf5', '#e5f5e0', '#c7e9c0', '#a1d99b', '#74c476', '#41ab5d', '#238b45'],
  'Oranges': ['#fff5eb', '#fee6ce', '#fdd0a2', '#fdae6b', '#fd8d3c', '#f16913', '#d94801'],
  'Purples': ['#fcfbfd', '#efedf5', '#dadaeb', '#bcbddc', '#9e9ac8', '#807dba', '#6a51a3'],
  'Reds': ['#fff5f0', '#fee0d2', '#fcbba1', '#fc9272', '#fb6a4a', '#ef3b2c', '#cb181d'],
  
  // Diverging palettes
  'Spectral': ['#9e0142', '#d53e4f', '#f46d43', '#fdae61', '#fee08b', '#e6f598', '#abdda4'],
  'RdYlBu': ['#d73027', '#f46d43', '#fdae61', '#fee090', '#e0f3f8', '#abd9e9', '#74add1'],
  'RdYlGn': ['#d73027', '#f46d43', '#fdae61', '#fee08b', '#d9ef8b', '#a6d96a', '#66bd63'],
  'PiYG': ['#d01c8b', '#f1b6da', '#fde0ef', '#f7f7f7', '#e6f5d0', '#b8e186', '#4d9221'],
  'BrBG': ['#8c510a', '#bf812d', '#dfc27d', '#f6e8c3', '#c7eae5', '#80cdc1', '#35978f'],
  
  // Modern/trendy palettes
  'Viridis': ['#440154', '#482777', '#3f4a8a', '#31678e', '#26838f', '#1f9d8a', '#6cce5a'],
  'Plasma': ['#0d0887', '#6a00a8', '#b12a90', '#e16462', '#fca636', '#f0f921', '#fcffa4'],
  'Warm': ['#375a7f', '#5bc0de', '#5cb85c', '#f0ad4e', '#d9534f', '#ad4e92', '#6f5499'],
  'Cool': ['#2c3e50', '#3498db', '#1abc9c', '#16a085', '#27ae60', '#2980b9', '#8e44ad'],
  'Earth': ['#8b4513', '#a0522d', '#cd853f', '#daa520', '#b8860b', '#228b22', '#006400']
};

// Color generation functions from template.html
const generateNodeColors = (nodeType, palette = 'Set3') => {
  const colors = colorPalettes[palette];
  const typeMap = {
    'Source': 0,
    'Transform': 1,
    'Join': 2,
    'Aggregation': 3,
    'Network': 4,
    'Sink': 5,
    'Tee': 6
  };
  
  const baseColor = colors[typeMap[nodeType] || 0];
  
  // Create gradient colors
  const primary = baseColor;
  const secondary = lightenColor(baseColor, 10);
  const tertiary = lightenColor(baseColor, 25);
  const border = darkenColor(baseColor, 5);
  
  // Create a gentle linear gradient
  const gradient = `linear-gradient(0deg, ${tertiary} 0%, ${secondary} 80%, ${primary} 100%)`;
  
  return { primary, secondary, tertiary, border, gradient };
};

// Color manipulation functions
const lightenColor = (color, percent) => `color-mix(in srgb, ${color} ${100-percent}%, white)`;
const darkenColor = (color, percent) => `color-mix(in srgb, ${color} ${100-percent}%, black)`;

const generateLocationColor = (locationId, totalLocations, palette = 'Set3') => {
  const colors = colorPalettes[palette];
  const color = colors[locationId % colors.length];
  return `${color}40`; // Add transparency
};

const generateLocationBorderColor = (locationId, totalLocations, palette = 'Set3') => {
  const colors = colorPalettes[palette];
  return colors[locationId % colors.length];
};

// This function is now unused in the hierarchical approach but kept for potential simple layouts.
const applyElkLayout = async (nodes, edges, layoutType = 'layered') => {
  // This function is now unused in the hierarchical approach but kept for potential simple layouts.
  if (!ELK) return nodes;
  
  console.log(`🔍 APPLYING SIMPLE ELK LAYOUT (${layoutType}) to ${nodes.length} nodes`);
  
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

function ReactFlowVisualization({ graphData }) {
  const [reactFlowReady, setReactFlowReady] = useState(false);
  const [initialNodes, setInitialNodes] = useState([]);
  const [initialEdges, setInitialEdges] = useState([]);
  const [currentLayout, setCurrentLayout] = useState('mrtree');
  const [colorPalette, setColorPalette] = useState('Set3');
  const [locationData, setLocationData] = useState(new Map());
  const [collapsedContainers, setCollapsedContainers] = useState({});

  // Load external libraries when component mounts
  useEffect(() => {
    loadExternalLibraries().then(() => {
      console.log('ReactFlow and ELK loaded successfully');
      setReactFlowReady(true);
    }).catch((error) => {
      console.error('Failed to load external libraries:', error);
    });
  }, []);

  // Process graph data when ReactFlow is loaded and data changes
  useEffect(() => {
    if (!reactFlowReady || !graphData || !ELK) return;

    const processData = async () => {
      console.log('Processing graph data for hierarchical layout:', graphData);
      
      // Centralized location processing
      const locations = new Map();
      if (graphData.locations) {
        graphData.locations.forEach(location => {
          if (location && typeof location.id !== 'undefined') {
            // Ensure the key is a number to match node.data.locationId
            locations.set(parseInt(location.id, 10), location);
          } else {
            console.warn('Skipping invalid location object:', location);
          }
        });
      }
      
      (graphData.nodes || []).forEach(node => {
        if (node.data?.locationId !== undefined && node.data?.location && !locations.has(node.data.locationId)) {
          console.log(`Node ${node.id} defines a new location: ${node.data.location}`);
          locations.set(node.data.locationId, { id: node.data.locationId, label: node.data.location });
        }
      });
      
      setLocationData(new Map(locations)); // Update state for the legend
      console.log('Final locations map:', locations);

      // Convert nodes with enhanced styling
      let processedNodes = (graphData.nodes || []).map(node => {
        const nodeColors = generateNodeColors(node.data?.nodeType || 'Transform', colorPalette);
        
        return {
          ...node,
          position: { x: 0, y: 0 },
          style: {
            background: nodeColors.gradient,
            border: `2px solid ${nodeColors.border}`,
            borderRadius: '8px',
            padding: '10px',
            color: '#333',
            fontSize: '12px',
            fontWeight: '500',
            width: 200,
            height: 60,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            textAlign: 'center',
          },
        };
      });

      // Convert edges with enhanced styling
      const processedEdges = (graphData.edges || []).map(edge => ({
        ...edge,
        type: 'bezier',
        style: { strokeWidth: 2, stroke: '#666666' },
        markerEnd: { type: 'arrowclosed', width: 20, height: 20, color: '#666666' },
      }));

      // Apply ELK layout with hierarchical grouping
      console.log('Applying ELK layout with hierarchical grouping...');
      const layoutResult = await applyHierarchicalLayout(processedNodes, processedEdges, currentLayout, locations, colorPalette, collapsedContainers);
      
      // layoutResult now contains both regular nodes and container nodes
      setInitialNodes(layoutResult.nodes);
      setInitialEdges(layoutResult.edges);
    };

    processData();
  }, [reactFlowReady, graphData, currentLayout, colorPalette, collapsedContainers]);

  // NEW HIERARCHICAL LAYOUT APPROACH
  const applyHierarchicalLayout = async (nodes, edges, layoutType, locations, currentPalette, collapsedContainers = {}) => {
    if (!ELK) return { nodes, edges };

    console.log('🚀 NEW APPROACH: Single hierarchical layout pass with ELK');
    console.log('📦 Collapsed containers:', collapsedContainers);

    console.log('--- DEBUG: Nodes received by applyHierarchicalLayout ---');
    nodes.forEach(n => {
      console.log(JSON.stringify({ id: n.id, data: n.data }, null, 2));
    });
    console.log('--- END DEBUG ---');

    const nodeMap = new Map(nodes.map(n => [n.id, n]));
    const locationGroups = new Map();
    const orphanNodeIds = new Set(nodes.map(n => n.id));

    // 1. Group nodes by location, using the passed-in 'locations' map.
    // This is more robust than iterating over location.nodes.
    console.log('--- DEBUG: Grouping nodes by location ---');
    nodes.forEach(node => {
      const locationId = node.data?.locationId;
      console.log(`Node ${node.id}: checking for locationId. Found: ${locationId}`);
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
    console.log('--- END DEBUG ---');

    console.log(`📊 GROUPED: ${locationGroups.size} location groups, ${orphanNodeIds.size} orphan nodes`);
    locationGroups.forEach((group, key) => {
      console.log(`- Group ${key}: ${group.nodeIds.size} nodes`);
    });

    // 2. Build the ELK graph structure with hierarchy
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
        // If expanded, include all child nodes
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
    
    console.log('📋 Existing node IDs in ELK graph:', Array.from(existingNodeIds));

    // Filter and reroute edges to only reference existing nodes
    const validElkEdges = [];
    console.log('🔗 Processing edges for collapsed containers...');
    edges.forEach(edge => {
      let sourceId = edge.source;
      let targetId = edge.target;
      
      console.log(`Processing edge: ${edge.source} -> ${edge.target}`);
      
      // Check if source node is in a collapsed container
      const sourceNode = nodeMap.get(edge.source);
      if (sourceNode?.data?.locationId !== undefined) {
        const sourceContainerId = `container_${sourceNode.data.locationId}`;
        if (collapsedContainers[sourceContainerId]) {
          console.log(`  Source ${edge.source} is in collapsed container ${sourceContainerId}`);
          sourceId = sourceContainerId;
        }
      }
      
      // Check if target node is in a collapsed container
      const targetNode = nodeMap.get(edge.target);
      if (targetNode?.data?.locationId !== undefined) {
        const targetContainerId = `container_${targetNode.data.locationId}`;
        if (collapsedContainers[targetContainerId]) {
          console.log(`  Target ${edge.target} is in collapsed container ${targetContainerId}`);
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
        console.log(`  ✅ Adding valid edge: ${sourceId} -> ${targetId}`);
        validElkEdges.push(newEdge);
      } else {
        console.log(`  ❌ Skipping edge - Source exists: ${existingNodeIds.has(sourceId)}, Target exists: ${existingNodeIds.has(targetId)}, Same: ${sourceId === targetId}`);
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

    console.log('ELK graph input:', JSON.stringify(elkGraph, null, 2));

    // 3. Apply ELK layout
    const layoutedGraph = await ELK.layout(elkGraph);
    console.log('ELK layout result:', layoutedGraph);

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
          type: isCollapsed ? 'default' : 'group', // Use regular node type for collapsed containers
          position: { x: elkNode.x, y: elkNode.y },
          style: containerStyle,
          data: {
            label: isCollapsed ? `${location.label || `Location ${location.id}`} (${elkNode.originalNodeIds?.length || 0} nodes)` : location.label || `Location ${location.id}`,
            isContainer: true,
            locationId: location.id,
            isCollapsed: isCollapsed,
            nodeCount: elkNode.originalNodeIds?.length || 0
          },
          draggable: true,
          selectable: true,
          connectable: true, // Always connectable, regardless of collapsed state
        });
      }
    });

    const validContainerIds = new Set(containerNodes.map(n => n.id));

    // Third pass: Create all child and orphan nodes
    nodes.forEach(originalNode => {
      const locationId = originalNode.data?.locationId;
      const isChild = locationId !== undefined && locationId !== null;
      const containerId = isChild ? `container_${locationId}` : null;
      const isContainerCollapsed = containerId && collapsedContainers[containerId];

      // Skip processing nodes that are in collapsed containers entirely
      if (isChild && isContainerCollapsed) {
        console.log(`Skipping node ${originalNode.id} because container ${containerId} is collapsed`);
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

    // Combine containers and other nodes, ensuring containers come first.
    const finalNodesResult = [...containerNodes, ...childAndOrphanNodes];
    
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
    
    console.log('Final nodes for React Flow:', finalNodesResult);
    console.log('Final edges for React Flow:', finalEdgesResult);
    return { nodes: finalNodesResult, edges: finalEdgesResult };
  };

  const handleLayoutChange = useCallback((newLayout) => {
    setCurrentLayout(newLayout);
  }, []);

  const handlePaletteChange = useCallback((newPalette) => {
    setColorPalette(newPalette);
  }, []);

  const handleContainerToggle = useCallback((containerId) => {
    setCollapsedContainers(prev => ({
      ...prev,
      [containerId]: !prev[containerId]
    }));
  }, []);

  if (!reactFlowReady) {
    return <div className={styles.loading}>Loading ReactFlow visualization...</div>;
  }

  if (!ReactFlowComponents) {
    return <div className={styles.loading}>ReactFlow not available.</div>;
  }

  return (
    <div className={styles.visualizationWrapper}>
      {/* Layout Controls */}
      <div className={styles.layoutControls}>
        <select 
          className={styles.layoutSelect}
          value={currentLayout} 
          onChange={(e) => handleLayoutChange(e.target.value)}
        >
          {Object.keys(elkLayouts).map(key => (
            <option key={key} value={key}>{key.charAt(0).toUpperCase() + key.slice(1)}</option>
          ))}
        </select>
        
        <select 
          className={styles.paletteSelect}
          value={colorPalette} 
          onChange={(e) => handlePaletteChange(e.target.value)}
        >
          {Object.keys(colorPalettes).map(key => (
            <option key={key} value={key}>{key}</option>
          ))}
        </select>
      </div>

      {/* Legend */}
      <div className={styles.unifiedLegend}>
        <h4>Legend</h4>
        
        <div className={styles.legendSection}>
          <strong>Node Types:</strong>
          {['Source', 'Transform', 'Join', 'Aggregation', 'Network', 'Sink', 'Tee'].map(type => {
            const colors = generateNodeColors(type, colorPalette);
            return (
              <div key={type} className={styles.legendItem}>
                <div 
                  className={styles.legendColor}
                  style={{ background: colors.primary, borderColor: colors.border }}
                />
                <span>{type}</span>
              </div>
            );
          })}
        </div>

        {locationData.size > 0 && (
          <div className={styles.legendSection}>
            <strong>Locations:</strong>
            {Array.from(locationData.entries()).map(([locationId, location]) => {
              const bgColor = generateLocationColor(locationId, locationData.size, colorPalette);
              const borderColor = generateLocationBorderColor(locationId, locationData.size, colorPalette);
              return (
                <div key={locationId} className={styles.legendItem}>
                  <div 
                    className={styles.locationLegendColor}
                    style={{ background: bgColor, borderColor: borderColor }}
                  />
                  <span>{location.label || location.name || `Location ${locationId}`}</span>
                </div>
              );
            })}
          </div>
        )}
      </div>
      
      <ReactFlowInner 
        nodes={initialNodes} 
        edges={initialEdges}
        locationData={locationData}
        colorPalette={colorPalette}
        onContainerToggle={handleContainerToggle}
      />
    </div>
  );
}

// Inner component that uses ReactFlow hooks
function ReactFlowInner({ nodes, edges, locationData, colorPalette, onContainerToggle }) {
  const { ReactFlow, Controls, MiniMap, Background, useNodesState, useEdgesState, addEdge } = ReactFlowComponents;

  const [currentNodes, setNodes, onNodesChange] = useNodesState(nodes);
  const [currentEdges, setEdges, onEdgesChange] = useEdgesState(edges);

  // Update nodes and edges when props change
  useEffect(() => {
    setNodes(nodes);
  }, [nodes, setNodes]);

  useEffect(() => {
    setEdges(edges);
  }, [edges, setEdges]);

  const onConnect = useCallback((connection) => {
    setEdges((eds) => addEdge(connection, eds));
  }, [setEdges, addEdge]);

  const onNodeClick = useCallback((event, node) => {
    if (node.type === 'group' || node.data?.isContainer) {
      onContainerToggle(node.id);
    }
  }, [onContainerToggle]);

  return (
    <div className={styles.reactflowWrapper}>
      <ReactFlow
        nodes={currentNodes}
        edges={currentEdges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onNodeClick={onNodeClick}
        fitView
        attributionPosition="bottom-left"
        nodesDraggable={true}
        nodesConnectable={true}
        elementsSelectable={true}
      >
        <Controls />
        <MiniMap 
          nodeColor={(node) => {
            if (node.data?.isContainer) {
              const locationId = node.data.locationId;
              return generateLocationBorderColor(locationId, locationData?.size || 1, colorPalette);
            }
            const nodeColors = generateNodeColors(node.data?.type || 'Transform', colorPalette);
            return nodeColors.primary;
          }}
          nodeStrokeWidth={2}
          nodeStrokeColor="#666"
          maskColor="rgba(240, 240, 240, 0.6)"
        />
        <Background color="#f5f5f5" gap={20} />
      </ReactFlow>
    </div>
  );
}

function FileDropZone({ onFileLoad, hasData }) {
  const [isDragOver, setIsDragOver] = useState(false);

  console.log('FileDropZone rendering, hasData:', hasData);

  const handleDragOver = useCallback((e) => {
    e.preventDefault();
    setIsDragOver(true);
  }, []);

  const handleDragLeave = useCallback((e) => {
    e.preventDefault();
    setIsDragOver(false);
  }, []);

  const handleDrop = useCallback((e) => {
    e.preventDefault();
    setIsDragOver(false);
    
    const files = Array.from(e.dataTransfer.files);
    const jsonFile = files.find(file => file.name.endsWith('.json'));
    
    if (jsonFile) {
      const reader = new FileReader();
      reader.onload = (event) => {
        try {
          const data = JSON.parse(event.target.result);
          onFileLoad(data);
        } catch (error) {
          alert('Invalid JSON file: ' + error.message);
        }
      };
      reader.readAsText(jsonFile);
    } else {
      alert('Please drop a JSON file');
    }
  }, [onFileLoad]);

  const handleFileInput = useCallback((e) => {
    const file = e.target.files[0];
    if (file && file.name.endsWith('.json')) {
      const reader = new FileReader();
      reader.onload = (event) => {
        try {
          const data = JSON.parse(event.target.result);
          onFileLoad(data);
        } catch (error) {
          alert('Invalid JSON file: ' + error.message);
        }
      };
      reader.readAsText(file);
    }
  }, [onFileLoad]);

  if (hasData) {
    console.log('FileDropZone: hasData is true, returning null');
    return null;
  }

  console.log('FileDropZone: rendering drop zone');

  return (
    <div 
      className={`${styles.dropZone} ${isDragOver ? styles.dragOver : ''}`}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      style={{ backgroundColor: '#fff', border: '3px dashed #ccc' }}
    >
      <div className={styles.dropContent}>
        <h3>Hydro Graph Visualizer</h3>
        <p>Drop a Hydro ReactFlow JSON file here or click to select</p>
        <input 
          type="file" 
          accept=".json"
          onChange={handleFileInput}
          className={styles.fileInput}
          id="file-input"
        />
        <label htmlFor="file-input" className={styles.fileInputLabel}>
          Choose File
        </label>
        <div className={styles.helpText}>
          <p>Generate JSON files using:</p>
          <code>built_flow.reactflow_to_file("graph.json")</code>
        </div>
      </div>
    </div>
  );
}

export default function Visualizer() {
  const location = useLocation();
  const [graphData, setGraphData] = useState(null);
  const [error, setError] = useState(null);

  console.log('Visualizer component rendering, graphData:', !!graphData);

  // Check for URL-encoded data on component mount
  useEffect(() => {
    // Parse URL hash for data parameter (like mermaid.live)
    const hash = location.hash;
    if (hash.startsWith('#data=')) {
      try {
        const encodedData = hash.substring(6); // Remove '#data='
        // Convert Base64URL to regular Base64
        const base64 = encodedData.replace(/-/g, '+').replace(/_/g, '/');
        // Add padding if needed
        const padded = base64 + '==='.slice(0, (4 - base64.length % 4) % 4);
        const jsonString = atob(padded); // Base64 decode
        const data = JSON.parse(jsonString);
        setGraphData(data);
      } catch (error) {
        setError('Failed to decode graph data from URL: ' + error.message);
      }
    }
  }, [location.hash]);

  const handleFileLoad = useCallback((data) => {
    console.log('File loaded, data:', data);
    setGraphData(data);
    setError(null);
  }, []);

  const handleClearData = useCallback(() => {
    setGraphData(null);
    setError(null);
    // Clear URL hash
    window.history.replaceState(null, null, window.location.pathname);
  }, []);

  console.log('About to render Layout with container');

  return (
    <Layout
      title="Graph Visualizer"
      description="Interactive ReactFlow visualization for Hydro graphs"
    >
      <div 
        className={styles.container} 
        style={{ 
          backgroundColor: '#f0f0f0', 
          minHeight: '500px',
          border: '2px solid red',
          padding: '20px'
        }}
      >
        <h1 style={{ color: 'black', margin: '20px 0' }}>Debug: Visualizer Container</h1>
        {console.log('Rendering container content, error:', error, 'graphData:', !!graphData)}
        {error && (
          <div className={styles.error}>
            <strong>Error:</strong> {error}
            <button onClick={() => setError(null)} className={styles.closeError}>×</button>
          </div>
        )}
        
        {graphData ? (
          <div className={styles.visualizationContainer}>
            <div className={styles.toolbar}>
              <h2>Hydro Graph Visualization</h2>
              <button onClick={handleClearData} className={styles.clearButton}>
                Load New Graph
              </button>
            </div>
            <ReactFlowVisualization graphData={graphData} />
          </div>
        ) : (
          <div style={{ border: '2px solid blue', padding: '10px', minHeight: '200px' }}>
            <h2 style={{ color: 'black' }}>Debug: Should show FileDropZone</h2>
            {console.log('Rendering FileDropZone')}
            <FileDropZone onFileLoad={handleFileLoad} hasData={!!graphData} />
          </div>
        )}
      </div>
    </Layout>
  );
}
