/**
 * Hydroscope - Complete Graph Visualization Experience
 * 
 * URL: /hydroscope
 * 
 * This is the main hydroscope interface, providing the complete visualization experience
 * that replaces the original /vis application. Features include:
 * - File upload and drag-drop
 * - Layout algorithm controls  
 * - Style tuning with color palettes
 * - Interactive container collapse/expand
 * - Information panel with metadata
 * - Grouping controls
 * - Pack/Unpack all operations
 */

import React from 'react';
import Layout from '@theme/Layout';
import BrowserOnly from '@docusaurus/BrowserOnly';
import { useLocation } from '@docusaurus/router';

// Import CSS from hydroscope npm package
import '@hydro-project/hydroscope/style.css';

// Typography constants for consistent styling
const TYPOGRAPHY = {
  PAGE_TITLE: '2.5em',
  PAGE_SUBTITLE: '0.9em'
};

function HydroscopeDemo() {
  const location = useLocation();
  const [HydroscopeFull, setHydroscopeFull] = React.useState(null);
  const [generateCompleteExample, setGenerateCompleteExample] = React.useState(null);
  const [parseDataFromUrl, setParseDataFromUrl] = React.useState(null);
  const [error, setError] = React.useState(null);
  const [loading, setLoading] = React.useState(true);
  const [graphData, setGraphData] = React.useState(null);
  const [filePath, setFilePath] = React.useState(null);

  // Load HydroscopeFull component from npm package
  React.useEffect(() => {
    const loadHydroscopeFull = async () => {
      try {
        console.log('🔍 Loading Hydroscope component from npm package...');
        
        const hydroscopeModule = await import('@hydro-project/hydroscope');
        
        console.log('📦 Hydroscope module loaded:', Object.keys(hydroscopeModule));
        
        const { HydroscopeFull, generateCompleteExample, parseDataFromUrl } = hydroscopeModule;
        
        if (!HydroscopeFull) {
          throw new Error('HydroscopeFull component not found in external module');
        }
        
        if (!parseDataFromUrl) {
          throw new Error('parseDataFromUrl function not found in external module');
        }
        
        console.log('✅ Hydroscope component loaded successfully');
        
        setHydroscopeFull(() => HydroscopeFull);
        setGenerateCompleteExample(() => generateCompleteExample);
        setParseDataFromUrl(() => parseDataFromUrl);
        setLoading(false);
        setError(null);
      } catch (err) {
        console.error('❌ Failed to load Hydroscope component:', err);
        setError(`Failed to load Hydroscope: ${err.message}`);
        setLoading(false);
      }
    };
    loadHydroscopeFull();
  }, []);

  // Handle URL data parameter (for sharing graphs via URL)
  React.useEffect(() => {
    if (loading || !parseDataFromUrl) return;
    
    const urlParams = new URLSearchParams(location.search);
    const hashParams = new URLSearchParams(location.hash.slice(1));
    const dataParam = urlParams.get('data') || hashParams.get('data');
    const compressedParam = urlParams.get('compressed') || hashParams.get('compressed');
  const fileParam = urlParams.get('file') || hashParams.get('file');
    
    // Handle file path parameter (from Rust debug output)
    if (fileParam && !graphData) {
      const decodedPath = decodeURIComponent(fileParam);
      console.log('📁 Received file path param:', decodedPath);
      // We cannot read arbitrary local files in-browser; pass the path to the UI for display/copy
      setFilePath(decodedPath);
      setError(null);
      return;
    }
    
    // Handle compressed or uncompressed data parameter using the hydroscope API
    if ((compressedParam || dataParam) && !graphData) {
      parseDataFromUrl(dataParam, compressedParam)
        .then(jsonData => {
          console.log('📊 Loading graph from URL parameter');
          setGraphData(jsonData);
          setError(null);
        })
        .catch(err => {
          console.error('❌ Failed to load graph from URL:', err);
          setError(`Failed to load graph from URL: ${err.message}`);
        });
    }
  }, [loading, location.search, location.hash, graphData, parseDataFromUrl]);

  // Generate example data when no data is provided - DISABLED to show FileDropZone by default
  // React.useEffect(() => {
  //   if (!loading && !graphData && generateCompleteExample) {
  //     try {
  //       console.log('🎲 Generating example graph data...');
  //       const exampleData = generateCompleteExample();
  //       setGraphData(exampleData);
  //     } catch (err) {
  //       console.error('❌ Failed to generate example data:', err);
  //       setError(`Failed to generate example: ${err.message}`);
  //     }
  //   }
  // }, [loading, graphData, generateCompleteExample]);

  // Handle file upload
  const handleFileUpload = React.useCallback((data, filename) => {
    console.log(`📁 File uploaded: ${filename}`);
    setGraphData(data);
  }, []);

  // Handle example generation
  const handleExampleGenerated = React.useCallback((data) => {
    console.log('🎲 Example generated, updating graph data');
    setGraphData(data);
  }, []);

  // Manual example generation (in case the component doesn't handle it internally)
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

  // Handle container interactions
  const handleContainerExpand = React.useCallback((containerId, visualizationState) => {
    console.log(`🔄 Container expanded: ${containerId}`);
  }, []);

  const handleContainerCollapse = React.useCallback((containerId, visualizationState) => {
    console.log(`🔄 Container collapsed: ${containerId}`);
  }, []);

  // Handle parsing events
  const handleParsed = React.useCallback((metadata, visualizationState) => {
    console.log('🎯 Graph parsed successfully:', metadata);
  }, []);

  const containerStyle = {
    minHeight: '100vh',
    display: 'flex',
    flexDirection: 'column',
  };

  const headerStyle = {
    padding: '20px',
    textAlign: 'center',
    borderBottom: '1px solid #e0e0e0',
    background: 'white',
  };

  const contentStyle = {
    flex: 1,
    display: 'flex',
    flexDirection: 'column',
    minHeight: 0,
  };

  return (
    <Layout 
      title="Hydroscope" 
      description="Complete graph visualization interface"
    >
      <div style={containerStyle}>
        <div style={headerStyle}>
          {/* <h1 style={{ fontSize: TYPOGRAPHY.PAGE_TITLE, margin: '0 0 10px 0' }}>
            Hydroscope
          </h1>
          <p style={{ fontSize: TYPOGRAPHY.PAGE_SUBTITLE, color: '#666', margin: 0 }}>
            Complete graph visualization interface with full controls and interactive features
          </p> */}
        </div>

        {loading && (
          <div style={{ padding: '40px', textAlign: 'center' }}>
            <p>Loading Hydroscope component...</p>
          </div>
        )}

        {error && (
          <div style={{ padding: '40px', textAlign: 'center', color: '#d32f2f' }}>
            <h3>Error</h3>
            <p>{error}</p>
          </div>
        )}

        {!loading && !error && HydroscopeFull && (
          <div style={contentStyle}>
            <HydroscopeFull
              data={graphData}
              showFileUpload={true}
              showSidebar={true}
              enableCollapse={true}
              autoFit={true}
              initialLayoutAlgorithm="mrtree"
              initialColorPalette="Set3"
              generatedFilePath={filePath}
              generateCompleteExample={generateCompleteExample}
              onFileUpload={handleFileUpload}
              onExampleGenerated={handleExampleGenerated}
              onCreateExample={handleCreateExample}
              onContainerExpand={handleContainerExpand}
              onContainerCollapse={handleContainerCollapse}
              onParsed={handleParsed}
            />
          </div>
        )}

        {!loading && !error && !HydroscopeFull && (
          <div style={{ padding: '40px', textAlign: 'center', color: '#666' }}>
            <p>Hydroscope component not available</p>
          </div>
        )}
      </div>
    </Layout>
  );
}

export default function HydroscopeFullPage() {
  return (
    <BrowserOnly fallback={<div>Loading...</div>}>
      {() => <HydroscopeDemo />}
    </BrowserOnly>
  );
}
