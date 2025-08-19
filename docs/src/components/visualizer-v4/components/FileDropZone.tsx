/**
 * FileDropZone Component
 *
 * Handles JSON graph upload via drag-and-drop or file input.
 * Integrates with the visualizer v4 JSON parser and provides
 * optional inline documentation and a complete example loader.
 */

import React, { useState, useCallback } from 'react';
import { generateSchemaDocumentation, SCHEMA_VERSION } from '../docs/generateJSONSchema';

// JSON Format Documentation Component
function JSONFormatDocumentation() {
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
      maxHeight: '500px',  // Fixed height instead of viewport-based
      overflowY: 'auto',
      border: '1px solid #e0e0e0',
      borderRadius: '6px',
      padding: '24px',
      background: '#fafafa',
      margin: '32px 10% 10% 10%',  // 10% left/right margins for 80% width
      width: '80%'  // Use 80% of container width
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

// Complete Example Component (separate from documentation)
function CompleteExampleDisplay() {
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
                // Trigger the file load with the example data
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

interface FileDropZoneProps {
  onFileLoad: (data: any) => void;
  hasData?: boolean;
  className?: string;
  generatedFilePath?: string;
}

const dropZoneStyles: React.CSSProperties = {
  flex: 1,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  margin: '20px',
  borderWidth: '3px',
  borderStyle: 'dashed',
  borderColor: '#ccc',
  borderRadius: '12px',
  background: '#f9f9f9',
  transition: 'all 0.2s ease',
  minHeight: '400px',
  height: 'calc(100vh - 200px)',
};

const dragOverStyles: React.CSSProperties = {
  ...dropZoneStyles,
  borderColor: '#007acc',
  background: '#f0f8ff',
};

const dropContentStyles: React.CSSProperties = {
  textAlign: 'center',
  padding: '40px',
  maxWidth: '90%',  // Increased to 90% for better use of space
  width: '100%',
  height: '100%',
  display: 'flex',
  flexDirection: 'column',
  justifyContent: 'space-between',
};

const fileInputStyles: React.CSSProperties = {
  display: 'none',
};

const fileInputLabelStyles: React.CSSProperties = {
  display: 'inline-block',
  padding: '12px 24px',
  background: '#007acc',
  color: 'white',
  borderRadius: '6px',
  cursor: 'pointer',
  transition: 'background 0.2s ease',
  fontWeight: 500,
  border: 'none',
};

const loadingStyles: React.CSSProperties = {
  flex: 1,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  fontSize: '18px',
  color: '#666',
  minHeight: '400px',
  height: 'calc(100vh - 200px)',
  background: '#f9f9f9',
  borderWidth: '2px',
  borderStyle: 'dashed',
  borderColor: '#ddd',
  margin: '20px',
  borderRadius: '8px',
};

function FileDropZone({ 
  onFileLoad, 
  hasData = false, 
  className = '', 
  generatedFilePath 
}: FileDropZoneProps) {
  const [isDragOver, setIsDragOver] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [dragCounter, setDragCounter] = useState(0);

  const handleDragEnter = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setDragCounter(prev => prev + 1);
    setIsDragOver(true);
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setDragCounter(prev => {
      const newCount = prev - 1;
      if (newCount <= 0) {
        setIsDragOver(false);
        return 0;
      }
      return newCount;
    });
  }, []);

  const processFile = useCallback(async (file: File) => {
    setIsLoading(true);
    try {
      const reader = new FileReader();
      reader.onload = (event) => {
        try {
          const data = JSON.parse(event.target?.result as string);
          onFileLoad(data);
        } catch (error) {
          console.error('JSON parsing error:', error);
          alert('Invalid JSON file: ' + (error as Error).message);
        } finally {
          setIsLoading(false);
        }
      };
      reader.onerror = () => {
        console.error('File reading error');
        alert('Error reading file');
        setIsLoading(false);
      };
      reader.readAsText(file);
    } catch (error) {
      console.error('File processing error:', error);
      alert('Error processing file: ' + (error as Error).message);
      setIsLoading(false);
    }
  }, [onFileLoad]);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOver(false);
    setDragCounter(0);
    
    const files = Array.from(e.dataTransfer.files);
    const jsonFile = files.find(file => file.name.endsWith('.json'));
    
    if (jsonFile) {
      processFile(jsonFile);
    } else {
      alert('Please drop a JSON file');
    }
  }, [processFile]);

  const handleFileInput = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file && file.name.endsWith('.json')) {
      processFile(file);
    } else if (file) {
      alert('Please select a JSON file');
    }
    // Reset the input so the same file can be selected again
    e.target.value = '';
  }, [processFile]);

  if (hasData) {
    return null;
  }

  if (isLoading) {
    return (
      <div style={loadingStyles} className={className}>
        <div>
          <div>Loading graph data...</div>
          <div style={{ fontSize: '14px', color: '#999', marginTop: '8px' }}>
            Parsing JSON and building visualization state
          </div>
        </div>
      </div>
    );
  }

  return (
    <div 
      style={isDragOver ? dragOverStyles : dropZoneStyles}
      className={className}
      onDragEnter={handleDragEnter}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      <div style={dropContentStyles}>
        {/* Top section - main content */}
        <div>
          <h3 style={{ marginBottom: '16px', color: '#333', fontSize: '24px' }}>
            Graph Visualization
          </h3>
          <p style={{ marginBottom: '24px', color: '#666', fontSize: '16px' }}>
            Drop a compatible JSON file here or click to choose.
          </p>
          
          <input 
            type="file" 
            accept=".json"
            onChange={handleFileInput}
            style={fileInputStyles}
            id="file-input"
          />
          <label 
            htmlFor="file-input" 
            style={fileInputLabelStyles}
            onMouseEnter={(e) => {
              (e.target as HTMLElement).style.background = '#005999';
            }}
            onMouseLeave={(e) => {
              (e.target as HTMLElement).style.background = '#007acc';
            }}
          >
            Choose File
          </label>
          
          {!generatedFilePath && (
            <>
              <JSONFormatDocumentation />
              <CompleteExampleDisplay />
            </>
          )}
        </div>

        {/* Bottom section - generated file path (positioned 20% from bottom) */}
        {generatedFilePath && (
          <div style={{ 
            paddingTop: '24px',
            borderTop: '1px solid #eee', 
            color: '#555', 
            fontSize: '14px',
            marginTop: 'auto',
            marginBottom: '20%' // This pushes it to 20% from bottom
          }}>
            <p style={{ marginBottom: '12px', color: '#333', fontWeight: 'bold' }}>
              📄 Generated file ready to load:
            </p>
            <div style={{ position: 'relative' }}>
              <code style={{
                display: 'block',
                background: '#e8f5e8',
                padding: '12px 50px 12px 12px', // Extra right padding for button
                borderRadius: '6px',
                marginTop: '8px',
                fontFamily: "'Monaco', 'Menlo', 'Ubuntu Mono', monospace",
                color: '#2e7d32',
                fontSize: '13px',
                wordBreak: 'break-all'
              }}>
                {generatedFilePath}
              </code>
              <button
                onClick={() => {
                  navigator.clipboard.writeText(generatedFilePath).then(() => {
                    // Visual feedback - could be enhanced with a toast
                    const btn = document.activeElement as HTMLButtonElement;
                    const originalText = btn.textContent;
                    btn.textContent = '✓';
                    btn.style.background = '#4caf50';
                    setTimeout(() => {
                      btn.textContent = originalText;
                      btn.style.background = '#007acc';
                    }, 1000);
                  }).catch(err => {
                    console.error('Failed to copy to clipboard:', err);
                    alert('Failed to copy to clipboard');
                  });
                }}
                style={{
                  position: 'absolute',
                  top: '50%',
                  right: '8px',
                  transform: 'translateY(-50%)',
                  background: '#007acc',
                  color: 'white',
                  border: 'none',
                  borderRadius: '4px',
                  padding: '4px 8px',
                  fontSize: '12px',
                  cursor: 'pointer',
                  transition: 'background 0.2s ease'
                }}
                onMouseEnter={(e) => {
                  (e.target as HTMLElement).style.background = '#005999';
                }}
                onMouseLeave={(e) => {
                  if ((e.target as HTMLElement).textContent !== '✓') {
                    (e.target as HTMLElement).style.background = '#007acc';
                  }
                }}
                title="Copy path to clipboard"
              >
                ⧉
              </button>
            </div>
            <p style={{ marginTop: '12px', fontSize: '12px', color: '#666' }}>
              💡 Drag and drop this file here, or click "Choose File" to browse for it
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

// Named and default exports for flexibility
export { FileDropZone };
export default FileDropZone;
