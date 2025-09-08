/**
 * Load the hydroscope visualizer for Hydro JSON
 */

import React from 'react';
import Layout from '@theme/Layout';
import BrowserOnly from '@docusaurus/BrowserOnly';
import { useLocation } from '@docusaurus/router';

import '@xyflow/react/dist/style.css';      // React Flow base styles
import '@hydro-project/hydroscope/style.css'; // Hydroscope generated styles
import 'antd/dist/reset.css';               // (Optional) Ant Design reset

// Typography constants for consistent styling
const TYPOGRAPHY = {
  PAGE_TITLE: '2.5em',
  PAGE_SUBTITLE: '0.9em'
};

function HydroscopeFn() {
  const location = useLocation();
  const [Hydroscope, setHydroscope] = React.useState(null);
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

  // Load Hydroscope component from npm package
  React.useEffect(() => {
    const loadHydroscope = async () => {
      try {
        const hydroscopeModule = await import('@hydro-project/hydroscope');

        const { Hydroscope, generateCompleteExample, parseDataFromUrl } = hydroscopeModule;

        if (!Hydroscope) {
          throw new Error('Hydroscope component not found in external module');
        }

        if (!parseDataFromUrl) {
          throw new Error('parseDataFromUrl function not found in external module');
        }

        console.log('✅ Hydroscope component loaded successfully');

        setHydroscope(() => Hydroscope);
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
    loadHydroscope();
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
    if (!data) {
      console.error('❌ No data received from file upload');
      setError('No data received from file upload');
      return;
    }

    // Let Hydroscope manage its own data internally
    setError(null);
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
        setError(null);
      } catch (err) {
        console.error('❌ Failed to generate example data:', err);
        setError(`Failed to generate example: ${err.message}`);
      }
    }
  }, [generateCompleteExample]);

  // Add a test button to trigger example generation
  const handleTestExample = React.useCallback(() => {
    handleCreateExample();
  }, [handleCreateExample]);

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

      {!loading && !error && Hydroscope && (
        <div style={contentStyle}>
          <Hydroscope
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
      )
      }

      {
        !loading && !error && !Hydroscope && (
          <div style={{ padding: '40px', textAlign: 'center', color: '#666' }}>
            <p>Hydroscope component not available</p>
          </div>
        )
      }
    </Layout >
  );
}

export default function HydroscopePage() {
  return (
    <BrowserOnly fallback={<div>Loading...</div>}>
      {() => <HydroscopeFn />}
    </BrowserOnly>
  );
}
