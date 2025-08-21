/**
 * Graph Visualization Homepage
 * 
 * Latest version of the graph visualization system using visualizer-v4.
 * Features file upload, JSON parsing, and ReactFlow v12 + ELK layout visualization.
 * 
 * This is the current/latest version - previous versions available at /vis3, /visualizer
 */

import React from 'react';
import Layout from '@theme/Layout';
import BrowserOnly from '@docusaurus/BrowserOnly';
import { useLocation } from '@docusaurus/router';

// Typography constants for consistent styling
const TYPOGRAPHY = {
  PAGE_TITLE: '2.5em',
  PAGE_SUBTITLE: '0.9em'
};

// Helper function to calculate container depth in hierarchy
function getContainerDepth(visState, containerId, depth = 0) {
  const container = visState.getContainer(containerId);
  if (!container) return depth;
  
  const parentId = visState.getNodeContainer(containerId);
  if (!parentId) return depth;
  
  return getContainerDepth(visState, parentId, depth + 1);
}

function HydroscopeComponent() {
  const location = useLocation();
  const [createVisualizationState, setCreateVisualizationState] = React.useState(null);
  const [FlowGraph, setFlowGraph] = React.useState(null);
  const [parseGraphJSON, setParseGraphJSON] = React.useState(null);
  const [createRenderConfig, setCreateRenderConfig] = React.useState(null);
  const [getAvailableGroupings, setGetAvailableGroupings] = React.useState(null);
  const [validateGraphJSON, setValidateGraphJSON] = React.useState(null);
  const [NODE_STYLES, setNodeStyles] = React.useState(null);
  const [EDGE_STYLES, setEdgeStyles] = React.useState(null);
  const [InfoPanel, setInfoPanel] = React.useState(null);
  const [LayoutControls, setLayoutControls] = React.useState(null);
  const [StyleTunerPanel, setStyleTunerPanel] = React.useState(null);
  const [FileDropZone, setFileDropZone] = React.useState(null);
  const [groupingOptions, setGroupingOptions] = React.useState([]);
  const [currentGrouping, setCurrentGrouping] = React.useState(null);
  const [colorPalette, setColorPalette] = React.useState('Set3');
  const [layoutAlgorithm, setLayoutAlgorithm] = React.useState('mrtree');
  const [autoFit, setAutoFit] = React.useState(true);
  const [styleConfig, setStyleConfig] = React.useState({
    edgeStyle: 'bezier',
    edgeColor: '#1976d2',
    edgeWidth: 2,
    edgeDashed: false,
    nodeBorderRadius: 4,
    nodePadding: 12,
    nodeFontSize: 12,
    containerBorderRadius: 8,
    containerBorderWidth: 2,
    containerShadow: 'LIGHT'
  });
  const [error, setError] = React.useState(null);
  const [loading, setLoading] = React.useState(true);
  const [currentVisualizationState, setCurrentVisualizationState] = React.useState(null);
  const [edgeStyleConfig, setEdgeStyleConfig] = React.useState(null);
  const [graphData, setGraphData] = React.useState(null);
  const [generatedFilePath, setGeneratedFilePath] = React.useState(null);
  
  // Force re-render counter when VisState internal state changes
  const [, forceUpdate] = React.useReducer(x => x + 1, 0);
  
  // Track if we're currently running a layout operation
  const [isLayoutRunning, setIsLayoutRunning] = React.useState(false);
  
  // Track if we're currently changing grouping (to prevent DropZone flicker)
  const [isChangingGrouping, setIsChangingGrouping] = React.useState(false);
  
  // Track which nodes are showing full labels (node ID -> boolean)
  const [nodesShowingFullLabel, setNodesShowingFullLabel] = React.useState(new Set());
  
  // Ref for FlowGraph to call fitView directly
  const flowGraphRef = React.useRef(null);

  // Load components on mount
  React.useEffect(() => {
    const loadVisualizerComponents = async () => {
      try {
        // Import all components from visualizer-v4
        const { 
          createVisualizationState,
          FlowGraph,
          parseGraphJSON,
          createRenderConfig,
          getAvailableGroupings,
          validateGraphJSON,
          NODE_STYLES,
          EDGE_STYLES,
          InfoPanel,
          LayoutControls,
          StyleTunerPanel,
          FileDropZone
        } = await import('../components/visualizer-v4');
        
        setCreateVisualizationState(() => createVisualizationState);
        setFlowGraph(() => FlowGraph);
        setParseGraphJSON(() => parseGraphJSON);
        setCreateRenderConfig(() => createRenderConfig);
        setGetAvailableGroupings(() => getAvailableGroupings);
        setValidateGraphJSON(() => validateGraphJSON);
        setNodeStyles(NODE_STYLES);
        setEdgeStyles(EDGE_STYLES);
        setInfoPanel(() => InfoPanel);
        setLayoutControls(() => LayoutControls);
        setStyleTunerPanel(() => StyleTunerPanel);
        setFileDropZone(() => FileDropZone);
        // Don't load grouping options here - wait until we have graph data
        setLoading(false);
        setError(null);
      } catch (err) {
        console.error('❌ Failed to load visualizer components:', err);
        setError(`Failed to load visualizer components: ${err.message}`);
        setLoading(false);
      }
    };
    loadVisualizerComponents();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Handle URL data parameter (for sharing graphs via URL)
  React.useEffect(() => {
    if (!parseGraphJSON || !createRenderConfig || !createVisualizationState || !validateGraphJSON || !getAvailableGroupings || loading) return;
    
    const urlParams = new URLSearchParams(location.search);
    // Also check URL fragment/hash for parameters
    const hashParams = new URLSearchParams(location.hash.slice(1)); // Remove the '#'
    const dataParam = urlParams.get('data') || hashParams.get('data');
    const compressedParam = urlParams.get('compressed') || hashParams.get('compressed');
    const fileParam = urlParams.get('file') || hashParams.get('file');
    
    // Handle file path parameter (from Rust debug output)
    if (fileParam && !generatedFilePath) {
      setGeneratedFilePath(decodeURIComponent(fileParam));
    }
    
    if (dataParam && !currentVisualizationState) {
      try {        
        // Decode the base64 data
        const jsonString = atob(dataParam);
        const jsonData = JSON.parse(jsonString);
        
        // Validate and parse the JSON
        const validationResult = validateGraphJSON(jsonData);
        if (!validationResult.isValid) {
          throw new Error(`Invalid graph data: ${validationResult.errors.join(', ')}`);
        }
        
        const parsedData = parseGraphJSON(jsonData);
        const renderConfig = createRenderConfig(parsedData);
        setCurrentVisualizationState(parsedData.state);
        setEdgeStyleConfig(renderConfig);
        setGraphData(jsonData);
        
        // Extract grouping options from the data
        const groupings = getAvailableGroupings(jsonData);
        setGroupingOptions(groupings);
        
        // Set default grouping if not set and groupings are available
        if ((!currentGrouping || typeof currentGrouping !== 'string') && groupings.length > 0) {
          setCurrentGrouping(groupings[0].id);
        }
        
      } catch (err) {
        console.error('❌ Error loading graph from URL:', err);
        setError(`Failed to load graph from URL: ${err.message}`);
      }
    } else if (compressedParam && !currentVisualizationState) {
      // Handle compressed data
      loadCompressedData(compressedParam);
    }
  }, [location.search, location.hash, parseGraphJSON, createRenderConfig, validateGraphJSON, getAvailableGroupings, createVisualizationState, loading, currentVisualizationState, currentGrouping, generatedFilePath]);

  // Load compressed data from URL parameter
  const loadCompressedData = React.useCallback(async (compressedData) => {
    try {
      setLoading(true);
      
      // Convert URL-safe base64 to standard base64
      // Replace URL-safe characters and add padding if needed
      let standardBase64 = compressedData
        .replace(/-/g, '+')
        .replace(/_/g, '/');
      
      // Add padding if needed
      while (standardBase64.length % 4) {
        standardBase64 += '=';
      }
      
      // Decode base64 and decompress
      const compressedBytes = Uint8Array.from(atob(standardBase64), c => c.charCodeAt(0));
      
      // Use browser's built-in decompression (if available) or fallback
      let jsonString;
      if (typeof DecompressionStream !== 'undefined') {
        // Modern browser with Compression Streams API
        const stream = new DecompressionStream('gzip');
        const writer = stream.writable.getWriter();
        const reader = stream.readable.getReader();
        
        writer.write(compressedBytes);
        writer.close();
        
        const chunks = [];
        let done = false;
        while (!done) {
          const { value, done: readerDone } = await reader.read();
          done = readerDone;
          if (value) chunks.push(value);
        }
        
        const decompressed = new Uint8Array(chunks.reduce((acc, chunk) => acc + chunk.length, 0));
        let offset = 0;
        for (const chunk of chunks) {
          decompressed.set(chunk, offset);
          offset += chunk.length;
        }
        
        jsonString = new TextDecoder().decode(decompressed);
      } else {
        // Fallback: assume uncompressed for older browsers
        console.warn('Browser does not support compression streams, assuming uncompressed data');
        jsonString = new TextDecoder().decode(compressedBytes);
      }
      
      const jsonData = JSON.parse(jsonString);
      
      // Validate and parse the JSON
      const validationResult = validateGraphJSON(jsonData);
      if (!validationResult.isValid) {
        throw new Error(`Invalid graph data: ${validationResult.errors.join(', ')}`);
      }
      
      const parsedData = parseGraphJSON(jsonData);
      const renderConfig = createRenderConfig(parsedData);
      setCurrentVisualizationState(parsedData.state);
      setEdgeStyleConfig(renderConfig);
      setGraphData(jsonData);
      
      // Extract grouping options from the data
      const groupings = getAvailableGroupings(jsonData);
      setGroupingOptions(groupings);
      
      // Set default grouping if not set and groupings are available
      if ((!currentGrouping || typeof currentGrouping !== 'string') && groupings.length > 0) {
        setCurrentGrouping(groupings[0].id);
      }      
    } catch (err) {
      console.error('❌ Error loading compressed data from URL:', err);
      setError(`Failed to load compressed graph from URL: ${err.message}`);
    } finally {
      setLoading(false);
    }
  }, [parseGraphJSON, createRenderConfig, validateGraphJSON, getAvailableGroupings, currentGrouping]);

  // File upload handler
  const handleFileLoad = React.useCallback((jsonData) => {
    if (!parseGraphJSON || !createRenderConfig || !validateGraphJSON || !getAvailableGroupings) {
      setError('Components not loaded yet');
      return;
    }
    
    try {
      // Validate the JSON
      const validationResult = validateGraphJSON(jsonData);
      if (!validationResult.isValid) {
        throw new Error(`Invalid graph data: ${validationResult.errors.join(', ')}`);
      }
      
      // Parse the JSON
      const parsedData = parseGraphJSON(jsonData);
      const renderConfig = createRenderConfig(parsedData);
      setCurrentVisualizationState(parsedData.state);
      setEdgeStyleConfig(renderConfig);
      setGraphData(jsonData);
      
      // Extract grouping options from the data
      const groupings = getAvailableGroupings(jsonData);
      setGroupingOptions(groupings);
      
      // Set default grouping if not set and groupings are available
      if ((!currentGrouping || typeof currentGrouping !== 'string') && groupings.length > 0) {
        setCurrentGrouping(groupings[0].id);
      }
      
      setError(null);
      
    } catch (err) {
      console.error('❌ Error processing file:', err);
      setError(`Failed to process file: ${err.message}`);
    }
  }, [parseGraphJSON, createRenderConfig, validateGraphJSON, getAvailableGroupings, currentGrouping]);

  // Event listener for example data loading from CompleteExampleDisplay
  React.useEffect(() => {
    const handleLoadExampleData = (event) => {
      const exampleData = event.detail;
      if (exampleData) {
        handleFileLoad(exampleData);
      }
    };

    window.addEventListener('load-example-data', handleLoadExampleData);
    
    return () => {
      window.removeEventListener('load-example-data', handleLoadExampleData);
    };
  }, [handleFileLoad]);

  // ============================
  // EVENT HANDLERS
  // ============================

  // Node click handler - handles container collapse/expand and label toggle
  const handleNodeClick = React.useCallback(async (event, node) => {
    if (!currentVisualizationState) return;
    
    // Check if this is a container node that can be collapsed/expanded
    if (node.type === 'container') {
      try {
        setIsLayoutRunning(true);
        
        const container = currentVisualizationState.getContainer(node.id);
        if (container) {
          console.log(`🔄 Container ${node.id} BEFORE operation - collapsed: ${container.collapsed}`);
          
          if (container.collapsed) {
            console.log(`🔄 Expanding container: ${node.id}`);
            currentVisualizationState.expandContainer(node.id);
          } else {
            console.log(`🔄 Collapsing container: ${node.id}`);
            currentVisualizationState.collapseContainer(node.id);
          }
          
          console.log(`🔄 Container ${node.id} AFTER operation - collapsed: ${container.collapsed}`);
          
          // CRITICAL: Trigger layout refresh after container state change
          // Container expansion/collapse changes which nodes are visible, requiring ELK re-layout
          console.log(`🔄 Container ${node.id} operation complete, triggering layout refresh...`);
          if (flowGraphRef.current && flowGraphRef.current.refreshLayout) {
            await flowGraphRef.current.refreshLayout();
            console.log(`✅ Layout refresh completed for container: ${node.id}`);
          } else {
            console.warn('⚠️ refreshLayout not available, using forceUpdate fallback');
            // Fallback: force component update if refreshLayout not available
            forceUpdate();
          }
          
          // Trigger auto-fit after layout completes
          if (autoFit && flowGraphRef.current && flowGraphRef.current.fitView) {
            setTimeout(() => {
              flowGraphRef.current.fitView();
            }, 500); // Wait for layout to complete
          }
        }
      } catch (err) {
        console.error('❌ Error toggling container:', err);
        setError(`Failed to toggle container: ${err.message}`);
      } finally {
        // Add a small delay to let the layout complete
        setTimeout(() => setIsLayoutRunning(false), 1000);
      }
    } else {
      // Standard node - toggle label display between short and full
      const nodeData = node.data;
      if (nodeData && nodeData.shortLabel && nodeData.fullLabel) {
        const nodeId = node.id;
        const isShowingFull = nodesShowingFullLabel.has(nodeId);
        
        console.log(`🏷️ Toggling label for node ${nodeId}: ${isShowingFull ? 'full→short' : 'short→full'}`);
        console.log(`🏷️ Before update - shortLabel: "${nodeData.shortLabel}", fullLabel: "${nodeData.fullLabel}"`);
        
        // Update the node's display label in the visualization state
        const currentLabel = isShowingFull ? nodeData.shortLabel : nodeData.fullLabel;
        console.log(`🏷️ Setting label to: "${currentLabel}"`);
        currentVisualizationState.updateNode(nodeId, { 
          label: currentLabel 
        });
        
        // Verify the update worked
        const updatedNode = currentVisualizationState._collections.graphNodes.get(nodeId);
        console.log(`🏷️ After update - node.label: "${updatedNode?.label}"`);
        
        // Update our tracking state
        setNodesShowingFullLabel(prev => {
          const newSet = new Set(prev);
          if (isShowingFull) {
            newSet.delete(nodeId);
          } else {
            newSet.add(nodeId);
          }
          return newSet;
        });
        
        // Trigger layout refresh to show the updated label
        console.log('🏷️ Triggering layout refresh...');
        if (flowGraphRef.current && flowGraphRef.current.refreshLayout) {
          await flowGraphRef.current.refreshLayout();
          console.log('✅ Layout refresh completed for label toggle');
        } else {
          // Fallback: force component update if refreshLayout not available
          console.log('🏷️ Using forceUpdate fallback');
          forceUpdate();
        }
      }
    }
  }, [currentVisualizationState, autoFit, nodesShowingFullLabel]);

  // Pack all containers (collapse all)
  const handlePackAll = React.useCallback(async () => {
    if (!currentVisualizationState) return;
    
    setIsLayoutRunning(true);
    
    try {
      // Get ALL containers (not just visible ones) to ensure we collapse nested hierarchies
      const allContainers = Array.from(currentVisualizationState.containers.values());
      
      // Sort containers by depth (deepest first) for bottom-up collapse
      const containersByDepth = allContainers
        .filter(container => !container.collapsed)
        .sort((a, b) => {
          const depthA = getContainerDepth(currentVisualizationState, a.id);
          const depthB = getContainerDepth(currentVisualizationState, b.id);
          return depthB - depthA; // Deepest first
        });
      
      containersByDepth.forEach(container => {
        const depth = getContainerDepth(currentVisualizationState, container.id);
      });
      
      // Collapse in depth order (deepest first)
      containersByDepth.forEach(container => {
        if (!container.collapsed) {
          currentVisualizationState.collapseContainer(container.id);
        }
      });
      
      // CRITICAL: Trigger layout refresh after collapsing all containers
      if (flowGraphRef.current && flowGraphRef.current.refreshLayout) {
        await flowGraphRef.current.refreshLayout();
      } else {
        // Fallback: force component update if refreshLayout not available
        forceUpdate();
      }
      
      // Trigger auto-fit after layout completes
      if (autoFit && flowGraphRef.current && flowGraphRef.current.fitView) {
        setTimeout(() => {
          flowGraphRef.current.fitView();
        }, 700); // Wait a bit longer for collapse to complete
      }
      
    } catch (err) {
      console.error('❌ Error packing containers:', err);
      setError(`Failed to pack containers: ${err.message}`);
    } finally {
      // Add a delay to let the layout complete
      setTimeout(() => setIsLayoutRunning(false), 1500);
    }
  }, [currentVisualizationState, autoFit]);

  // Unpack all containers (expand all)
  const handleUnpackAll = React.useCallback(async () => {
    if (!currentVisualizationState) return;
    
    setIsLayoutRunning(true);
    
    try {
      // Get ALL containers and expand them all (not just root containers)
      // This ensures every collapsed container gets expanded, regardless of hierarchy
      const allContainers = Array.from(currentVisualizationState.containers.values());
      
      // Expand all collapsed containers individually
      allContainers.forEach(container => {
        if (container.collapsed) {
          console.log('Expanding container:', container.id);
          currentVisualizationState.expandContainer(container.id);
        }
      });
      
      // CRITICAL: Trigger layout refresh after expanding all containers
      if (flowGraphRef.current && flowGraphRef.current.refreshLayout) {
        await flowGraphRef.current.refreshLayout();
      } else {
        // Fallback: force component update if refreshLayout not available
        forceUpdate();
      }
      
      // Trigger auto-fit after layout completes
      if (autoFit && flowGraphRef.current && flowGraphRef.current.fitView) {
        setTimeout(() => {
          flowGraphRef.current.fitView();
        }, 700); // Wait a bit longer for expand to complete
      }
    } catch (err) {
      console.error('❌ Error unpacking containers:', err);
      setError(`Failed to unpack containers: ${err.message}`);
    } finally {
      // Add a delay to let the layout complete
      setTimeout(() => setIsLayoutRunning(false), 1500);
    }
  }, [currentVisualizationState, autoFit]);

  // Fit view handler
  const handleFitView = React.useCallback(() => {
    if (flowGraphRef.current && flowGraphRef.current.fitView) {
      flowGraphRef.current.fitView();
    } else {
      console.warn('⚠️ FlowGraph ref not available, using fallback method');
      // Fallback to the old toggle method
      setAutoFit(false);
      setTimeout(() => setAutoFit(true), 100);
    }
  }, []);

  // Layout algorithm change handler
  const handleLayoutAlgorithmChange = React.useCallback((newAlgorithm) => {
    setLayoutAlgorithm(newAlgorithm);
    // Note: This will trigger a re-render which should cause FlowGraph to re-layout
    // The actual layout change will be handled by FlowGraph's props change detection
  }, []);

  // Dynamic canvas sizing based on window dimensions
  const [canvasHeight, setCanvasHeight] = React.useState(600);
  
  React.useEffect(() => {
    const updateCanvasSize = () => {
      const windowHeight = window.innerHeight;
      const windowWidth = window.innerWidth;
      
      // Calculate height based on window size - account for page padding (24px total) 
      // header text (~60px), controls, and margins (~80px total)
      let newHeight = windowHeight - 160; 
      
      // Minimum height for usability
      const minHeight = 400;
      newHeight = Math.max(newHeight, minHeight);
      
      setCanvasHeight(newHeight);
    };
    
    const handleResize = () => {
      updateCanvasSize();
      
      // Also trigger auto-fit directly on resize if enabled
      if (autoFit && currentVisualizationState) {
        setTimeout(() => {
          if (flowGraphRef.current && flowGraphRef.current.fitView) {
            flowGraphRef.current.fitView();
          }
        }, 300); // Delay to let canvas resize complete
      }
    };
    
    // Initial calculation
    updateCanvasSize();
    
    // Update on window resize
    window.addEventListener('resize', handleResize);
    
    return () => window.removeEventListener('resize', handleResize);
  }, [autoFit, currentVisualizationState]);

  // Auto-fit when canvas size changes (secondary trigger)
  React.useEffect(() => {
    if (autoFit && currentVisualizationState) {
      // Add a delay to let the DOM update with the new size
      setTimeout(() => {
        if (flowGraphRef.current && flowGraphRef.current.fitView) {
          flowGraphRef.current.fitView();
        } else {
          // Fallback: toggle autoFit to trigger re-fit
          setAutoFit(false);
          setTimeout(() => setAutoFit(true), 50);
        }
      }, 100);
    }
  }, [canvasHeight]); // Simplified dependencies

  // Color palette change handler
  const handleColorPaletteChange = React.useCallback((newPalette) => {
    setColorPalette(newPalette);
    // Note: This will trigger a re-render which should update node colors
  }, []);

  // Grouping change handler - this will re-parse the data with the new grouping
  const handleGroupingChange = React.useCallback((newGrouping) => {
    if (!parseGraphJSON || !createRenderConfig || !graphData || !createVisualizationState) return;
    
    setIsChangingGrouping(true);
    setCurrentGrouping(newGrouping);
    
    try {
      // Re-parse the original graph data with the new grouping
      const parsedData = parseGraphJSON(graphData, newGrouping);
      const renderConfig = createRenderConfig(parsedData);
      setCurrentVisualizationState(parsedData.state);
      setEdgeStyleConfig(renderConfig);
    } catch (err) {
      console.error('❌ Error changing grouping:', err);
      setError(`Failed to change grouping: ${err.message}`);
    } finally {
      // Clear the grouping change loading state
      setTimeout(() => setIsChangingGrouping(false), 100);
    }
  }, [parseGraphJSON, createRenderConfig, graphData, createVisualizationState]);

  // Hierarchy tree toggle handler (for InfoPanel tree)
  const handleHierarchyToggle = React.useCallback(async (containerId) => {
    if (!currentVisualizationState) return;
    
    try {
      const container = currentVisualizationState.getContainer(containerId);
      if (container) {
        const wasCollapsed = container.collapsed;
        if (wasCollapsed) {
          currentVisualizationState.expandContainer(containerId);
        } else {
          currentVisualizationState.collapseContainer(containerId);
        }
        
        // CRITICAL: Trigger layout refresh after container state change
        if (flowGraphRef.current && flowGraphRef.current.refreshLayout) {
          await flowGraphRef.current.refreshLayout();
        } else {
          // Fallback: force component update if refreshLayout not available
          forceUpdate();
        }
      }
    } catch (err) {
      console.error('❌ Error toggling hierarchy:', err);
      setError(`Failed to toggle hierarchy: ${err.message}`);
    }
  }, [currentVisualizationState]);

  // Render loading state
  if (loading) {
    return (
      <div style={{ 
        display: 'flex', 
        justifyContent: 'center', 
        alignItems: 'center', 
        height: '400px',
        fontSize: '18px',
        color: '#666'
      }}>
        Loading hydroscope components...
      </div>
    );
  }

  // Render error state
  if (error) {
    return (
      <div style={{ 
        padding: '24px',
        backgroundColor: '#ffebee',
        border: '1px solid #f44336',
        borderRadius: '8px',
        margin: '24px',
        color: '#d32f2f'
      }}>
        <h3 style={{ margin: '0 0 12px 0', color: '#d32f2f' }}>Error</h3>
        <p style={{ margin: 0 }}>{error}</p>
        <button 
          onClick={() => window.location.reload()}
          style={{
            marginTop: '16px',
            padding: '8px 16px',
            backgroundColor: '#f44336',
            color: 'white',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer'
          }}
        >
          Reload Page
        </button>
      </div>
    );
  }

  // Render main interface
  return (
    <div style={{ 
      padding: '20px', // Increased padding for better spacing
      maxWidth: 'none', // Remove max width constraint 
      margin: '0 20px 40px 20px', // Add left/right margins and bottom margin
      width: 'calc(100vw - 80px)', // Full width minus margins (20px * 2 on each side)
      minHeight: '100vh', // Use minimum height instead of fixed height
      overflow: 'auto', // Enable scrolling
      boxSizing: 'border-box'
    }}>
      <div style={{ marginBottom: '16px' }}> {/* Reduced margin */}
        <h1 style={{ margin: '0 0 4px 0', fontSize: TYPOGRAPHY.PAGE_TITLE }}>Graph Visualizer</h1> {/* Using constant */}
        <p style={{ margin: '0 0 8px 0', color: '#666', fontSize: TYPOGRAPHY.PAGE_SUBTITLE }}> {/* Using constant */}
          A hierarchical graph visualization tool for complex graphs, originally written for the <a href="https://hydro.run">Hydro project</a>.
        </p>
      </div>

      {/* Controls */}
      {currentVisualizationState && LayoutControls && (
        <div style={{ marginBottom: '8px', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}> {/* Reduced margin, flex layout */}
          <LayoutControls
            visualizationState={currentVisualizationState}
            currentLayout={layoutAlgorithm}
            onLayoutChange={handleLayoutAlgorithmChange}
            autoFit={autoFit}
            onAutoFitToggle={setAutoFit}
            onCollapseAll={handlePackAll}
            onExpandAll={handleUnpackAll}
            onFitView={handleFitView}
          />
          
          {/* Action Buttons */}
          <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
            <button
              onClick={() => {
                // Clear React state
                setCurrentVisualizationState(null);
                setGraphData(null);
                setError(null);
                setGeneratedFilePath(null);
                setGroupingOptions([]);
                setCurrentGrouping(null);
                
                // Clear URL parameters and reload to ensure clean state
                const url = new URL(window.location);
                url.search = ''; // Clear query parameters
                url.hash = '';   // Clear hash parameters
                window.history.replaceState({}, '', url.toString());
                
                // Reload to ensure completely clean state
                window.location.reload();
              }}
              style={{
                padding: '6px 12px',
                backgroundColor: '#6c757d',
                color: 'white',
                border: 'none',
                borderRadius: '4px',
                cursor: 'pointer',
                fontSize: '14px'
              }}
            >
              Clear Graph
            </button>
          </div>
        </div>
      )}      {isChangingGrouping ? (
        <div style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          height: '400px',
          border: '1px solid #ddd',
          borderRadius: '8px',
          backgroundColor: 'white'
        }}>
          <div style={{ textAlign: 'center', color: '#666' }}>
            <div style={{ 
              width: '40px',
              height: '40px',
              margin: '0 auto 16px',
              border: '4px solid #f3f3f3',
              borderTop: '4px solid #3498db',
              borderRadius: '50%',
              animation: 'groupingSpin 1s linear infinite'
            }}></div>
            <div style={{ fontSize: '18px', marginBottom: '8px' }}>
              Applying New Grouping...
            </div>
            <div style={{ fontSize: '14px', color: '#999' }}>
              Restructuring the graph hierarchy
            </div>
          </div>
          <style>
            {`
              @keyframes groupingSpin {
                0% { transform: rotate(0deg); }
                100% { transform: rotate(360deg); }
              }
            `}
          </style>
        </div>
      ) : (!currentVisualizationState && !isChangingGrouping) ? (
        FileDropZone ? (
          <FileDropZone 
            onFileLoad={handleFileLoad}
            hasData={!!currentVisualizationState}
            className="vis-file-drop"
            generatedFilePath={generatedFilePath}
          />
        ) : (
          <div style={{
            border: '2px dashed #ccc',
            borderRadius: '8px',
            padding: '48px',
            textAlign: 'center',
            backgroundColor: '#fafafa'
          }}>
            <p>Loading file upload component...</p>
          </div>
        )
      ) : (
        <div style={{ marginBottom: '24px' }}> {/* Increased margin for better spacing */}
          <div 
            style={{
              width: '100%',
              height: `${canvasHeight}px`,
              border: '1px solid #ddd',
              borderRadius: '8px',
              backgroundColor: 'white',
              position: 'relative',
              display: 'flex', // Use flexbox layout
              overflow: 'hidden'
            }}
          >
            {/* InfoPanel - fixed width sidebar */}
            {InfoPanel && currentVisualizationState && (
              <div style={{
                width: '300px',
                height: '100%',
                borderRight: '1px solid #eee',
                overflow: 'auto',
                flexShrink: 0
              }}>
                <InfoPanel
                  visualizationState={currentVisualizationState}
                  legendData={graphData && graphData.legend ? graphData.legend : {}}
                  edgeStyleConfig={graphData && graphData.edgeStyleConfig ? graphData.edgeStyleConfig : null}
                  hierarchyChoices={Array.isArray(groupingOptions) ? groupingOptions : []}
                  currentGrouping={typeof currentGrouping === 'string' ? currentGrouping : null}
                  onGroupingChange={handleGroupingChange}
                  onToggleContainer={handleHierarchyToggle}
                  collapsedContainers={new Set(currentVisualizationState.visibleContainers
                    .filter(container => container.collapsed)
                    .map(container => container.id))}
                  colorPalette={colorPalette}
                />
              </div>
            )}
            
            {/* FlowGraph - takes remaining space */}
            <div style={{ 
              flex: 1,
              height: '100%',
              width: '100%',
              position: 'relative',
              overflow: 'hidden'
            }}>
              {FlowGraph && (() => {
                const fullConfig = {
                  fitView: autoFit,
                  colorPalette: colorPalette,
                  ...edgeStyleConfig,
                  edgeStyle: styleConfig.edgeStyle,
                  edgeColor: styleConfig.edgeColor,
                  edgeWidth: styleConfig.edgeWidth,
                  edgeDashed: styleConfig.edgeDashed,
                  nodeBorderRadius: styleConfig.nodeBorderRadius,
                  nodePadding: styleConfig.nodePadding,
                  nodeFontSize: styleConfig.nodeFontSize,
                  containerBorderRadius: styleConfig.containerBorderRadius,
                  containerBorderWidth: styleConfig.containerBorderWidth,
                  containerShadow: styleConfig.containerShadow
                };
                return (
                  <FlowGraph 
                  ref={flowGraphRef}
                  visualizationState={currentVisualizationState}
                  layoutConfig={{ 
                    algorithm: layoutAlgorithm,
                    enableSmartCollapse: true 
                  }}
                  eventHandlers={{ 
                    onNodeClick: handleNodeClick 
                  }}
                  config={fullConfig}
                  // onLayoutComplete={() => console.log('Layout complete!')}
                  onError={(err) => {
                    console.error('Visualization error:', err);
                    setError(`Visualization error: ${err.message}`);
                  }}
                  style={{ 
                    width: '100%', 
                    height: '100%'
                  }}
                />
                );
              })()}
            </div>

            {/* Style Tuner Panel */}
            {StyleTunerPanel && (
              <div style={{ position: 'absolute', top: 12, right: 12, zIndex: 1500, width: 320 }}>
                <StyleTunerPanel 
                  value={styleConfig} 
                  onChange={setStyleConfig} 
                  colorPalette={colorPalette}
                  onPaletteChange={handleColorPaletteChange}
                />
              </div>
            )}
            
            {/* Layout operation loading overlay */}
            {isLayoutRunning && (
              <div style={{
                position: 'absolute',
                top: 0,
                left: 0,
                right: 0,
                bottom: 0,
                backgroundColor: 'rgba(255, 255, 255, 0.8)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                zIndex: 2000,
                borderRadius: '8px'
              }}>
                <div style={{ textAlign: 'center', color: '#333' }}>
                  <div style={{ 
                    width: '32px',
                    height: '32px',
                    margin: '0 auto 12px',
                    border: '3px solid #f3f3f3',
                    borderTop: '3px solid #3498db',
                    borderRadius: '50%',
                    animation: 'canvasSpin 1s linear infinite'
                  }}></div>
                  <div style={{ fontSize: '16px', fontWeight: 'bold', marginBottom: '4px' }}>
                    Updating Layout...
                  </div>
                  <div style={{ fontSize: '12px', color: '#666' }}>
                    Complex graphs may take a moment
                  </div>
                </div>
                <style>
                  {`
                    @keyframes canvasSpin {
                      0% { transform: rotate(0deg); }
                      100% { transform: rotate(360deg); }
                    }
                  `}
                </style>
              </div>
            )}
          </div>
        </div>
      )}
      
      {/* Version number and attribution displayed underneath canvas */}
      <div style={{ 
        textAlign: 'center', 
        marginTop: '16px', 
        fontSize: '12px', 
        color: '#999' 
      }}>
        <div>HyGraphViz v0.4</div>
      </div>
    </div>
  );
}

export default function VisPage() {
  return (
    <Layout
      title="Hyrarchical Graph Visualizer"
      description="A hierarchical graph visualization tool for complex graphs, built for the Hydro project.">
      <main>
        <BrowserOnly fallback={<div>Loading...</div>}>
          {() => <HydroscopeComponent />}
        </BrowserOnly>
      </main>
    </Layout>
  );
}
