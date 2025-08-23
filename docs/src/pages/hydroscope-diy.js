/**
 * Hydroscope DIY Demo - Custom Assembly with Individual Components
 * 
 * This page demonstrates how to build custom graph visualizations using individual 
 * components from the external hydroscope repository DIY toolkit:
 * - Basic Hydroscope component for rendering
 * - Individual UI components (LayoutControls, StyleTunerPanel, InfoPanel, etc.)
 * - Manual event handler setup
 * - Custom layout and styling
 */

import React from 'react';
import Layout from '@theme/Layout';
import BrowserOnly from '@docusaurus/BrowserOnly';
import { useLocation } from '@docusaurus/router';

// Import CSS from external hydroscope repo
import '@hydro-project/hydroscope/style.css';

// Typography constants for consistent styling
const TYPOGRAPHY = {
  PAGE_TITLE: '2.5em',
  PAGE_SUBTITLE: '0.9em'
};

function HydroscopeDIYComponent() {
  const location = useLocation();
  
  // Component state
  const [components, setComponents] = React.useState({});
  const [error, setError] = React.useState(null);
  const [loading, setLoading] = React.useState(true);
  const [graphData, setGraphData] = React.useState(null);
  const [visualizationState, setVisualizationState] = React.useState(null);
  const [metadata, setMetadata] = React.useState(null);
  
  // Configuration state
  const [colorPalette, setColorPalette] = React.useState('Set3');
  const [layoutAlgorithm, setLayoutAlgorithm] = React.useState('mrtree');
  const [isLayoutRunning, setIsLayoutRunning] = React.useState(false);
  
  // Refs
  const hydroscopeRef = React.useRef(null);

  // Load individual components from external repo
  React.useEffect(() => {
    const loadComponents = async () => {
      try {
        console.log('🔍 Loading individual components from external repo...');
        
        // Import all the DIY toolkit components
        const hydroscopeModule = await import('@hydro-project/hydroscope');
        
        console.log('📦 Hydroscope module loaded:', Object.keys(hydroscopeModule));
        
        const {
          // Core components
          Hydroscope,
          generateCompleteExample,
          
          // DIY Toolkit components
          LayoutControls,
          StyleTunerPanel,
          InfoPanel,
          FileDropZone,
          
          // Utility functions
          createContainerClickHandler,
          COLOR_PALETTES,
          LAYOUT_ALGORITHMS
        } = hydroscopeModule;
        
        // Validate required components
        const requiredComponents = {
          Hydroscope,
          generateCompleteExample,
          LayoutControls,
          StyleTunerPanel,
          InfoPanel,
          FileDropZone,
          createContainerClickHandler
        };
        
        for (const [name, component] of Object.entries(requiredComponents)) {
          if (!component) {
            throw new Error(`${name} component not found in external module`);
          }
        }
        
        console.log('✅ All DIY toolkit components loaded successfully');
        
        setComponents({
          ...requiredComponents,
          COLOR_PALETTES: COLOR_PALETTES || ['Set1', 'Set2', 'Set3'],
          LAYOUT_ALGORITHMS: LAYOUT_ALGORITHMS || ['mrtree', 'elkLayered']
        });
        setLoading(false);
        setError(null);
      } catch (err) {
        console.error('❌ Failed to load DIY toolkit components:', err);
        setError(`Failed to load DIY toolkit components: ${err.message}`);
        setLoading(false);
      }
    };
    loadComponents();
  }, []);

  // Handle URL data parameter
  React.useEffect(() => {
    if (loading) return;
    
    const urlParams = new URLSearchParams(location.search);
    const hashParams = new URLSearchParams(location.hash.slice(1));
    const dataParam = urlParams.get('data') || hashParams.get('data');
    
    if (dataParam && !graphData) {
      try {        
        const jsonString = atob(dataParam);
        const jsonData = JSON.parse(jsonString);
        console.log('📊 Loading graph from URL data parameter');
        setGraphData(jsonData);
      } catch (err) {
        console.error('❌ Failed to load graph from URL data:', err);
        setError(`Failed to load graph from URL: ${err.message}`);
      }
    }
  }, [loading, location.search, location.hash, graphData]);

  // Handle file upload
  const handleFileUpload = React.useCallback((uploadedData, filename) => {
    console.log(`📁 File uploaded via DIY FileDropZone: ${filename}`);
    setGraphData(uploadedData);
    setError(null);
  }, []);

  // Generate example data
  const handleGenerateExample = React.useCallback(() => {
    if (!components.generateCompleteExample) return;
    
    try {
      console.log('🎲 Generating example data...');
      const exampleData = components.generateCompleteExample();
      setGraphData(exampleData);
      setError(null);
    } catch (err) {
      console.error('❌ Failed to generate example:', err);
      setError(`Failed to generate example: ${err.message}`);
    }
  }, [components.generateCompleteExample]);

  // Handle parsing to capture visualization state
  const handleParsed = React.useCallback((parsedMetadata, visState) => {
    console.log('🎯 DIY Demo: Received visualization state');
    setVisualizationState(visState);
    setMetadata(parsedMetadata);
  }, []);

  // Create custom node click handler using the DIY toolkit
  const handleNodeClick = React.useMemo(() => {
    if (!components.createContainerClickHandler || !visualizationState) return undefined;
    
    return components.createContainerClickHandler(
      visualizationState,
      async () => {
        if (hydroscopeRef.current?.refreshLayout) {
          setIsLayoutRunning(true);
          try {
            await hydroscopeRef.current.refreshLayout();
          } finally {
            setIsLayoutRunning(false);
          }
        }
      },
      {
        enableCollapse: true,
        autoFit: true,
        onExpand: (containerId) => console.log(`📂 Container expanded: ${containerId}`),
        onCollapse: (containerId) => console.log(`📦 Container collapsed: ${containerId}`)
      }
    );
  }, [components.createContainerClickHandler, visualizationState]);

  // Handle layout algorithm change
  const handleLayoutChange = React.useCallback(async (algorithm) => {
    setLayoutAlgorithm(algorithm);
    if (hydroscopeRef.current?.refreshLayout) {
      setIsLayoutRunning(true);
      try {
        await hydroscopeRef.current.refreshLayout();
      } finally {
        setIsLayoutRunning(false);
      }
    }
  }, []);

  // Handle pack/unpack all
  const handlePackAll = React.useCallback(async () => {
    if (!visualizationState) return;
    setIsLayoutRunning(true);
    try {
      visualizationState.collapseAllContainers();
      if (hydroscopeRef.current?.refreshLayout) {
        await hydroscopeRef.current.refreshLayout();
      }
    } finally {
      setIsLayoutRunning(false);
    }
  }, [visualizationState]);

  const handleUnpackAll = React.useCallback(async () => {
    if (!visualizationState) return;
    setIsLayoutRunning(true);
    try {
      visualizationState.expandAllContainers();
      if (hydroscopeRef.current?.refreshLayout) {
        await hydroscopeRef.current.refreshLayout();
      }
    } finally {
      setIsLayoutRunning(false);
    }
  }, [visualizationState]);

  const layoutConfig = {
    algorithm: layoutAlgorithm,
  };

  const renderConfig = {
    colorPalette,
    fitView: true,
  };

  return (
    <Layout
      title="Hydroscope DIY Demo"
      description="Custom graph visualization assembly with individual components">
      <div style={{
        padding: '20px',
        minHeight: 'calc(100vh - 180px)',
        display: 'flex',
        flexDirection: 'column'
      }}>
        
        {/* Page Header */}
        <div style={{ marginBottom: '20px', textAlign: 'center' }}>
          <h1 style={{ 
            fontSize: TYPOGRAPHY.PAGE_TITLE, 
            marginBottom: '10px',
            color: '#2e8555'
          }}>
            Hydroscope DIY Demo
          </h1>
          <p style={{ 
            fontSize: TYPOGRAPHY.PAGE_SUBTITLE,
            color: '#666',
            maxWidth: '800px',
            margin: '0 auto'
          }}>
            Custom graph visualization built by assembling individual components from the DIY toolkit.
            Perfect for custom layouts and specialized requirements.
          </p>
        </div>

        {/* Error Display */}
        {error && (
          <div style={{
            background: '#fff2f0',
            border: '1px solid #ffccc7',
            borderRadius: '6px',
            padding: '16px',
            marginBottom: '20px',
            color: '#a8071a'
          }}>
            <strong>Error:</strong> {error}
            <br />
            <button 
              onClick={() => setError(null)}
              style={{
                marginTop: '10px',
                padding: '4px 8px',
                border: '1px solid #d9d9d9',
                borderRadius: '4px',
                cursor: 'pointer'
              }}
            >
              Dismiss
            </button>
          </div>
        )}

        {/* Loading State */}
        {loading && (
          <div style={{
            display: 'flex',
            justifyContent: 'center',
            alignItems: 'center',
            height: '400px',
            fontSize: '16px',
            color: '#666'
          }}>
            Loading DIY toolkit components...
          </div>
        )}

        {/* Main Content */}
        {!loading && (
          <div style={{ display: 'flex', gap: '20px', flex: 1 }}>
            
            {/* Left Panel - Controls */}
            <div style={{ 
              width: '300px', 
              display: 'flex', 
              flexDirection: 'column', 
              gap: '16px',
              flexShrink: 0
            }}>
              
              {/* File Upload */}
              {components.FileDropZone && (
                <div style={{ 
                  padding: '16px', 
                  border: '1px solid #d9d9d9', 
                  borderRadius: '6px',
                  backgroundColor: '#fafafa'
                }}>
                  <h4 style={{ margin: '0 0 12px 0', color: '#2e8555' }}>File Upload</h4>
                  <components.FileDropZone
                    onFileLoad={handleFileUpload}
                    acceptedTypes={['.json']}
                  />
                  <button
                    onClick={handleGenerateExample}
                    style={{
                      width: '100%',
                      marginTop: '8px',
                      padding: '8px',
                      backgroundColor: '#2e8555',
                      color: 'white',
                      border: 'none',
                      borderRadius: '4px',
                      cursor: 'pointer'
                    }}
                  >
                    Generate Example
                  </button>
                </div>
              )}

              {/* Layout Controls */}
              {components.LayoutControls && visualizationState && (
                <div style={{ 
                  padding: '16px', 
                  border: '1px solid #d9d9d9', 
                  borderRadius: '6px',
                  backgroundColor: '#fafafa'
                }}>
                  <h4 style={{ margin: '0 0 12px 0', color: '#2e8555' }}>Layout Controls</h4>
                  <components.LayoutControls
                    layoutAlgorithm={layoutAlgorithm}
                    onLayoutChange={handleLayoutChange}
                    onPackAll={handlePackAll}
                    onUnpackAll={handleUnpackAll}
                    isLayoutRunning={isLayoutRunning}
                    visualizationState={visualizationState}
                    onRefreshLayout={() => hydroscopeRef.current?.refreshLayout()}
                  />
                </div>
              )}

              {/* Style Tuner */}
              {components.StyleTunerPanel && visualizationState && (
                <div style={{ 
                  padding: '16px', 
                  border: '1px solid #d9d9d9', 
                  borderRadius: '6px',
                  backgroundColor: '#fafafa'
                }}>
                  <h4 style={{ margin: '0 0 12px 0', color: '#2e8555' }}>Style Tuner</h4>
                  <components.StyleTunerPanel
                    colorPalette={colorPalette}
                    onPaletteChange={setColorPalette}
                    visualizationState={visualizationState}
                  />
                </div>
              )}

              {/* Info Panel */}
              {components.InfoPanel && visualizationState && (
                <div style={{ 
                  padding: '16px', 
                  border: '1px solid #d9d9d9', 
                  borderRadius: '6px',
                  backgroundColor: '#fafafa'
                }}>
                  <h4 style={{ margin: '0 0 12px 0', color: '#2e8555' }}>Graph Info</h4>
                  <components.InfoPanel
                    visualizationState={visualizationState}
                  />
                </div>
              )}
            </div>

            {/* Right Panel - Graph */}
            <div style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
              
              {/* Graph Container */}
              {components.Hydroscope && graphData ? (
                <components.Hydroscope
                  ref={hydroscopeRef}
                  data={graphData}
                  config={renderConfig}
                  layoutConfig={layoutConfig}
                  onParsed={handleParsed}
                  eventHandlers={{
                    onNodeClick: handleNodeClick
                  }}
                  fillViewport={true}
                />
              ) : (
                <div style={{
                  flex: 1,
                  display: 'flex',
                  justifyContent: 'center',
                  alignItems: 'center',
                  fontSize: '16px',
                  color: '#666',
                  border: '2px dashed #d9d9d9',
                  borderRadius: '6px'
                }}>
                  Upload a JSON file or generate an example to view the custom-assembled graph
                </div>
              )}
            </div>
          </div>
        )}

        {/* DIY Toolkit Information */}
        <div style={{ 
          marginTop: '30px', 
          padding: '20px', 
          backgroundColor: '#f6f8fa', 
          borderRadius: '6px' 
        }}>
          <h3 style={{ marginTop: 0, color: '#2e8555' }}>DIY Toolkit Components</h3>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(300px, 1fr))', gap: '20px' }}>
            <div>
              <strong>🧩 Core Components</strong>
              <ul style={{ margin: '8px 0', paddingLeft: '20px' }}>
                <li><code>Hydroscope</code> - Basic graph rendering</li>
                <li><code>FileDropZone</code> - File upload widget</li>
                <li><code>LayoutControls</code> - Algorithm selection</li>
                <li><code>StyleTunerPanel</code> - Color customization</li>
              </ul>
            </div>
            <div>
              <strong>🔧 Utility Functions</strong>
              <ul style={{ margin: '8px 0', paddingLeft: '20px' }}>
                <li><code>createContainerClickHandler</code></li>
                <li><code>createLayoutConfig</code></li>
                <li><code>generateCompleteExample</code></li>
                <li><code>COLOR_PALETTES</code> array</li>
              </ul>
            </div>
            <div>
              <strong>📊 This Demo Shows</strong>
              <ul style={{ margin: '8px 0', paddingLeft: '20px' }}>
                <li>Manual component assembly</li>
                <li>Custom event handler setup</li>
                <li>State management between components</li>
                <li>Flexible layout configuration</li>
              </ul>
            </div>
          </div>
          <div style={{ marginTop: '16px', padding: '12px', backgroundColor: '#e6f7ff', borderRadius: '4px' }}>
            <strong>💡 Code Example:</strong> This page demonstrates building a custom visualization by importing 
            individual components and wiring them together manually, giving you complete control over layout and functionality.
          </div>
        </div>
      </div>
    </Layout>
  );
}

export default function HydroscopeDIYPage() {
  return (
    <BrowserOnly fallback={<div>Loading...</div>}>
      {() => <HydroscopeDIYComponent />}
    </BrowserOnly>
  );
}
