import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
/**
 * @fileoverview Working Demo Page - REAL ELK + ReactFlow Integration
 *
 * This is a fully functional demo that:
 * 1. Loads real graph data (including chat.json subset)
 * 2. Runs actual ELK layout via our ELKBridge
 * 3. Renders with actual ReactFlow via our ReactFlowBridge
 * 4. Demonstrates the hyperedge layout fix in action
 */
import { useState, useEffect } from 'react';
import { ReactFlow, Background, Controls, MiniMap } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { createVisualizationState } from '../core/VisState';
import { createVisualizationEngine } from '../core/VisualizationEngine';
import { loadGraphFromJSON, SAMPLE_CHAT_SUBSET, SAMPLE_COMPLEX_GRAPH } from '../utils/EnhancedJSONLoader';
export function SimpleDemoPage() {
    const [visState] = useState(() => createVisualizationState());
    const [engine] = useState(() => createVisualizationEngine(visState, {
        autoLayout: false, // Manual control for demo
        enableLogging: true
    }));
    const [reactFlowData, setReactFlowData] = useState(null);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState(null);
    const [selectedDataset, setSelectedDataset] = useState('chat');
    const [engineState, setEngineState] = useState(engine.getState());
    // Listen to engine state changes
    useEffect(() => {
        engine.onStateChange('demo-page', (state) => {
            setEngineState(state);
            console.log('🔄 Engine state changed:', state.phase);
        });
        return () => {
            engine.removeStateListener('demo-page');
        };
    }, [engine]);
    // Get dataset
    const getDataset = (dataset) => {
        switch (dataset) {
            case 'chat': return SAMPLE_CHAT_SUBSET;
            case 'complex': return SAMPLE_COMPLEX_GRAPH;
            case 'simple': return {
                nodes: [
                    { id: 'a', label: 'Node A', style: 'default' },
                    { id: 'b', label: 'Node B', style: 'default' },
                    { id: 'c', label: 'Node C', style: 'default' }
                ],
                edges: [
                    { id: 'e1', source: 'a', target: 'b', style: 'default' },
                    { id: 'e2', source: 'b', target: 'c', style: 'default' }
                ]
            };
            default: return SAMPLE_CHAT_SUBSET;
        }
    };
    // Load data and run the complete pipeline
    const runVisualization = async (dataset) => {
        try {
            setLoading(true);
            setError(null);
            setReactFlowData(null);
            const data = getDataset(dataset);
            console.log('� Step 1: Loading data into VisState...');
            loadGraphFromJSON(data, visState);
            console.log('📊 Step 2: Running ELK layout...');
            await engine.runLayout(); // This calls our ELKBridge!
            console.log('🎨 Step 3: Converting to ReactFlow...');
            const result = engine.getReactFlowData(); // This calls our ReactFlowBridge!
            setReactFlowData(result);
            setLoading(false);
            console.log('✅ Complete visualization pipeline finished!');
            console.log('📊 Result:', {
                nodes: result.nodes.length,
                edges: result.edges.length,
                hyperEdges: result.edges.filter(e => e.type === 'hyper').length
            });
        }
        catch (err) {
            console.error('❌ Visualization pipeline failed:', err);
            setError(err instanceof Error ? err.message : String(err));
            setLoading(false);
        }
    };
    // Load initial data
    useEffect(() => {
        runVisualization('chat');
    }, []);
    const handleDatasetChange = (dataset) => {
        setSelectedDataset(dataset);
        runVisualization(dataset);
    };
    return (_jsxs("div", { style: { padding: '20px', height: '100vh', display: 'flex', flexDirection: 'column' }, children: [_jsxs("div", { style: { marginBottom: '20px' }, children: [_jsx("h1", { style: { margin: '0 0 16px 0', color: '#333' }, children: "\uD83D\uDE80 Real ELK + ReactFlow Demo" }), _jsxs("div", { style: {
                            display: 'flex',
                            gap: '16px',
                            alignItems: 'center',
                            padding: '16px',
                            background: '#f8f9fa',
                            borderRadius: '8px',
                            border: '1px solid #e9ecef',
                            flexWrap: 'wrap'
                        }, children: [_jsx("div", { style: { fontSize: '14px', color: '#666', minWidth: '120px' }, children: _jsx("strong", { children: "Dataset:" }) }), _jsx("button", { onClick: () => handleDatasetChange('simple'), style: {
                                    padding: '8px 16px',
                                    background: selectedDataset === 'simple' ? '#007bff' : '#6c757d',
                                    color: 'white',
                                    border: 'none',
                                    borderRadius: '4px',
                                    cursor: 'pointer',
                                    fontSize: '14px'
                                }, children: "Simple (3 nodes)" }), _jsx("button", { onClick: () => handleDatasetChange('chat'), style: {
                                    padding: '8px 16px',
                                    background: selectedDataset === 'chat' ? '#007bff' : '#6c757d',
                                    color: 'white',
                                    border: 'none',
                                    borderRadius: '4px',
                                    cursor: 'pointer',
                                    fontSize: '14px'
                                }, children: "Chat System (10 nodes)" }), _jsx("button", { onClick: () => handleDatasetChange('complex'), style: {
                                    padding: '8px 16px',
                                    background: selectedDataset === 'complex' ? '#007bff' : '#6c757d',
                                    color: 'white',
                                    border: 'none',
                                    borderRadius: '4px',
                                    cursor: 'pointer',
                                    fontSize: '14px'
                                }, children: "Complex Pipeline (10 nodes)" }), _jsxs("div", { style: {
                                    marginLeft: 'auto',
                                    padding: '8px 12px',
                                    background: engineState.phase === 'displayed' ? '#d4edda' :
                                        engineState.phase === 'error' ? '#f8d7da' :
                                            loading ? '#fff3cd' : '#e2e3e5',
                                    borderRadius: '4px',
                                    fontSize: '12px',
                                    color: engineState.phase === 'displayed' ? '#155724' :
                                        engineState.phase === 'error' ? '#721c24' :
                                            loading ? '#856404' : '#6c757d'
                                }, children: ["Status: ", loading ? 'Processing...' : engineState.phase, " | Layouts: ", engineState.layoutCount] })] }), _jsxs("div", { style: {
                            marginTop: '12px',
                            padding: '12px',
                            background: '#e8f4fd',
                            borderRadius: '6px',
                            fontSize: '12px',
                            color: '#0056b3'
                        }, children: [_jsx("strong", { children: "\uD83D\uDD25 REAL Bridge Architecture:" }), " VisState \u2192 ELKBridge (with hyperedges!) \u2192 ReactFlowBridge \u2192 Display", reactFlowData && (_jsxs("span", { style: { marginLeft: '16px' }, children: ["| Nodes: ", reactFlowData.nodes.length, "| Edges: ", reactFlowData.edges.length, "| Hyperedges: ", reactFlowData.edges.filter(e => e.type === 'hyper').length] }))] })] }), error && (_jsxs("div", { style: {
                    padding: '16px',
                    background: '#ffe6e6',
                    border: '1px solid #ff9999',
                    borderRadius: '8px',
                    marginBottom: '20px',
                    color: '#cc0000'
                }, children: [_jsx("strong", { children: "Pipeline Error:" }), " ", error, _jsx("div", { style: { marginTop: '8px' }, children: _jsx("button", { onClick: () => runVisualization(selectedDataset), style: {
                                padding: '4px 12px',
                                background: '#dc3545',
                                color: 'white',
                                border: 'none',
                                borderRadius: '4px',
                                cursor: 'pointer',
                                fontSize: '12px'
                            }, children: "Retry Pipeline" }) })] })), _jsx("div", { style: {
                    flex: 1,
                    border: '2px solid #ddd',
                    borderRadius: '8px',
                    overflow: 'hidden',
                    position: 'relative'
                }, children: loading ? (_jsx("div", { style: {
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        height: '100%',
                        background: '#f5f5f5'
                    }, children: _jsxs("div", { style: { textAlign: 'center' }, children: [_jsx("div", { style: { fontSize: '48px', marginBottom: '16px' }, children: engineState.phase === 'laying_out' ? '�' :
                                    engineState.phase === 'rendering' ? '🎨' : '�🔄' }), _jsxs("div", { style: { fontSize: '18px', color: '#666', marginBottom: '8px' }, children: [engineState.phase === 'laying_out' && 'Running ELK Layout Engine...', engineState.phase === 'rendering' && 'Converting to ReactFlow...', engineState.phase === 'initial' && 'Initializing Pipeline...'] }), _jsxs("div", { style: { fontSize: '14px', color: '#999' }, children: [engineState.phase === 'laying_out' && '🔥 Including ALL edges (regular + hyperedges)', engineState.phase === 'rendering' && '🌉 Translating coordinates via bridges', engineState.phase === 'initial' && 'Loading data into VisState...'] })] }) })) : reactFlowData ? (_jsxs(_Fragment, { children: [_jsxs("div", { style: {
                                position: 'absolute',
                                top: '10px',
                                left: '10px',
                                zIndex: 1000,
                                background: 'rgba(255, 255, 255, 0.95)',
                                padding: '10px 12px',
                                borderRadius: '6px',
                                border: '1px solid #ddd',
                                fontSize: '12px',
                                color: '#666',
                                boxShadow: '0 2px 4px rgba(0,0,0,0.1)'
                            }, children: [_jsx("div", { children: _jsx("strong", { children: "Real ELK + ReactFlow" }) }), _jsxs("div", { children: ["Nodes: ", reactFlowData.nodes.length] }), _jsxs("div", { children: ["Edges: ", reactFlowData.edges.length] }), _jsxs("div", { children: ["Hyperedges: ", reactFlowData.edges.filter(e => e.type === 'hyper').length] }), _jsxs("div", { children: ["Layouts: ", engineState.layoutCount] })] }), _jsx("div", { style: {
                                position: 'absolute',
                                top: '10px',
                                right: '10px',
                                zIndex: 1000
                            }, children: _jsx("button", { onClick: () => runVisualization(selectedDataset), style: {
                                    padding: '8px 12px',
                                    background: '#28a745',
                                    color: 'white',
                                    border: 'none',
                                    borderRadius: '4px',
                                    cursor: 'pointer',
                                    fontSize: '12px',
                                    boxShadow: '0 2px 4px rgba(0,0,0,0.1)'
                                }, children: "\uD83D\uDD04 Re-run Pipeline" }) }), _jsxs(ReactFlow, { nodes: reactFlowData.nodes, edges: reactFlowData.edges, fitView: true, fitViewOptions: { padding: 0.1, maxZoom: 1.2 }, attributionPosition: "bottom-left", nodesDraggable: true, nodesConnectable: false, elementsSelectable: true, panOnDrag: true, zoomOnScroll: true, minZoom: 0.1, maxZoom: 2, children: [_jsx(Background, { color: "#ccc" }), _jsx(Controls, {}), _jsx(MiniMap, { nodeColor: "#666", nodeStrokeWidth: 2, position: "bottom-right" })] })] })) : (_jsx("div", { style: {
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        height: '100%',
                        background: '#f9f9f9'
                    }, children: _jsxs("div", { style: { textAlign: 'center', color: '#666' }, children: [_jsx("div", { style: { fontSize: '48px', marginBottom: '16px' }, children: "\uD83D\uDCCA" }), _jsx("div", { children: "Select a dataset above to run the pipeline" })] }) })) }), _jsx("div", { style: {
                    marginTop: '16px',
                    padding: '12px',
                    background: '#f8f9fa',
                    borderRadius: '6px',
                    fontSize: '12px',
                    color: '#666'
                }, children: _jsxs("div", { style: { display: 'flex', justifyContent: 'space-between', flexWrap: 'wrap', gap: '16px' }, children: [_jsxs("div", { children: [_jsx("strong", { children: "\uD83D\uDCA1 Hyperedge Fix:" }), " Try \"Complex Pipeline\" to see collapsed containers connecting to external nodes via hyperedges (no overlaps!)"] }), _jsxs("div", { children: [_jsx("strong", { children: "\u26A1 Pipeline:" }), " ", engineState.phase, engineState.lastUpdate && ` (${new Date(engineState.lastUpdate).toLocaleTimeString()})`] })] }) })] }));
}
/**
 * Export for easy integration
 */
export default SimpleDemoPage;
//# sourceMappingURL=SimpleDemoPage.js.map