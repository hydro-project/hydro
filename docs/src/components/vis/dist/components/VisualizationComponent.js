import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { ReactFlow, Background, Controls, MiniMap } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { useVisualization } from '../hooks/useVisualization';
export function VisualizationComponent({ visState, config, className = '', style = {} }) {
    const { reactFlowData, engineState, runLayout, visualize, onDataChanged, isLoading, isReady, hasError, error } = useVisualization(visState, config);
    // Loading state
    if (isLoading) {
        return (_jsx("div", { className: `visualization-loading ${className}`, style: {
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                minHeight: '400px',
                background: '#f5f5f5',
                border: '1px solid #ddd',
                borderRadius: '8px',
                ...style
            }, children: _jsxs("div", { style: { textAlign: 'center' }, children: [_jsx("div", { style: { fontSize: '24px', marginBottom: '16px' }, children: "\uD83D\uDD04" }), _jsxs("div", { style: { fontSize: '16px', color: '#666' }, children: [engineState.phase === 'laying_out' && 'Running layout...', engineState.phase === 'rendering' && 'Generating visualization...', engineState.phase === 'initial' && 'Initializing...'] }), _jsxs("div", { style: { fontSize: '12px', color: '#999', marginTop: '8px' }, children: ["Phase: ", engineState.phase, " | Layouts: ", engineState.layoutCount] })] }) }));
    }
    // Error state
    if (hasError) {
        return (_jsx("div", { className: `visualization-error ${className}`, style: {
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                minHeight: '400px',
                background: '#ffe6e6',
                border: '1px solid #ff9999',
                borderRadius: '8px',
                ...style
            }, children: _jsxs("div", { style: { textAlign: 'center', maxWidth: '500px' }, children: [_jsx("div", { style: { fontSize: '24px', marginBottom: '16px' }, children: "\u274C" }), _jsx("div", { style: { fontSize: '16px', color: '#cc0000', marginBottom: '12px' }, children: "Visualization Error" }), _jsx("div", { style: { fontSize: '14px', color: '#666', marginBottom: '16px' }, children: error }), _jsx("button", { onClick: visualize, style: {
                            padding: '8px 16px',
                            background: '#007bff',
                            color: 'white',
                            border: 'none',
                            borderRadius: '4px',
                            cursor: 'pointer'
                        }, children: "Retry" })] }) }));
    }
    // No data state
    if (!reactFlowData) {
        return (_jsx("div", { className: `visualization-empty ${className}`, style: {
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                minHeight: '400px',
                background: '#f9f9f9',
                border: '1px solid #ddd',
                borderRadius: '8px',
                ...style
            }, children: _jsxs("div", { style: { textAlign: 'center' }, children: [_jsx("div", { style: { fontSize: '24px', marginBottom: '16px' }, children: "\uD83D\uDCCA" }), _jsx("div", { style: { fontSize: '16px', color: '#666', marginBottom: '12px' }, children: "Ready to visualize" }), _jsx("button", { onClick: visualize, style: {
                            padding: '8px 16px',
                            background: '#28a745',
                            color: 'white',
                            border: 'none',
                            borderRadius: '4px',
                            cursor: 'pointer'
                        }, children: "Generate Visualization" })] }) }));
    }
    // Success state - render ReactFlow
    return (_jsxs("div", { className: `visualization-display ${className}`, style: {
            height: '600px',
            border: '1px solid #ddd',
            borderRadius: '8px',
            overflow: 'hidden',
            ...style
        }, children: [_jsxs("div", { style: {
                    padding: '12px 16px',
                    background: '#f8f9fa',
                    borderBottom: '1px solid #ddd',
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center'
                }, children: [_jsxs("div", { style: { fontSize: '14px', color: '#666' }, children: ["Nodes: ", reactFlowData.nodes.length, " | Edges: ", reactFlowData.edges.length, " | Layouts: ", engineState.layoutCount] }), _jsxs("div", { style: { display: 'flex', gap: '8px' }, children: [_jsx("button", { onClick: runLayout, style: {
                                    padding: '4px 12px',
                                    background: '#007bff',
                                    color: 'white',
                                    border: 'none',
                                    borderRadius: '4px',
                                    cursor: 'pointer',
                                    fontSize: '12px'
                                }, children: "Re-layout" }), _jsx("button", { onClick: onDataChanged, style: {
                                    padding: '4px 12px',
                                    background: '#6c757d',
                                    color: 'white',
                                    border: 'none',
                                    borderRadius: '4px',
                                    cursor: 'pointer',
                                    fontSize: '12px'
                                }, children: "Refresh" })] })] }), _jsx("div", { style: { height: 'calc(100% - 57px)' }, children: _jsxs(ReactFlow, { nodes: reactFlowData.nodes, edges: reactFlowData.edges, fitView: true, attributionPosition: "bottom-left", children: [_jsx(Background, {}), _jsx(Controls, {}), _jsx(MiniMap, {})] }) })] }));
}
export function ExampleVisualization({ visState }) {
    return (_jsxs("div", { style: { padding: '20px' }, children: [_jsx("h2", { style: { marginBottom: '20px' }, children: "\uD83C\uDF09 New Bridge Architecture Demo" }), _jsxs("div", { style: { marginBottom: '16px', padding: '16px', background: '#e8f4fd', borderRadius: '8px' }, children: [_jsx("h3", { style: { margin: '0 0 8px 0', color: '#0056b3' }, children: "\u2728 Features Demonstrated:" }), _jsxs("ul", { style: { margin: 0, paddingLeft: '20px', color: '#666' }, children: [_jsxs("li", { children: ["\uD83D\uDD27 ", _jsx("strong", { children: "ELKBridge" }), ": Includes ALL edges (regular + hyperedges)"] }), _jsxs("li", { children: ["\uD83C\uDFA8 ", _jsx("strong", { children: "ReactFlowBridge" }), ": Clean coordinate translation"] }), _jsxs("li", { children: ["\u26A1 ", _jsx("strong", { children: "VisualizationEngine" }), ": State machine orchestration"] }), _jsxs("li", { children: ["\uD83C\uDFAF ", _jsx("strong", { children: "React Integration" }), ": Hooks and error handling"] })] })] }), _jsx(VisualizationComponent, { visState: visState, config: {
                    autoLayout: true,
                    autoVisualize: true,
                    enableLogging: true
                }, style: { height: '700px' } })] }));
}
//# sourceMappingURL=VisualizationComponent.js.map