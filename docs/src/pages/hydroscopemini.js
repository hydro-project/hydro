/**
 * HydroscopeMini Demo - Interactive with Basic Controls
 * 
 * URL: /hydroscopemini
 * 
 * Demonstrates the interactive hydroscope component with:
 * - Click containers to collapse/expand automatically
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

function HydroscopeMiniDemo() {
  const location = useLocation();
  const [HydroscopeMini, setHydroscopeMini] = React.useState(null);
  const [generateCompleteExample, setGenerateCompleteExample] = React.useState(null);
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState(null);
  const [graphData, setGraphData] = React.useState(null);

  // Load hydroscope components
  React.useEffect(() => {
    const loadHydroscope = async () => {
      try {
        console.log('🔍 Loading hydroscope components...');
        
        // Import the HydroscopeMini component from the external repository
        const hydroscopeModule = await import('@hydro-project/hydroscope');
        
        console.log('✅ Module loaded:', Object.keys(hydroscopeModule));
        
        const { HydroscopeMini: HydroscopeMiniComponent, generateCompleteExample: generateFn } = hydroscopeModule;
        
        if (HydroscopeMiniComponent) {
          setHydroscopeMini(() => HydroscopeMiniComponent);
          console.log('✅ HydroscopeMini component loaded');
        }
        
        if (generateFn) {
          setGenerateCompleteExample(() => generateFn);
          console.log('✅ generateCompleteExample loaded');
          
          // Generate initial data
          try {
            const initialData = generateFn();
            setGraphData(initialData);
            console.log('📊 Initial data generated:', initialData?.nodes?.length, 'nodes');
          } catch (genErr) {
            console.error('❌ Failed to generate initial data:', genErr);
          }
        }
        
        setLoading(false);
        
      } catch (err) {
        console.error('❌ Failed to load hydroscope:', err);
        setError(`Failed to load hydroscope: ${err.message}`);
        setLoading(false);
      }
    };
    
    loadHydroscope();
  }, []);

  const generateNewData = () => {
    if (generateCompleteExample) {
      try {
        const newData = generateCompleteExample();
        setGraphData(newData);
        console.log('🎲 New data generated:', newData?.nodes?.length, 'nodes');
      } catch (err) {
        console.error('❌ Failed to generate new data:', err);
      }
    }
  };

  const containerStyle = {
    minHeight: '100vh',
    display: 'flex',
    flexDirection: 'column',
  };

  const headerStyle = {
    padding: '20px',
    textAlign: 'center',
    borderBottom: '1px solid #e0e0e0',
    backgroundColor: '#fafafa',
  };

  const contentStyle = {
    flex: 1,
    minHeight: 0,
  };

  return (
    <Layout 
      title="HydroscopeMini Demo" 
      description="Interactive graph visualization with built-in container controls"
    >
      <div style={containerStyle}>
        <div style={headerStyle}>
          <h1 style={{ fontSize: TYPOGRAPHY.PAGE_TITLE, margin: '0 0 10px 0' }}>
            HydroscopeMini Demo
          </h1>
          <p style={{ fontSize: TYPOGRAPHY.PAGE_SUBTITLE, color: '#666', margin: '0 0 15px 0' }}>
            Interactive graph with built-in container collapse/expand - just click containers!
          </p>
          <div style={{ 
            display: 'inline-block', 
            padding: '10px 15px', 
            backgroundColor: '#e3f2fd', 
            borderRadius: '4px',
            border: '1px solid #90caf9'
          }}>
            <strong>💡 Try this:</strong> Click any container (grouped nodes) to collapse/expand it
          </div>
        </div>

        {loading && (
          <div style={{ padding: '40px', textAlign: 'center' }}>
            <p>Loading hydroscope components...</p>
          </div>
        )}

        {error && (
          <div style={{ padding: '40px', textAlign: 'center', color: '#d32f2f' }}>
            <h3>Error</h3>
            <p>{error}</p>
          </div>
        )}

        {!loading && !error && HydroscopeMini && graphData && (
          <div style={contentStyle}>
            <div style={{
              padding: '8px 16px',
              borderBottom: '1px solid #e0e0e0',
              display: 'flex',
              gap: '8px',
              backgroundColor: '#f9f9f9'
            }}>
              <button 
                onClick={generateNewData}
                style={{
                  padding: '4px 8px',
                  border: '1px solid #d9d9d9',
                  backgroundColor: '#ffffff',
                  borderRadius: '4px',
                  cursor: 'pointer'
                }}
              >
                🎲 Generate New Data
              </button>
              <span style={{ fontSize: '0.9em', color: '#666', alignSelf: 'center' }}>
                {graphData.nodes?.length || 0} nodes, {graphData.edges?.length || 0} edges
              </span>
            </div>
            
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
          </div>
        )}
      </div>
    </Layout>
  );
}

export default function HydroscopeMiniPage() {
  return (
    <BrowserOnly fallback={<div>Loading...</div>}>
      {() => <HydroscopeMiniDemo />}
    </BrowserOnly>
  );
}
