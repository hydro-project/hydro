import React, { useState } from 'react';
import { generateSchemaDocumentation, SCHEMA_VERSION } from '../docs/generateJSONSchema';

export function CompleteExampleDisplay(): JSX.Element {
  const [isExpanded, setIsExpanded] = useState(false);
  const schema = generateSchemaDocumentation();

  const toggleExpanded = () => setIsExpanded(!isExpanded);

  if (!isExpanded) {
    return (
      <div style={{ 
        marginTop: '16px', 
        color: '#777', 
        fontSize: '14px' 
      }}>
        <button
          onClick={toggleExpanded}
          style={{
            background: 'none',
            border: 'none',
            color: '#007acc',
            textDecoration: 'underline',
            cursor: 'pointer',
            fontSize: '14px',
            padding: 0
          }}
        >
          📝 View complete working example
        </button>
      </div>
    );
  }

  return (
    <div style={{ 
      marginTop: '16px', 
      paddingTop: '24px', 
      color: '#555', 
      fontSize: '13px',
      maxHeight: '600px',
      overflowY: 'auto',
      border: '1px solid #e0e0e0',
      borderRadius: '6px',
      padding: '24px',
      background: '#fafafa',
      margin: '16px 10% 10% 10%',
      width: '80%'
    }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
        <h4 style={{ margin: 0, color: '#333', fontSize: '16px' }}>
          Complete Working Example <span style={{ fontSize: '12px', color: '#666' }}>({SCHEMA_VERSION})</span>
        </h4>
        <button
          onClick={toggleExpanded}
          style={{
            background: '#f0f0f0',
            border: '1px solid #ddd',
            borderRadius: '4px',
            padding: '4px 8px',
            cursor: 'pointer',
            fontSize: '12px'
          }}
        >
          ✕ Close
        </button>
      </div>

      <div style={{ fontFamily: "'Monaco', 'Menlo', 'Ubuntu Mono', monospace", fontSize: '13px' }}>
        <pre style={{ 
          background: '#f8f8f8', 
          padding: '16px', 
          borderRadius: '4px', 
          border: '1px solid #e0e0e0',
          overflow: 'auto',
          margin: '8px 0',
          lineHeight: '1.4',
          whiteSpace: 'pre',
          textAlign: 'left',
          tabSize: 2,
          maxHeight: '320px'
        }}>{schema.completeExample}</pre>

        <div style={{ textAlign: 'center', margin: '16px 0' }}>
          <button
            onClick={() => {
              try {
                const exampleData = JSON.parse(schema.completeExample);
                const event = new CustomEvent('load-example-data', { detail: exampleData });
                window.dispatchEvent(event);
              } catch (err) {
                console.error('❌ Error loading example data:', err);
                alert('Failed to load example data');
              }
            }}
            style={{
              background: '#28a745',
              color: 'white',
              border: 'none',
              borderRadius: '6px',
              padding: '10px 20px',
              fontSize: '14px',
              fontWeight: '500',
              cursor: 'pointer',
              transition: 'background 0.2s ease'
            }}
            onMouseEnter={(e) => {
              (e.target as HTMLElement).style.background = '#218838';
            }}
            onMouseLeave={(e) => {
              (e.target as HTMLElement).style.background = '#28a745';
            }}
            title="Load this example as your starting graph"
          >
            ✨ Create Graph from Example
          </button>
        </div>

        <div style={{ marginTop: '12px', fontSize: '11px', color: '#666' }}>
          <em>This example is automatically validated against the parser ({SCHEMA_VERSION}).</em>
        </div>
      </div>
    </div>
  );
}
