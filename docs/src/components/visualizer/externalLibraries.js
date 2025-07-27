/**
 * External Library Loader for ReactFlow and ELK
 * 
 * Manages loading of ReactFlow and ELK.js libraries from CDN
 * in a way that's compatible with React 18 and Docusaurus
 */

import React from 'react';

let ReactFlowComponents = null;
let ELK = null;

export const loadExternalLibraries = async () => {
  // Ensure React/ReactDOM are available globally for ReactFlow
  if (!window.React) {
    window.React = React;
  }
  
  if (!window.ReactDOM) {
    const ReactDOM = await import('react-dom');
    window.ReactDOM = ReactDOM;
  }
  
  // Load ReactFlow (compatible with React 18)
  if (!window.ReactFlow) {
    const reactFlowScript = document.createElement('script');
    reactFlowScript.src = 'https://unpkg.com/reactflow@11.11.4/dist/umd/index.js';
    document.head.appendChild(reactFlowScript);
    
    await new Promise((resolve, reject) => {
      reactFlowScript.onload = resolve;
      reactFlowScript.onerror = reject;
    });
  }
  
  // Load ELK.js for advanced layouts
  if (!window.ELK) {
    const elkScript = document.createElement('script');
    elkScript.src = 'https://unpkg.com/elkjs@0.8.2/lib/elk.bundled.js';
    document.head.appendChild(elkScript);
    
    await new Promise((resolve, reject) => {
      elkScript.onload = resolve;
      elkScript.onerror = reject;
    });
    
    ELK = new window.ELK();
  } else {
    ELK = new window.ELK();
  }
  
  // Load CSS
  if (!document.querySelector('link[href*="reactflow"]')) {
    const link = document.createElement('link');
    link.rel = 'stylesheet';
    link.href = 'https://unpkg.com/reactflow@11.11.4/dist/style.css';
    document.head.appendChild(link);
  }
  
  // Extract ReactFlow components
  const ReactFlowLib = window.ReactFlow;
  const { 
    default: ReactFlowComponent, 
    Controls, 
    MiniMap, 
    Background, 
    Handle,
    Position,
    useNodesState, 
    useEdgesState, 
    addEdge, 
    applyNodeChanges, 
    applyEdgeChanges,
    getBezierPath
  } = ReactFlowLib;
  
  ReactFlowComponents = {
    ReactFlow: ReactFlowComponent,
    Controls,
    MiniMap,
    Background,
    Handle,
    Position,
    useNodesState,
    useEdgesState,
    addEdge,
    applyNodeChanges,
    applyEdgeChanges,
    getBezierPath
  };
  
  return { ReactFlowComponents, ELK };
};

export { ReactFlowComponents, ELK };
