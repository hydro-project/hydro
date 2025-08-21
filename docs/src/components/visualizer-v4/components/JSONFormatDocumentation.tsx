import React, { useState } from 'react';
import { generateSchemaDocumentation, SCHEMA_VERSION } from '../docs/generateJSONSchema';

export function JSONFormatDocumentation(): JSX.Element {
  const [isExpanded, setIsExpanded] = useState(false);
  const schema = generateSchemaDocumentation();

  const toggleExpanded = () => setIsExpanded(!isExpanded);

  if (!isExpanded) {
    return (
      <div style={{ 
        marginTop: '32px', 
        paddingTop: '24px', 
        borderTop: '1px solid #eee', 
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
          📖 Click here for documentation on our JSON format
        </button>
      </div>
    );
  }

  return (
    <div style={{ 
      marginTop: '32px', 
      paddingTop: '24px', 
      borderTop: '1px solid #eee', 
      color: '#555', 
      fontSize: '13px',
      maxHeight: '500px',
      overflowY: 'auto',
      border: '1px solid #e0e0e0',
      borderRadius: '6px',
      padding: '24px',
      background: '#fafafa',
      margin: '32px 10% 10% 10%',
      width: '80%'
    }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
        <h4 style={{ margin: 0, color: '#333', fontSize: '16px' }}>
          Hydro JSON Format Documentation <span style={{ fontSize: '12px', color: '#666' }}>({SCHEMA_VERSION})</span>
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
        <h5 style={{ color: '#333', marginTop: '0' }}>Required Structure:</h5>
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
          tabSize: 2
        }}>{schema.requiredExample}</pre>

        <h5 style={{ color: '#333', marginTop: '24px' }}>Optional Configuration:</h5>
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
          tabSize: 2
        }}>{schema.optionalExample}</pre>

        <div style={{ marginTop: '12px', fontSize: '11px', color: '#666' }}>
          <em>This documentation is automatically synchronized with the JSON parser schema ({SCHEMA_VERSION}).</em>
        </div>
      </div>
    </div>
  );
}
