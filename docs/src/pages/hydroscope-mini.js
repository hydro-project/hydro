/**
 * HydroscopeMini Demo - Interactive Graph with Built-in Controls
 * 
 * This page demonstrates the HydroscopeMini component from the external hydroscope repository.
 * HydroscopeMini provides interactive graph visualization with:
 * - Built-in container collapse/expand on click
 * - Pack/Unpack all controls
 * - Auto-fit after operations
 * - Zero configuration required - just pass data
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

function HydroscopeMiniComponent() {
  const location = useLocation();
  const [HydroscopeMini, setHydroscopeMini] = React.useState(null);
  const [generateCompleteExample, setGenerateCompleteExample] = React.useState(null);
  const [error, setError] = React.useState(null);
  const [loading, setLoading] = React.useState(true);
  const [graphData, setGraphData] = React.useState(null);
  const [generatedFilePath, setGeneratedFilePath] = React.useState(null);

  // Load HydroscopeMini component from external repo
  React.useEffect(() => {
    const loadHydroscope = async () => {
      try {
        console.log('🔍 Loading HydroscopeMini component from external repo...');
        
        // Import the HydroscopeMini component from the external repository
        const hydroscopeModule = await import('@hydro-project/hydroscope');
        
        console.log('📦 Hydroscope module loaded:', Object.keys(hydroscopeModule));
        
        const { HydroscopeMini, generateCompleteExample } = hydroscopeModule;
        
        if (!HydroscopeMini) {
          throw new Error('HydroscopeMini component not found in external module');
        }
        
        if (!generateCompleteExample) {
          throw new Error('generateCompleteExample function not found in external module');
        }
        
        console.log('✅ External HydroscopeMini component and example generator loaded successfully');
        
        setHydroscopeMini(() => HydroscopeMini);
        setGenerateCompleteExample(() => generateCompleteExample);
        setLoading(false);
        setError(null);
      } catch (err) {
        console.error('❌ Failed to load external HydroscopeMini component:', err);
        setError(`Failed to load external HydroscopeMini component: ${err.message}`);
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
    const fileParam = urlParams.get('file') || hashParams.get('file');
    
    // Handle file path parameter (from Rust debug output)
    if (fileParam && !generatedFilePath) {
      setGeneratedFilePath(decodeURIComponent(fileParam));
    }
    
    if (dataParam && !graphData) {
      try {        
        // Decode the base64 data
        const jsonString = atob(dataParam);
        const jsonData = JSON.parse(jsonString);
        
        console.log('📊 Loading graph from URL data parameter');
        setGraphData(jsonData);
      } catch (err) {
        console.error('❌ Failed to load graph from URL data:', err);
        setError(`Failed to load graph from URL: ${err.message}`);
      }
    }
  }, [loading, location.search, location.hash, graphData, generatedFilePath]);

  // Handle file upload
  const handleFileUpload = React.useCallback((file) => {
    const reader = new FileReader();
    reader.onload = (e) => {
      try {
        const jsonData = JSON.parse(e.target.result);
        console.log(`📁 File uploaded: ${file.name}`);
        setGraphData(jsonData);
        setError(null);
      } catch (err) {
        console.error('❌ Failed to parse uploaded file:', err);
        setError(`Failed to parse uploaded file: ${err.message}`);
      }
    };
    reader.readAsText(file);
  }, []);

  // Generate example data
  const handleGenerateExample = React.useCallback(() => {
    if (!generateCompleteExample) return;
    
    try {
      console.log('🎲 Generating example data...');
      const exampleData = generateCompleteExample();
      setGraphData(exampleData);
      setError(null);
    } catch (err) {
      console.error('❌ Failed to generate example:', err);
      setError(`Failed to generate example: ${err.message}`);
    }
  }, [generateCompleteExample]);

  return (
    <Layout
      title="HydroscopeMini Demo"
      description="Interactive graph visualization with built-in container controls">
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
            HydroscopeMini Demo
          </h1>
          <p style={{ 
            fontSize: TYPOGRAPHY.PAGE_SUBTITLE,
            color: '#666',
            maxWidth: '800px',
            margin: '0 auto'
          }}>
            Interactive graph visualization with built-in container collapse/expand and basic controls.
            Perfect balance between simplicity and interactivity.
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
            Loading HydroscopeMini component...
          </div>
        )}

        {/* File Upload and Controls */}
        {!loading && (
          <div style={{
            display: 'flex',
            gap: '20px',
            marginBottom: '20px',
            flexWrap: 'wrap',
            alignItems: 'center',
            justifyContent: 'center'
          }}>
            <input
              type="file"
              accept=".json"
              onChange={(e) => {
                const file = e.target.files[0];
                if (file) handleFileUpload(file);
              }}
              style={{
                padding: '8px',
                border: '1px solid #d9d9d9',
                borderRadius: '4px'
              }}
            />
            <button
              onClick={handleGenerateExample}
              style={{
                padding: '8px 16px',
                backgroundColor: '#2e8555',
                color: 'white',
                border: 'none',
                borderRadius: '4px',
                cursor: 'pointer'
              }}
            >
              Generate Example
            </button>
            <button
              onClick={() => setGraphData(null)}
              style={{
                padding: '8px 16px',
                backgroundColor: '#f5222d',
                color: 'white',
                border: 'none',
                borderRadius: '4px',
                cursor: 'pointer'
              }}
            >
              Clear Graph
            </button>
            {generatedFilePath && (
              <span style={{ fontSize: '14px', color: '#666' }}>
                File: {generatedFilePath}
              </span>
            )}
          </div>
        )}

        {/* HydroscopeMini Component */}
        {!loading && HydroscopeMini && graphData && (
          <HydroscopeMini
            data={graphData}
            showControls={true}
            enableCollapse={true}
            autoFit={true}
            onParsed={(metadata, visualizationState) => {
              console.log('📊 HydroscopeMini parsed data:', { metadata, visualizationState });
            }}
            onContainerCollapse={(containerId, visualizationState) => {
              console.log(`📦 Container collapsed: ${containerId}`);
            }}
            onContainerExpand={(containerId, visualizationState) => {
              console.log(`📂 Container expanded: ${containerId}`);
            }}
          />
        )}

        {/* No Data State */}
        {!loading && !graphData && (
          <div style={{
            display: 'flex',
            justifyContent: 'center',
            alignItems: 'center',
            height: '400px',
            fontSize: '16px',
            color: '#666',
            border: '2px dashed #d9d9d9',
            borderRadius: '6px'
          }}>
            Upload a JSON file or generate an example to view the interactive graph
          </div>
        )}

        {/* Feature Comparison */}
        <div style={{ 
          marginTop: '30px', 
          padding: '20px', 
          backgroundColor: '#f6f8fa', 
          borderRadius: '6px' 
        }}>
          <h3 style={{ marginTop: 0, color: '#2e8555' }}>HydroscopeMini Features</h3>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(300px, 1fr))', gap: '20px' }}>
            <div>
              <strong>✅ Interactive Features</strong>
              <ul style={{ margin: '8px 0', paddingLeft: '20px' }}>
                <li>Click containers to collapse/expand</li>
                <li>Pack/Unpack all buttons</li>
                <li>Refresh layout button</li>
                <li>Auto-fit after operations</li>
              </ul>
            </div>
            <div>
              <strong>🎯 Perfect For</strong>
              <ul style={{ margin: '8px 0', paddingLeft: '20px' }}>
                <li>Embedded visualizations</li>
                <li>Quick graph exploration</li>
                <li>Minimal setup required</li>
                <li>Interactive demos</li>
              </ul>
            </div>
            <div>
              <strong>🔄 Comparison</strong>
              <ul style={{ margin: '8px 0', paddingLeft: '20px' }}>
                <li><strong>Basic Hydroscope:</strong> Read-only</li>
                <li><strong>HydroscopeMini:</strong> Interactive ✨</li>
                <li><strong>HydroscopeFull:</strong> Complete UI</li>
              </ul>
            </div>
          </div>
          <div style={{ marginTop: '16px', padding: '12px', backgroundColor: '#e6f7ff', borderRadius: '4px' }}>
            <strong>💡 Try It:</strong> Upload a graph or generate an example, then click on any container node to collapse/expand it. 
            Use the Pack/Unpack buttons to collapse or expand all containers at once.
          </div>
        </div>
      </div>
    </Layout>
  );
}

export default function HydroscopeMiniPage() {
  return (
    <BrowserOnly fallback={<div>Loading...</div>}>
      {() => <HydroscopeMiniComponent />}
    </BrowserOnly>
  );
}
