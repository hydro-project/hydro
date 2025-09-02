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
  const [availableHeight, setAvailableHeight] = React.useState('100vh');

  // Calculate available height dynamically
  React.useEffect(() => {
    const calculateHeight = () => {
      // Get the navbar height dynamically
      const navbar = document.querySelector('.navbar');
      const navbarHeight = navbar ? navbar.offsetHeight : 60;
      setAvailableHeight(`calc(100vh - ${navbarHeight}px)`);
    };

    calculateHeight();
    window.addEventListener('resize', calculateHeight);
    return () => window.removeEventListener('resize', calculateHeight);
  }, []);

  // Load HydroscopeFull component from npm package
  React.useEffect(() => {
    const loadHydroscopeFull = async () => {
      try {
        const hydroscopeModule = await import('@hydro-project/hydroscope');
        
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
      // We cannot read arbitrary local files in-browser; pass the path to the UI for display/copy
      setFilePath(decodedPath);
      setError(null);
      return;
    }
    
    // Handle compressed or uncompressed data parameter using the hydroscope API
        if ((compressedParam || dataParam) && !graphData) {
          parseDataFromUrl(dataParam, compressedParam)
            .then(jsonData => {
              // Diagnostic: Log the parsed graphData and edgeStyleConfig
              console.log('[Hydro GraphData]', jsonData);
              if (jsonData && jsonData.edgeStyleConfig) {
                console.log('[Hydro EdgeStyleConfig]', jsonData.edgeStyleConfig);
              } else {
                console.warn('[Hydro EdgeStyleConfig] MISSING in parsed graphData');
              }
              setGraphData(jsonData);
              setError(null);
            })
            .catch(err => {
              console.error('❌ Failed to load graph from URL:', err);
              setError(`Failed to load graph from URL: ${err.message}`);
            });
    }
  }, [loading, location.search, location.hash, graphData, parseDataFromUrl]);

  // Handle file upload
  const handleFileUpload = React.useCallback((data, filename) => {
    setGraphData(data);
  }, []);

  // Handle example generation
  const handleExampleGenerated = React.useCallback((data) => {
    setGraphData(data);
  }, []);

  // Manual example generation (in case the component doesn't handle it internally)
  const handleCreateExample = React.useCallback(() => {
    if (generateCompleteExample) {
      try {
        const exampleData = generateCompleteExample();
        setGraphData(exampleData);
      } catch (err) {
        console.error('❌ Failed to generate example data:', err);
        setError(`Failed to generate example: ${err.message}`);
      }
    }
  }, [generateCompleteExample]);

  const containerStyle = {
    height: availableHeight,
    display: 'flex',
    flexDirection: 'column',
    overflow: 'hidden',
  };

  const headerStyle = {
    padding: '20px',
    textAlign: 'center',
    borderBottom: '1px solid #e0e0e0',
    background: 'white',
    flexShrink: 0, // Prevent header from shrinking
  };

  const contentStyle = {
    flex: 1,
    display: 'flex',
    flexDirection: 'column',
    minHeight: 0, // Allow content to shrink
    overflow: 'hidden',
  };

  return (
    <Layout 
      title="Hydroscope" 
      description="Complete graph visualization interface"
      noFooter={true}
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
          <>
            <div style={contentStyle}>
              <HydroscopeFull
                data={graphData}
                showFileUpload={true}
                showInfoPanel={true}
                showStylePanel={true}
                enableCollapse={true}
                autoFit={true}
                initialLayoutAlgorithm="mrtree"
                initialColorPalette="Set3"
                generatedFilePath={filePath}
                generateCompleteExample={generateCompleteExample}
                onFileUpload={handleFileUpload}
                onExampleGenerated={handleExampleGenerated}
                onCreateExample={handleCreateExample}
                style={{ 
                  height: '100%',
                  width: '100%',
                }}
              />
            </div>
          </>
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
