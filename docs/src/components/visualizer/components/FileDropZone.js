/**
 * File Drop Zone Component
 * 
 * Handles file upload via drag-and-drop or file input
 */

import React, { useState, useCallback } from 'react';
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
        } catch (error) {
          alert('Invalid JSON file: ' + error.message);
        }
      };
      reader.readAsText(jsonFile);
    } else {
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
        } catch (error) {
          alert('Invalid JSON file: ' + error.message);
        }
      };
      reader.readAsText(file);
    }
  }, [onFileLoad]);

  if (hasData) {
    return null;
  }

  return (
    <div 
      className={`${styles.dropZone} ${isDragOver ? styles.dragOver : ''}`}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      style={{ backgroundColor: '#fff', border: '3px dashed #ccc' }}
    >
      <div className={styles.dropContent}>
        <h3>Hydro Graph Visualizer</h3>
        <p>Drop a Hydro ReactFlow JSON file here or click to select</p>
        <input 
          type="file" 
          accept=".json"
          onChange={handleFileInput}
          className={styles.fileInput}
          id="file-input"
        />
        <label htmlFor="file-input" className={styles.fileInputLabel}>
          Choose File
        </label>
        <div className={styles.helpText}>
          <p>Generate JSON files using:</p>
          <code>built_flow.reactflow_to_file("graph.json")</code>
        </div>
      </div>
    </div>
  );
}
