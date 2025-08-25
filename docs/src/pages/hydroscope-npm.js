/**
 * Hydroscope - Complete Graph Visualization Experience
 * 
 * URL: /hydroscope-npm
 * 
 * This page demonstrates the proper integration with the published @hydro-project/hydroscope
 * npm package, providing a clean and maintainable way to include Hydroscope in Hydro docs.
 * 
 * Features:
 * - Uses published npm package instead of static files
 * - Proper CSS imports and component loading
 * - Full-screen layout optimized for visualization
 * - File upload, example generation, and interactive controls
 * - Responsive design that works well in Docusaurus
 */

import React from 'react';
import Layout from '@theme/Layout';
import BrowserOnly from '@docusaurus/BrowserOnly';
import { useLocation } from '@docusaurus/router';

// Import Hydroscope from npm package
import '@hydro-project/hydroscope/style.css';

function HydroscopeNpmDemo() {
  const location = useLocation();
  const [HydroscopeFull, setHydroscopeFull] = React.useState(null);
  const [generateCompleteExample, setGenerateCompleteExample] = React.useState(null);
  const [error, setError] = React.useState(null);
  const [loading, setLoading] = React.useState(true);
  const [graphData, setGraphData] = React.useState(null);

  // Load Hydroscope components from npm package
  React.useEffect(() => {
    const loadHydroscope = async () => {
      try {
        console.log('🔍 Loading Hydroscope from npm package...');
        
        // Import from the published npm package
        const hydroscopeModule = await import('@hydro-project/hydroscope');
        
        console.log('📦 Hydroscope module loaded:', Object.keys(hydroscopeModule));
        
        const { HydroscopeFull, generateCompleteExample } = hydroscopeModule;
        
        if (!HydroscopeFull) {
          throw new Error('HydroscopeFull component not found in npm package');
        }
        
        if (!generateCompleteExample) {
          throw new Error('generateCompleteExample function not found in npm package');
        }
        
        console.log('✅ Hydroscope npm package loaded successfully');
        
        setHydroscopeFull(() => HydroscopeFull);
        setGenerateCompleteExample(() => generateCompleteExample);
        setLoading(false);
        setError(null);
      } catch (err) {
        console.error('❌ Failed to load Hydroscope npm package:', err);
        setError(`Failed to load Hydroscope npm package: ${err.message}`);
        setLoading(false);
      }
    };
    
    loadHydroscope();
  }, []);

  // Handle URL data parameter (for sharing graphs via URL)
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

  // Event handlers
  const handleFileUpload = React.useCallback((data, filename) => {
    console.log(`📁 File uploaded: ${filename}`);
    setGraphData(data);
  }, []);

  const handleExampleGenerated = React.useCallback((data) => {
    console.log('🎲 Example generated, updating graph data');
    setGraphData(data);
  }, []);

  const handleCreateExample = React.useCallback(() => {
    if (generateCompleteExample) {
      try {
        console.log('🎲 Manually generating example graph data...');
        const exampleData = generateCompleteExample();
        setGraphData(exampleData);
      } catch (err) {
        console.error('❌ Failed to generate example data:', err);
        setError(`Failed to generate example: ${err.message}`);
      }
    }
  }, [generateCompleteExample]);

  const handleContainerExpand = React.useCallback((containerId, visualizationState) => {
    console.log(`🔄 Container expanded: ${containerId}`);
  }, []);

  const handleContainerCollapse = React.useCallback((containerId, visualizationState) => {
    console.log(`🔄 Container collapsed: ${containerId}`);
  }, []);

  const handleParsed = React.useCallback((metadata, visualizationState) => {
    console.log('🎯 Graph parsed successfully:', metadata);
  }, []);

  // Layout styles optimized for full-screen visualization
  const containerStyle = {
    minHeight: '100vh',
    display: 'flex',
    flexDirection: 'column',
    background: '#ffffff',
  };

  const headerStyle = {
    padding: '20px',
    textAlign: 'center',
    borderBottom: '1px solid #e0e0e0',
    background: '#fafafa',
    flexShrink: 0,
  };

  const contentStyle = {
    flex: 1,
    display: 'flex',
    flexDirection: 'column',
    minHeight: 0, // Important for flex child to shrink
    overflow: 'hidden', // Prevent content from overflowing
  };

  const hydroscopeWrapperStyle = {
    flex: 1,
    display: 'flex',
    flexDirection: 'column',
    minHeight: 0,
    position: 'relative',
  };

  return (
    <Layout 
      title="Hydroscope (NPM)" 
      description="Hydroscope graph visualization using the published npm package"
      noFooter={true} // Remove footer for full-screen experience
    >
      <div style={containerStyle}>
        <div style={headerStyle}>
          <h1 style={{ fontSize: '2.5em', margin: '0 0 10px 0', color: '#1f2937' }}>
            Hydroscope
          </h1>
          <p style={{ fontSize: '0.9em', color: '#666', margin: 0 }}>
            Graph visualization powered by <code>@hydro-project/hydroscope</code> npm package
          </p>
        </div>

        {loading && (
          <div style={{ 
            padding: '40px', 
            textAlign: 'center',
            flex: 1,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center'
          }}>
            <div>
              <div style={{ marginBottom: '16px', fontSize: '18px' }}>Loading Hydroscope...</div>
              <div style={{ color: '#666' }}>Importing from npm package...</div>
            </div>
          </div>
        )}

        {error && (
          <div style={{ 
            padding: '40px', 
            textAlign: 'center', 
            color: '#d32f2f',
            flex: 1,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center'
          }}>
            <div>
              <h3 style={{ marginBottom: '16px' }}>Error Loading Hydroscope</h3>
              <p style={{ marginBottom: '16px' }}>{error}</p>
              <div style={{ fontSize: '14px', color: '#666' }}>
                <p>This might be because:</p>
                <ul style={{ textAlign: 'left', display: 'inline-block' }}>
                  <li>The npm package is not installed</li>
                  <li>There's a version mismatch</li>
                  <li>The component export structure has changed</li>
                </ul>
                <p>Try running: <code>npm install @hydro-project/hydroscope@latest</code></p>
              </div>
            </div>
          </div>
        )}

        {!loading && !error && HydroscopeFull && (
          <div style={contentStyle}>
            <div style={hydroscopeWrapperStyle}>
              <HydroscopeFull
                data={graphData}
                showFileUpload={true}
                showInfoPanel={true}
                enableCollapse={true}
                autoFit={true}
                initialLayoutAlgorithm="mrtree"
                initialColorPalette="Set3"
                generateCompleteExample={generateCompleteExample}
                onFileUpload={handleFileUpload}
                onExampleGenerated={handleExampleGenerated}
                onCreateExample={handleCreateExample}
                onContainerExpand={handleContainerExpand}
                onContainerCollapse={handleContainerCollapse}
                onParsed={handleParsed}
                style={{
                  height: '100%',
                  width: '100%',
                  minHeight: '600px',
                }}
              />
            </div>
          </div>
        )}

        {!loading && !error && !HydroscopeFull && (
          <div style={{ 
            padding: '40px', 
            textAlign: 'center', 
            color: '#666',
            flex: 1,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center'
          }}>
            <div>
              <h3>Hydroscope Not Available</h3>
              <p>The HydroscopeFull component was not found in the npm package.</p>
              <p>Please check the package documentation or try updating to the latest version.</p>
            </div>
          </div>
        )}
      </div>
    </Layout>
  );
}

export default function HydroscopeNpmPage() {
  return (
    <BrowserOnly fallback={
      <div style={{ 
        height: '100vh', 
        display: 'flex', 
        alignItems: 'center', 
        justifyContent: 'center' 
      }}>
        <div>Loading Hydroscope...</div>
      </div>
    }>
      {() => <HydroscopeNpmDemo />}
    </BrowserOnly>
  );
}
