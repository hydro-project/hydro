/**
 * Visual Test Page for Collapsed Containers
 * Simple page to test rendering of collapsed containers with hyperEdges
 */

import React from 'react';
import Layout from '@theme/Layout';
import BrowserOnly from '@docusaurus/BrowserOnly';

export default function CollapsedContainersTestPage() {
  return (
    <Layout
      title="Collapsed Containers Visual Test"
      description="Visual test for collapsed container rendering with hyperEdges"
    >
      <div style={{ padding: '20px' }}>
        <h1>Collapsed Containers Visual Test</h1>
        <p>
          This page shows two collapsed containers with bidirectional hyperEdges.
          It's designed to test the rendering pipeline when both containers are collapsed.
        </p>
        
        <BrowserOnly fallback={<div>Loading...</div>}>
          {() => {
            // Use a more explicit import to avoid initialization issues
            try {
              const { default: CollapsedContainersVisualTest } = require('../components/visualizer-v4/visual-tests/CollapsedContainersVisualTest');
              return React.createElement(CollapsedContainersVisualTest);
            } catch (error) {
              console.error('Error loading visual test:', error);
              return (
                <div style={{ padding: '20px', color: 'red' }}>
                  <h3>Error loading visual test:</h3>
                  <pre>{error.message}</pre>
                  <p>Check the browser console for more details.</p>
                </div>
              );
            }
          }}
        </BrowserOnly>
        
        <div style={{ marginTop: '20px', padding: '15px', backgroundColor: '#f5f5f5', borderRadius: '5px' }}>
          <h3>Expected Behavior:</h3>
          <ul>
            <li>Two collapsed containers should be visible (loc_0 and loc_1)</li>
            <li>Containers should be positioned side by side, not overlapping</li>
            <li>Two hyperEdges should connect the containers bidirectionally</li>
            <li>No individual nodes should be visible (they're hidden inside collapsed containers)</li>
            <li>Clicking on containers or edges should log details to console</li>
          </ul>
          
          <h3>Debug Steps:</h3>
          <ol>
            <li>Open browser console to see debug logs</li>
            <li>Look for any rendering errors or warnings</li>
            <li>Check if containers have proper positions and dimensions</li>
            <li>Verify hyperEdges are being created and positioned correctly</li>
          </ol>
        </div>
      </div>
    </Layout>
  );
}
