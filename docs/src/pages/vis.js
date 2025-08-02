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
  const [GraphFlow, setGraphFlow] = React.useState(null);
  const [createVisualizationState, setCreateVisualizationState] = React.useState(null);
  const [parseGraphJSON, setParseGraphJSON] = React.useState(null);
  const [FileDropZone, setFileDropZone] = React.useState(null);
  const [InfoPanel, setInfoPanel] = React.useState(null);
  const [NODE_STYLES, setNodeStyles] = React.useState(null);
  const [EDGE_STYLES, setEdgeStyles] = React.useState(null);
  const [error, setError] = React.useState(null);
  const [visualizationState, setVisualizationState] = React.useState(null);
  const [parseMetadata, setParseMetadata] = React.useState(null);
  // Helper function to create default legend data
  const createDefaultLegendData = () => {
    if (!visualizationState) return { title: "Legend", items: [] };
    
    // Get the node type config from parse metadata if available
    const nodeTypeConfig = parseMetadata?.nodeTypeConfig;
    
    // Collect all unique node types from the visualization state
    const nodeTypes = new Set();
    if (visualizationState.visibleNodes) {
      visualizationState.visibleNodes.forEach(node => {
        const nodeType = node.nodeType || node.data?.nodeType || 'Transform';
        if (nodeType) {
          nodeTypes.add(nodeType);
        }
      });
    }
    
    // Create legend items based on nodeTypeConfig if available, otherwise use defaults
    let legendItems = [];
    if (nodeTypeConfig?.types && Array.isArray(nodeTypeConfig.types)) {
      // Use the types from nodeTypeConfig that are actually present in the graph
      legendItems = nodeTypeConfig.types
        .filter(typeConfig => nodeTypes.has(typeConfig.id))
        .map(typeConfig => ({
          type: typeConfig.id,
          label: typeConfig.label || typeConfig.id
        }));
    } else {
      // Fallback to just the node types we found
      legendItems = Array.from(nodeTypes).map(type => ({
        type: type,
        label: type.charAt(0).toUpperCase() + type.slice(1)
      }));
    }
    
    return {
      title: "Node Types",
      items: legendItems
    };
  };

  // State for collapsed containers
  const [collapsedContainers, setCollapsedContainers] = React.useState(new Set());

  React.useEffect(() => {
    // Dynamically import the visualization components
    const loadComponents = async () => {
      try {
        const visStateModule = await import('../components/vis/core/VisState.ts');
        const graphFlowModule = await import('../components/vis/render/GraphFlow.tsx');
        const constantsModule = await import('../components/vis/shared/constants.ts');
        const parserModule = await import('../components/vis/core/JSONParser.ts');
        const componentsModule = await import('../components/vis/components/index.ts');
        
        setCreateVisualizationState(() => visStateModule.createVisualizationState);
        setGraphFlow(() => graphFlowModule.GraphFlow);
        setParseGraphJSON(() => parserModule.parseGraphJSON);
        setFileDropZone(() => componentsModule.FileDropZone);
        setInfoPanel(() => componentsModule.InfoPanel);
        setNodeStyles(constantsModule.NODE_STYLES);
        setEdgeStyles(constantsModule.EDGE_STYLES);
      } catch (err) {
        console.error('Failed to load visualization components:', err);
        setError(err.message);
      }
    };

    loadComponents();
  }, []);

  const handleFileLoad = React.useCallback((jsonData) => {
    try {
      console.log('Loading JSON data:', jsonData);
      
      // Parse the JSON data into a VisualizationState
      const parseResult = parseGraphJSON(jsonData);
      console.log('Parse result:', parseResult);
      
      setVisualizationState(parseResult.state);
      setParseMetadata(parseResult.metadata);
      setCollapsedContainers(new Set());
      setError(null);
    } catch (err) {
      console.error('Error parsing JSON:', err);
      setError('Failed to parse JSON data: ' + err.message);
    }
  }, [parseGraphJSON]);

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
    console.log('Grouping changed to:', groupingId);
    // In a real implementation, this would re-parse the data with the new grouping
    // For now, just update the metadata
    if (parseMetadata) {
      setParseMetadata({
        ...parseMetadata,
        selectedGrouping: groupingId
      });
    }
  }, [parseMetadata]);

  const createSampleGraph = React.useCallback(() => {
    if (!createVisualizationState || !NODE_STYLES || !EDGE_STYLES) return;
    
    // Create a sample demonstration graph with containers
    const sampleState = createVisualizationState();
    
    // Add sample nodes with different styles (using professional node types)
    sampleState.setGraphNode('source', { 
      label: 'Data Source', 
      style: NODE_STYLES.DEFAULT,
      nodeType: 'Source'
    });
    sampleState.setGraphNode('transform', { 
      label: 'Transform', 
      style: NODE_STYLES.HIGHLIGHTED,
      nodeType: 'Transform'
    });
    sampleState.setGraphNode('join', { 
      label: 'Join', 
      style: NODE_STYLES.DEFAULT,
      nodeType: 'Join'
    });
    sampleState.setGraphNode('filter', { 
      label: 'Filter', 
      style: NODE_STYLES.WARNING,
      nodeType: 'Filter'
    });
    sampleState.setGraphNode('sink', { 
      label: 'Data Sink', 
      style: NODE_STYLES.DEFAULT,
      nodeType: 'Sink'
    });
    sampleState.setGraphNode('error_handler', { 
      label: 'Error Handler', 
      style: NODE_STYLES.ERROR,
      nodeType: 'Operator'
    });
    
    // Add sample containers to demonstrate hierarchy
    sampleState.setContainer('processing_group', {
      expandedDimensions: { width: 300, height: 200 },
      collapsed: false,
      children: ['transform', 'join']
    });
    sampleState.setContainer('output_group', {
      expandedDimensions: { width: 250, height: 150 },
      collapsed: false,
      children: ['filter', 'sink']
    });
    
    // Add sample edges with different styles
    sampleState.setGraphEdge('edge1', { 
      source: 'source', 
      target: 'transform',
      style: EDGE_STYLES.DEFAULT
    });
    sampleState.setGraphEdge('edge2', { 
      source: 'transform', 
      target: 'join',
      style: EDGE_STYLES.THICK
    });
    sampleState.setGraphEdge('edge3', { 
      source: 'join', 
      target: 'filter',
      style: EDGE_STYLES.DEFAULT
    });
    sampleState.setGraphEdge('edge4', { 
      source: 'filter', 
      target: 'sink',
      style: EDGE_STYLES.DEFAULT
    });
    sampleState.setGraphEdge('edge5', { 
      source: 'transform', 
      target: 'error_handler',
      style: EDGE_STYLES.DASHED
    });
    
    setVisualizationState(sampleState);
    setParseMetadata({
      selectedGrouping: 'sample_grouping',
      nodeCount: 6,
      edgeCount: 5,
      containerCount: 2,
      availableGroupings: [
        { id: 'sample_grouping', name: 'Processing Groups' }
      ]
    });
    setCollapsedContainers(new Set());
  }, [createVisualizationState, NODE_STYLES, EDGE_STYLES]);

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

  if (!GraphFlow || !createVisualizationState || !NODE_STYLES || !EDGE_STYLES || !FileDropZone || !parseGraphJSON || !InfoPanel) {
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
    <div style={{ minHeight: '100vh', padding: '40px 20px' }}>
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
          
          <div style={{ marginBottom: '32px' }}>
            <button 
              onClick={createSampleGraph}
              style={{
                padding: '12px 24px',
                backgroundColor: '#007acc',
                color: 'white',
                border: 'none',
                borderRadius: '6px',
                cursor: 'pointer',
                fontSize: '14px',
                marginRight: '16px'
              }}
            >
              Try Sample
            </button>
            <span style={{ color: '#999' }}>or drag a JSON file below</span>
          </div>

          <FileDropZone onFileLoad={handleFileLoad} />
        </div>
      ) : (
        // Visualization Section
        <div style={{ maxWidth: '1400px', margin: '0 auto', position: 'relative' }}>
          {/* Simple Controls */}
          <div style={{ 
            display: 'flex', 
            justifyContent: 'space-between', 
            alignItems: 'center',
            marginBottom: '20px',
            padding: '12px 0',
            borderBottom: '1px solid #eee'
          }}>
            <div style={{ fontSize: '14px', color: '#666' }}>
              {parseMetadata && (
                <>
                  {parseMetadata.nodeCount} nodes, {parseMetadata.edgeCount} edges
                  {parseMetadata.containerCount > 0 && `, ${parseMetadata.containerCount} containers`}
                  {parseMetadata.selectedGrouping && ` (${parseMetadata.selectedGrouping})`}
                </>
              )}
            </div>
            <button 
              onClick={() => {
                setVisualizationState(null);
                setParseMetadata(null);
                setCollapsedContainers(new Set());
                setError(null);
              }}
              style={{
                padding: '6px 12px',
                backgroundColor: '#f5f5f5',
                color: '#666',
                border: '1px solid #ddd',
                borderRadius: '4px',
                cursor: 'pointer',
                fontSize: '12px'
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
            <GraphFlow 
              visualizationState={visualizationState}
              metadata={parseMetadata}
              collapsedContainers={collapsedContainers}
              onToggleContainer={handleToggleContainer}
              onLayoutComplete={() => console.log('Layout complete!')}
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
                  legendData={createDefaultLegendData()}
                  hierarchyChoices={parseMetadata?.availableGroupings || []}
                  currentGrouping={parseMetadata?.selectedGrouping}
                  onGroupingChange={handleGroupingChange}
                  collapsedContainers={collapsedContainers}
                  onToggleContainer={handleToggleContainer}
                  colorPalette="Set3"
                  onPositionChange={(panelId, position) => {
                    console.log(`Panel ${panelId} moved to ${position}`);
                  }}
                />
              </div>
            </div>
            
            {/* Zoom Controls */}
            <div style={{ 
              position: 'absolute', 
              bottom: '16px', 
              left: '16px',
              pointerEvents: 'auto',
              display: 'flex',
              flexDirection: 'column',
              gap: '8px'
            }}>
              <button
                onClick={() => {
                  // TODO: Implement zoom in functionality
                  console.log('Zoom in');
                }}
                style={{
                  width: '40px',
                  height: '40px',
                  backgroundColor: '#fff',
                  border: '1px solid #ddd',
                  borderRadius: '6px',
                  cursor: 'pointer',
                  fontSize: '18px',
                  fontWeight: 'bold',
                  color: '#333',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  boxShadow: '0 2px 4px rgba(0,0,0,0.1)',
                  transition: 'all 0.2s ease'
                }}
                onMouseEnter={(e) => {
                  e.target.style.backgroundColor = '#f5f5f5';
                  e.target.style.boxShadow = '0 4px 8px rgba(0,0,0,0.15)';
                }}
                onMouseLeave={(e) => {
                  e.target.style.backgroundColor = '#fff';
                  e.target.style.boxShadow = '0 2px 4px rgba(0,0,0,0.1)';
                }}
              >
                +
              </button>
              <button
                onClick={() => {
                  // TODO: Implement zoom out functionality
                  console.log('Zoom out');
                }}
                style={{
                  width: '40px',
                  height: '40px',
                  backgroundColor: '#fff',
                  border: '1px solid #ddd',
                  borderRadius: '6px',
                  cursor: 'pointer',
                  fontSize: '18px',
                  fontWeight: 'bold',
                  color: '#333',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  boxShadow: '0 2px 4px rgba(0,0,0,0.1)',
                  transition: 'all 0.2s ease'
                }}
                onMouseEnter={(e) => {
                  e.target.style.backgroundColor = '#f5f5f5';
                  e.target.style.boxShadow = '0 4px 8px rgba(0,0,0,0.15)';
                }}
                onMouseLeave={(e) => {
                  e.target.style.backgroundColor = '#fff';
                  e.target.style.boxShadow = '0 2px 4px rgba(0,0,0,0.1)';
                }}
              >
                −
              </button>
              <button
                onClick={() => {
                  // TODO: Implement fit to view functionality
                  console.log('Fit to view');
                }}
                style={{
                  width: '40px',
                  height: '40px',
                  backgroundColor: '#fff',
                  border: '1px solid #ddd',
                  borderRadius: '6px',
                  cursor: 'pointer',
                  fontSize: '12px',
                  fontWeight: 'bold',
                  color: '#333',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  boxShadow: '0 2px 4px rgba(0,0,0,0.1)',
                  transition: 'all 0.2s ease'
                }}
                onMouseEnter={(e) => {
                  e.target.style.backgroundColor = '#f5f5f5';
                  e.target.style.boxShadow = '0 4px 8px rgba(0,0,0,0.15)';
                }}
                onMouseLeave={(e) => {
                  e.target.style.backgroundColor = '#fff';
                  e.target.style.boxShadow = '0 2px 4px rgba(0,0,0,0.1)';
                }}
                title="Fit to view"
              >
                ⌂
              </button>
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
            • Drag the graph to pan • Scroll to zoom • Click containers in the hierarchy tree to collapse/expand them
            • Drag panel headers to reposition • Click the ▼ button to collapse panels
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
