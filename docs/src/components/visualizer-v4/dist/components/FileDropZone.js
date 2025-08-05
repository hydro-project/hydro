import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
/**
 * File Drop Zone Component for Vis System
 *
 * Handles file upload via drag-and-drop or file input.
 * Integrates with the new vis system's JSON parser.
 */
import { useState, useCallback } from 'react';
const dropZoneStyles = {
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
const dragOverStyles = {
    ...dropZoneStyles,
    borderColor: '#007acc',
    background: '#f0f8ff',
};
const dropContentStyles = {
    textAlign: 'center',
    padding: '40px',
    maxWidth: '500px',
};
const fileInputStyles = {
    display: 'none',
};
const fileInputLabelStyles = {
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
const loadingStyles = {
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
export function FileDropZone({ onFileLoad, hasData = false, className }) {
    const [isDragOver, setIsDragOver] = useState(false);
    const [isLoading, setIsLoading] = useState(false);
    const handleDragOver = useCallback((e) => {
        e.preventDefault();
        setIsDragOver(true);
    }, []);
    const handleDragLeave = useCallback((e) => {
        e.preventDefault();
        setIsDragOver(false);
    }, []);
    const processFile = useCallback(async (file) => {
        setIsLoading(true);
        try {
            const reader = new FileReader();
            reader.onload = (event) => {
                try {
                    const data = JSON.parse(event.target?.result);
                    onFileLoad(data);
                }
                catch (error) {
                    console.error('JSON parsing error:', error);
                    alert('Invalid JSON file: ' + error.message);
                }
                finally {
                    setIsLoading(false);
                }
            };
            reader.onerror = () => {
                console.error('File reading error');
                alert('Error reading file');
                setIsLoading(false);
            };
            reader.readAsText(file);
        }
        catch (error) {
            console.error('File processing error:', error);
            alert('Error processing file: ' + error.message);
            setIsLoading(false);
        }
    }, [onFileLoad]);
    const handleDrop = useCallback((e) => {
        e.preventDefault();
        setIsDragOver(false);
        const files = Array.from(e.dataTransfer.files);
        const jsonFile = files.find(file => file.name.endsWith('.json'));
        if (jsonFile) {
            processFile(jsonFile);
        }
        else {
            alert('Please drop a JSON file');
        }
    }, [processFile]);
    const handleFileInput = useCallback((e) => {
        const file = e.target.files?.[0];
        if (file && file.name.endsWith('.json')) {
            processFile(file);
        }
        else if (file) {
            alert('Please select a JSON file');
        }
        // Reset the input so the same file can be selected again
        e.target.value = '';
    }, [processFile]);
    if (hasData) {
        return null;
    }
    if (isLoading) {
        return (_jsx("div", { style: loadingStyles, className: className, children: _jsxs("div", { children: [_jsx("div", { children: "Loading graph data..." }), _jsx("div", { style: { fontSize: '14px', color: '#999', marginTop: '8px' }, children: "Parsing JSON and building visualization state" })] }) }));
    }
    return (_jsx("div", { style: isDragOver ? dragOverStyles : dropZoneStyles, className: className, onDragOver: handleDragOver, onDragLeave: handleDragLeave, onDrop: handleDrop, children: _jsxs("div", { style: dropContentStyles, children: [_jsx("h3", { style: { marginBottom: '16px', color: '#333', fontSize: '24px' }, children: "Graph Visualization" }), _jsx("p", { style: { marginBottom: '24px', color: '#666', fontSize: '16px' }, children: "Drop a Hydro ReactFlow JSON file here or click to select" }), _jsx("input", { type: "file", accept: ".json", onChange: handleFileInput, style: fileInputStyles, id: "file-input" }), _jsx("label", { htmlFor: "file-input", style: fileInputLabelStyles, onMouseEnter: (e) => {
                        e.target.style.background = '#005999';
                    }, onMouseLeave: (e) => {
                        e.target.style.background = '#007acc';
                    }, children: "Choose File" }), _jsxs("div", { style: {
                        marginTop: '32px',
                        paddingTop: '24px',
                        borderTop: '1px solid #eee',
                        color: '#777',
                        fontSize: '14px'
                    }, children: [_jsx("p", { children: "Generate JSON files using:" }), _jsx("code", { style: {
                                display: 'block',
                                background: '#f5f5f5',
                                padding: '8px 12px',
                                borderRadius: '4px',
                                marginTop: '8px',
                                fontFamily: "'Monaco', 'Menlo', 'Ubuntu Mono', monospace",
                                color: '#d73a49'
                            }, children: "built_flow.reactflow_to_file(\"graph.json\")" }), _jsx("p", { style: { marginTop: '16px', fontSize: '12px' }, children: "Supports the new framework-independent visualization system" })] })] }) }));
}
export default FileDropZone;
//# sourceMappingURL=FileDropZone.js.map