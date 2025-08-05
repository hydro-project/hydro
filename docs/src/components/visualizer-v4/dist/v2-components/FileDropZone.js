import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
/**
 * File Drop Zone Component
 *
 * Handles file upload via drag-and-drop or file input
 */
import React, { useState, useCallback } from 'react';
import { COLORS } from '../utils/constants.js';
import styles from '../../../pages/visualizer.module.css';
export function FileDropZone({ onFileLoad, hasData }) {
    const [isDragOver, setIsDragOver] = useState(false);
    const handleDragOver = useCallback((e) => {
        e.preventDefault();
        setIsDragOver(true);
    }, []);
    const handleDragLeave = useCallback((e) => {
        e.preventDefault();
        setIsDragOver(false);
    }, []);
    const handleDrop = useCallback((e) => {
        e.preventDefault();
        setIsDragOver(false);
        const files = Array.from(e.dataTransfer.files);
        const jsonFile = files.find(file => file.name.endsWith('.json'));
        if (jsonFile) {
            const reader = new FileReader();
            reader.onload = (event) => {
                try {
                    const data = JSON.parse(event.target.result);
                    onFileLoad(data);
                }
                catch (error) {
                    alert('Invalid JSON file: ' + error.message);
                }
            };
            reader.readAsText(jsonFile);
        }
        else {
            alert('Please drop a JSON file');
        }
    }, [onFileLoad]);
    const handleFileInput = useCallback((e) => {
        const file = e.target.files[0];
        if (file && file.name.endsWith('.json')) {
            const reader = new FileReader();
            reader.onload = (event) => {
                try {
                    const data = JSON.parse(event.target.result);
                    onFileLoad(data);
                }
                catch (error) {
                    alert('Invalid JSON file: ' + error.message);
                }
            };
            reader.readAsText(file);
        }
    }, [onFileLoad]);
    if (hasData) {
        return null;
    }
    return (_jsx("div", { className: `${styles.dropZone} ${isDragOver ? styles.dragOver : ''}`, onDragOver: handleDragOver, onDragLeave: handleDragLeave, onDrop: handleDrop, style: { backgroundColor: COLORS.WHITE, border: `3px dashed ${COLORS.GRAY_LIGHT}` }, children: _jsxs("div", { className: styles.dropContent, children: [_jsx("h3", { children: "Hydro Graph Visualizer" }), _jsx("p", { children: "Drop a Hydro ReactFlow JSON file here or click to select" }), _jsx("input", { type: "file", accept: ".json", onChange: handleFileInput, className: styles.fileInput, id: "file-input" }), _jsx("label", { htmlFor: "file-input", className: styles.fileInputLabel, children: "Choose File" }), _jsxs("div", { className: styles.helpText, children: [_jsx("p", { children: "Generate JSON files using:" }), _jsx("code", { children: "built_flow.reactflow_to_file(\"graph.json\")" })] })] }) }));
}
//# sourceMappingURL=FileDropZone.js.map