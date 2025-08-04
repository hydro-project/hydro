/**
 * Vis System Homepage
 * 
 * Main entry point for the new framework-independent graph visualization system.
 * Features file upload, JSON parsing, and ReactFlow v12 + ELK layout visualization.
 */

import React from 'react';
import Layout from '@theme/Layout';
import BrowserOnly from '@docusaurus/BrowserOnly';

function VisHomepageComponent() {
  const [FlowGraph, setFlowGraph] = React.useState(null);
  const [createVisualizationState, setCreateVisualizationState] = React.useState(null);
  const [parseGraphJSON, setParseGraphJSON] = React.useState(null);
  const [FileDropZone, setFileDropZone] = React.useState(null);
  const [InfoPanel, setInfoPanel] = React.useState(null);
  const [LayoutControls, setLayoutControls] = React.useState(null);
  const [createDefaultLegendData, setCreateDefaultLegendData] = React.useState(null);
  const [NODE_STYLES, setNodeStyles] = React.useState(null);
  const [EDGE_STYLES, setEdgeStyles] = React.useState(null);
  const [error, setError] = React.useState(null);
  const [visualizationState, setVisualizationState] = React.useState(null);
  const [parseMetadata, setParseMetadata] = React.useState(null);
  const [originalJsonData, setOriginalJsonData] = React.useState(null); // Store original data for re-parsing
  const [visStateVersion, setVisStateVersion] = React.useState(0); // Track VisState mutations for re-rendering
  const [flowGraphKey, setFlowGraphKey] = React.useState(0); // Force fresh FlowGraph instance for resets
  
    // Layout and color states
  const [currentLayout, setCurrentLayout] = React.useState('mrtree');
  const [colorPalette, setColorPalette] = React.useState('Set3');
  const [autoFit, setAutoFit] = React.useState(false);
  // Generate legend data based on node types present in the visualization state
  const generateLegendData = () => {
    // Collect all unique node types from the visualization state
    const nodeTypes = new Set();
    if (visualizationState?.visibleNodes) {
      visualizationState.visibleNodes.forEach(node => {
        const nodeType = node.nodeType || node.data?.nodeType || 'Transform';
        if (nodeType) {
          nodeTypes.add(nodeType);
        }
      });
    }
    
    // Get nodeTypeConfig from parseMetadata
    const nodeTypeConfig = parseMetadata?.nodeTypeConfig;
    
    // Create legend items based on nodeTypeConfig if available, otherwise use defaults
    let legendItems = [];
    if (nodeTypeConfig?.types && Array.isArray(nodeTypeConfig.types) && nodeTypes.size > 0) {
      // Use the types from nodeTypeConfig that are actually present in the graph
      legendItems = nodeTypeConfig.types
        .filter(typeConfig => nodeTypes.has(typeConfig.id))
        .map(typeConfig => ({
          type: typeConfig.id,
          label: typeConfig.label || typeConfig.id
        }));
    } else if (nodeTypes.size > 0) {
      // Enhanced fallback with better descriptions for Hydro node types
      const typeDescriptions = {
        'Transform': 'Data transformation operations',
        'Tee': 'Data splitting operations', 
        'Sink': 'Data output destinations',
        'Network': 'Network communication nodes',
        'Source': 'Data input sources',
        'Join': 'Data joining operations',
        'Aggregation': 'Data aggregation operations'
      };
      
      legendItems = Array.from(nodeTypes).map(type => ({
        type: type,
        label: type.charAt(0).toUpperCase() + type.slice(1),
        description: typeDescriptions[type] || `${type} operations`
      }));
    } else {
      // Default legend when no data is loaded
      legendItems = [
        { type: 'Transform', label: 'Transform', description: 'Data transformation operations' },
        { type: 'Tee', label: 'Tee', description: 'Data splitting operations' },
        { type: 'Sink', label: 'Sink', description: 'Data output destinations' },
        { type: 'Network', label: 'Network', description: 'Network communication nodes' }
      ];
    }
    
    return {
      title: "Node Types",
      items: legendItems
    };
  };  // State for collapsed containers
  const [collapsedContainers, setCollapsedContainers] = React.useState(new Set());

  React.useEffect(() => {
    // Dynamically import the visualization components
    const loadComponents = async () => {
      try {
        const visStateModule = await import('../components/vis/core/VisState.ts');
        const FlowGraphModule = await import('../components/vis/render/FlowGraph.tsx');
        const constantsModule = await import('../components/vis/shared/constants.ts');
        const parserModule = await import('../components/vis/core/JSONParser.ts');
        const componentsModule = await import('../components/vis/components/index.ts');
        
        setCreateVisualizationState(() => visStateModule.createVisualizationState);
        setFlowGraph(() => FlowGraphModule.FlowGraph);
        setParseGraphJSON(() => parserModule.parseGraphJSON);
        setFileDropZone(() => componentsModule.FileDropZone);
        setInfoPanel(() => componentsModule.InfoPanel);
        setLayoutControls(() => componentsModule.LayoutControls);
        setCreateDefaultLegendData(() => componentsModule.createDefaultLegendData);
        setNodeStyles(constantsModule.NODE_STYLES);
        setEdgeStyles(constantsModule.EDGE_STYLES);
      } catch (err) {
        console.error('Failed to load visualization components:', err);
        setError(err.message);
      }
    };

    loadComponents();
  }, []);

  // Synchronous resetAll function - completely reinitializes everything
  const resetAll = React.useCallback((jsonData, groupingId = null) => {
    console.log('[HomePage] 🔄 Starting complete reset with groupingId:', groupingId);
    
    try {
      // 1. Parse JSON with new grouping to get fresh VisualizationState
      const parseResult = parseGraphJSON(jsonData, groupingId);
      
      // 2. Force fresh FlowGraph instance (new ELK, new ReactFlow, new VisState)
      const newKey = Date.now(); // Use timestamp for unique key
      console.log('[HomePage] 🔑 Setting new flowGraphKey:', newKey);
      setFlowGraphKey(newKey);
      
      // 3. Update all state atomically
      setVisualizationState(parseResult.state);
      setParseMetadata(parseResult.metadata);
      setCollapsedContainers(new Set()); // Reset collapsed state
      setError(null);
      
      console.log('[HomePage] ✅ Complete reset successful');
      return parseResult;
    } catch (err) {
      console.error('[HomePage] ❌ Error during complete reset:', err);
      setError('Failed to reset visualization: ' + err.message);
      throw err;
    }
  }, [parseGraphJSON]);

  const handleFileLoad = React.useCallback((jsonData) => {
    try {
      // Store the original JSON data for re-parsing with different groupings
      setOriginalJsonData(jsonData);
      
      // Use resetAll for clean initialization
      resetAll(jsonData);
    } catch (err) {
      console.error('Error parsing JSON:', err);
      setError('Failed to parse JSON data: ' + err.message);
    }
  }, [resetAll]);

  const handleToggleContainer = React.useCallback((containerId) => {
    setCollapsedContainers(prev => {
      const newSet = new Set(prev);
      if (newSet.has(containerId)) {
        newSet.delete(containerId);
      } else {
        newSet.add(containerId);
      }
      return newSet;
    });
  }, []);

  const handleGroupingChange = React.useCallback((groupingId) => {
    console.log('[HomePage] 🔄 Grouping changed to:', groupingId);
    
    if (!originalJsonData) {
      console.log('[HomePage] ⚠️ No original JSON data available for re-parsing');
      return;
    }
    
    // Use resetAll for complete reinitialization
    resetAll(originalJsonData, groupingId);
  }, [originalJsonData, resetAll]);

  // Handle container click for collapse/expand
  const handleNodeClick = React.useCallback((event, node) => {
    console.log(`[HomePage] 🖱️ Node click received: ${node.id}, type: ${node.type}`);
    
    // Check if this is a container node that can be collapsed/expanded
    if (node.type === 'container' && visualizationState) {
      event.stopPropagation();
      
      try {
        // Find the container in the visualization state
        const container = visualizationState.getContainer(node.id);
        if (container) {
          console.log(`[HomePage] 🖱️ Container ${node.id} clicked, currently ${container.collapsed ? 'collapsed' : 'expanded'}`);
          
          // Toggle the container state
          if (container.collapsed) {
            visualizationState.expandContainer(node.id);
            console.log(`[HomePage] ↗️ Expanded container ${node.id}`);
          } else {
            visualizationState.collapseContainer(node.id);
            console.log(`[HomePage] ↙️ Collapsed container ${node.id}`);
          }
          
          // Notify React that VisState has changed
          setVisStateVersion(prev => prev + 1);
          
        } else {
          console.log(`[HomePage] ❌ Container ${node.id} not found in visualization state`);
        }
      } catch (error) {
        console.error('[HomePage] ❌ Error toggling container:', error);
      }
    } else {
      console.log(`[HomePage] ℹ️ Non-container node clicked: ${node.id} (type: ${node.type})`);
    }
  }, [visualizationState]);

  // Layout control handlers
  const handleLayoutChange = React.useCallback(async (layout) => {
    setCurrentLayout(layout);
    console.log('[HomePage] 🔧 Layout changed to:', layout);
    
    // The layoutConfig prop change will trigger re-layout automatically
    // No need to force a complete re-render which breaks drag functionality
    
    console.log('[HomePage] ✅ Layout change applied successfully');
  }, []);

  const handlePaletteChange = React.useCallback((palette) => {
    setColorPalette(palette);
    // TODO: Implement color palette change
    console.log('Color palette changed to:', palette);
  }, []);

  const handleCollapseAll = React.useCallback(() => {
    if (!visualizationState) return;
    
    try {
      // Collapse all expanded containers
      visualizationState.visibleContainers.forEach(container => {
        if (!container.collapsed) {
          visualizationState.collapseContainer(container.id);
        }
      });
      
      setVisStateVersion(prev => prev + 1);
      console.log('[HomePage] 📦 Collapsed all containers');
    } catch (error) {
      console.error('[HomePage] ❌ Error collapsing all containers:', error);
    }
  }, [visualizationState]);

  const handleExpandAll = React.useCallback(() => {
    if (!visualizationState) return;
    
    try {
      // Expand all collapsed containers
      visualizationState.visibleContainers.forEach(container => {
        if (container.collapsed) {
          visualizationState.expandContainer(container.id);
        }
      });
      
      setVisStateVersion(prev => prev + 1);
      console.log('[HomePage] 📤 Expanded all containers');
    } catch (error) {
      console.error('[HomePage] ❌ Error expanding all containers:', error);
    }
  }, [visualizationState]);

  const handleAutoFitToggle = React.useCallback((enabled) => {
    setAutoFit(enabled);
    console.log('[HomePage] 🔄 Auto fit toggled:', enabled);
  }, []);

  const handleFitView = React.useCallback(() => {
    console.log('[HomePage] 🎯 Fit view requested');
    // TODO: This should call a method on FlowGraph directly instead of forcing re-render
    // For now, trigger a re-render as a workaround
    setVisStateVersion(prev => prev + 1);
  }, []);

  const createSampleGraph = React.useCallback(() => {
    if (!createVisualizationState || !NODE_STYLES || !EDGE_STYLES) return;
    
    // Create mock JSON data for sample graph
    const sampleJsonData = {
      nodes: [
        { id: 'source', label: 'Data Source', nodeType: 'Source' },
        { id: 'transform', label: 'Transform', nodeType: 'Transform' },
        { id: 'join', label: 'Join', nodeType: 'Join' },
        { id: 'filter', label: 'Filter', nodeType: 'Filter' },
        { id: 'sink', label: 'Data Sink', nodeType: 'Sink' },
        { id: 'error_handler', label: 'Error Handler', nodeType: 'Operator' }
      ],
      edges: [
        { id: 'edge1', source: 'source', target: 'transform' },
        { id: 'edge2', source: 'transform', target: 'join' },
        { id: 'edge3', source: 'join', target: 'filter' },
        { id: 'edge4', source: 'filter', target: 'sink' },
        { id: 'edge5', source: 'transform', target: 'error_handler' }
      ],
      hierarchyChoices: [
        { id: 'sample_grouping', name: 'Processing Groups' },
        { id: 'functional_grouping', name: 'Functional Groups' }
      ],
      hierarchies: [
        {
          id: 'sample_grouping',
          name: 'Processing Groups',
          containers: [
            { 
              id: 'processing_group', 
              label: 'Processing Group',
              children: ['transform', 'join'] 
            },
            { 
              id: 'output_group', 
              label: 'Output Group', 
              children: ['filter', 'sink'] 
            }
          ]
        },
        {
          id: 'functional_grouping', 
          name: 'Functional Groups',
          containers: [
            { 
              id: 'input_output_group', 
              label: 'I/O Operations',
              children: ['source', 'sink'] 
            },
            { 
              id: 'processing_group', 
              label: 'Data Processing', 
              children: ['transform', 'join', 'filter'] 
            }
          ]
        }
      ]
    };
    
    // Store original data and use resetAll for clean initialization
    setOriginalJsonData(sampleJsonData);
    resetAll(sampleJsonData, 'sample_grouping');
  }, [createVisualizationState, NODE_STYLES, EDGE_STYLES, resetAll]);

  if (error) {
    return (
      <div style={{ padding: '40px 20px', textAlign: 'center' }}>
        <div style={{ 
          background: '#ffebee', 
          border: '1px solid #f44336', 
          color: '#c62828', 
          padding: '16px', 
          borderRadius: '4px',
          maxWidth: '600px',
          margin: '0 auto'
        }}>
          <strong>Error:</strong> {error}
          <br />
          <button 
            onClick={() => window.location.reload()}
            style={{
              marginTop: '12px',
              padding: '6px 12px',
              backgroundColor: '#f44336',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer',
              fontSize: '12px'
            }}
          >
            Reload
          </button>
        </div>
      </div>
    );
  }

  if (!FlowGraph || !createVisualizationState || !NODE_STYLES || !EDGE_STYLES || !FileDropZone || !parseGraphJSON || !InfoPanel || !LayoutControls) {
    return (
      <div style={{ 
        display: 'flex', 
        alignItems: 'center', 
        justifyContent: 'center', 
        minHeight: '400px',
        fontSize: '14px',
        color: '#999'
      }}>
        Loading visualization system...
      </div>
    );
  }

  return (
    <div style={{ minHeight: '100vh', padding: '10px 20px' }}>
      {!visualizationState ? (
        <div style={{ maxWidth: '800px', margin: '0 auto', textAlign: 'center' }}>
          <h1 style={{ 
            fontSize: '32px', 
            marginBottom: '16px',
            color: '#333'
          }}>
            Graph Visualizer
          </h1>
          <p style={{ 
            fontSize: '16px', 
            marginBottom: '40px',
            color: '#666'
          }}>
            Interactive graph visualization with ReactFlow and ELK layout
          </p>

          <FileDropZone onFileLoad={handleFileLoad} />
          
          <div style={{ marginTop: '20px' }}>
            <button
              onClick={createSampleGraph}
              style={{
                padding: '10px 20px',
                backgroundColor: '#007bff',
                color: 'white',
                border: 'none',
                borderRadius: '4px',
                cursor: 'pointer',
                fontSize: '14px'
              }}
            >
              Load Sample Data
            </button>
          </div>
        </div>
      ) : (
        // Visualization Section
        <div style={{ maxWidth: '1400px', margin: '0 auto', position: 'relative' }}>
          {/* Combined Controls Bar */}
          <div style={{ 
            display: 'flex', 
            justifyContent: 'space-between', 
            alignItems: 'center',
            marginBottom: '8px',
            padding: '8px',
            backgroundColor: '#f8f9fa',
            border: '1px solid #dee2e6',
            borderRadius: '6px',
            flexWrap: 'wrap',
            gap: '8px'
          }}>
            {/* Left: Graph Info */}
            <div style={{ fontSize: '14px', color: '#666', minWidth: '0', flex: '0 0 auto' }}>
              {parseMetadata && (
                <>
                  {parseMetadata.nodeCount} nodes, {parseMetadata.edgeCount} edges
                  {parseMetadata.containerCount > 0 && `, ${parseMetadata.containerCount} containers`}
                  {parseMetadata.selectedGrouping && ` (${parseMetadata.selectedGrouping})`}
                </>
              )}
            </div>
            
            {/* Center: Layout Controls */}
            <div style={{ flex: '1 1 auto', display: 'flex', justifyContent: 'center', minWidth: '0' }}>
              <LayoutControls
                visualizationState={visualizationState}
                currentLayout={currentLayout}
                onLayoutChange={handleLayoutChange}
                colorPalette={colorPalette}
                onPaletteChange={handlePaletteChange}
                onCollapseAll={handleCollapseAll}
                onExpandAll={handleExpandAll}
                autoFit={autoFit}
                onAutoFitToggle={handleAutoFitToggle}
                onFitView={handleFitView}
                style={{ 
                  backgroundColor: 'transparent',
                  border: 'none',
                  padding: '0'
                }}
              />
            </div>

            {/* Right: Reset Button */}
            <button 
              onClick={() => {
                setVisualizationState(null);
                setParseMetadata(null);
                setOriginalJsonData(null);
                setCollapsedContainers(new Set());
                setError(null);
                setFlowGraphKey(prev => prev + 1); // Fresh instance for next load
              }}
              style={{
                padding: '6px 12px',
                backgroundColor: '#6c757d',
                color: 'white',
                border: 'none',
                borderRadius: '4px',
                cursor: 'pointer',
                fontSize: '12px',
                flex: '0 0 auto'
              }}
            >
              Reset
            </button>
          </div>

          {/* Visualization Container */}
          <div style={{ 
            position: 'relative',
            height: '700px',
            border: '1px solid #ddd',
            borderRadius: '4px',
            overflow: 'hidden',
            backgroundColor: '#fafafa'
          }}>
            {/* Main Graph Visualization */}
            {console.log('[HomePage] 🔑 Rendering FlowGraph with key:', flowGraphKey)}
            <FlowGraph 
              key={flowGraphKey} // Force fresh instance for complete resets
              reactFlowKey={flowGraphKey} // Force ReactFlow internal reset too
              visualizationState={visualizationState}
              layoutConfig={{ algorithm: currentLayout }}
              metadata={parseMetadata}
              visStateVersion={visStateVersion}
              eventHandlers={{
                onNodeClick: handleNodeClick
              }}
              onLayoutComplete={() => {}}
              onError={(err) => {
                console.error('Visualization error:', err);
                setError('Visualization error: ' + err.message);
              }}
              style={{ width: '100%', height: '100%' }}
            />
            
            {/* InfoPanel Overlay */}
            <div style={{ position: 'absolute', top: 0, left: 0, right: 0, bottom: 0, pointerEvents: 'none' }}>
              <div style={{ pointerEvents: 'auto' }}>
                <InfoPanel
                  visualizationState={visualizationState}
                  legendData={generateLegendData()}
                  hierarchyChoices={parseMetadata?.availableGroupings || []}
                  currentGrouping={parseMetadata?.selectedGrouping}
                  onGroupingChange={handleGroupingChange}
                  collapsedContainers={collapsedContainers}
                  onToggleContainer={handleToggleContainer}
                  colorPalette="Set2"
                  onPositionChange={(panelId, position) => {
                    // Panel position changed
                  }}
                />
              </div>
            </div>
          </div>
          
          {/* Instructions */}
          <div style={{ 
            marginTop: '16px',
            padding: '12px',
            backgroundColor: '#f8f9fa',
            borderRadius: '4px',
            fontSize: '12px',
            color: '#666'
          }}>
            <strong>Instructions:</strong> 
            • Drag the graph to pan • Scroll to zoom • Use the controls in the bottom-left for zoom and fit-to-view
            • Click containers in the hierarchy tree to collapse/expand them • Drag panel headers to reposition • Click the ▼ button to collapse panels
          </div>
        </div>
      )}
    </div>
  );
}

export default function VisHomepage() {
  return (
    <Layout
      title="Graph Visualizer"
      description="Interactive graph visualization with ReactFlow v12 and ELK layout">
      <main>
        <BrowserOnly fallback={<div>Loading...</div>}>
          {() => <VisHomepageComponent />}
        </BrowserOnly>
      </main>
    </Layout>
  );
}
