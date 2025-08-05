import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
/**
 * @fileoverview Bridge-Based GraphFlow Component
 *
 * Complete replacement for alpha GraphFlow using our bridge architecture.
 * Maintains identical API while using the new VisualizationEngine internally.
 */
import { useEffect, useState, useCallback } from 'react';
import { ReactFlow, Background, Controls, MiniMap } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { createVisualizationEngine } from '../core/VisualizationEngine';
import { ReactFlowConverter } from './ReactFlowConverter';
import { DEFAULT_RENDER_CONFIG } from './config';
import { nodeTypes } from './nodes';
import { edgeTypes } from './edges';
export function GraphFlow({ visualizationState, config = DEFAULT_RENDER_CONFIG, eventHandlers, className, style }) {
    const [reactFlowData, setReactFlowData] = useState(null);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState(null);
    // Create converter and engine
    const [converter] = useState(() => new ReactFlowConverter());
    const [engine] = useState(() => createVisualizationEngine(visualizationState, {
        autoLayout: true, // Always auto-layout for alpha compatibility
        enableLogging: false
    }));
    // Listen to visualization state changes
    useEffect(() => {
        const handleStateChange = async () => {
            try {
                setLoading(true);
                setError(null);
                console.log('[GraphFlow] 🔄 Visualization state changed, updating...');
                // Run layout
                await engine.runLayout();
                // Convert to ReactFlow format
                const data = converter.convert(visualizationState);
                setReactFlowData(data);
                console.log('[GraphFlow] ✅ Updated ReactFlow data:', {
                    nodes: data.nodes.length,
                    edges: data.edges.length
                });
            }
            catch (err) {
                console.error('[GraphFlow] ❌ Failed to update visualization:', err);
                setError(err instanceof Error ? err.message : String(err));
            }
            finally {
                setLoading(false);
            }
        };
        // Initial render
        handleStateChange();
        // For alpha compatibility, we just do initial render
        // Real change detection would be implemented with proper state listeners
    }, [visualizationState, engine, converter]);
    // Handle node events
    const onNodeClick = useCallback((event, node) => {
        console.log('[GraphFlow] 🖱️ Node clicked:', node.id);
        eventHandlers?.onNodeClick?.(event, node);
    }, [eventHandlers]);
    const onEdgeClick = useCallback((event, edge) => {
        console.log('[GraphFlow] 🖱️ Edge clicked:', edge.id);
        eventHandlers?.onEdgeClick?.(event, edge);
    }, [eventHandlers]);
    // Loading state
    if (loading && !reactFlowData) {
        return (_jsx("div", { className: className, style: {
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                height: '400px',
                background: '#f5f5f5',
                border: '1px solid #ddd',
                borderRadius: '8px',
                ...style
            }, children: _jsxs("div", { style: { textAlign: 'center', color: '#666' }, children: [_jsx("div", { style: { fontSize: '24px', marginBottom: '8px' }, children: "\uD83D\uDD04" }), _jsx("div", { children: "Running layout..." })] }) }));
    }
    // Error state
    if (error) {
        return (_jsx("div", { className: className, style: {
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                height: '400px',
                background: '#ffe6e6',
                border: '1px solid #ff9999',
                borderRadius: '8px',
                ...style
            }, children: _jsxs("div", { style: { textAlign: 'center', color: '#cc0000' }, children: [_jsx("div", { style: { fontSize: '24px', marginBottom: '8px' }, children: "\u274C" }), _jsx("div", { children: _jsx("strong", { children: "Visualization Error:" }) }), _jsx("div", { style: { fontSize: '14px', marginTop: '4px' }, children: error })] }) }));
    }
    // No data state
    if (!reactFlowData) {
        return (_jsx("div", { className: className, style: {
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                height: '400px',
                background: '#f9f9f9',
                border: '1px solid #ddd',
                borderRadius: '8px',
                ...style
            }, children: _jsxs("div", { style: { textAlign: 'center', color: '#666' }, children: [_jsx("div", { style: { fontSize: '24px', marginBottom: '8px' }, children: "\uD83D\uDCCA" }), _jsx("div", { children: "No visualization data" })] }) }));
    }
    // Main ReactFlow render
    return (_jsxs("div", { className: className, style: { height: '400px', ...style }, children: [_jsxs(ReactFlow, { nodes: reactFlowData.nodes, edges: reactFlowData.edges, nodeTypes: nodeTypes, edgeTypes: edgeTypes, onNodeClick: onNodeClick, onEdgeClick: onEdgeClick, fitView: config.fitView !== false, fitViewOptions: { padding: 0.1, maxZoom: 1.2 }, attributionPosition: "bottom-left", nodesDraggable: config.nodesDraggable !== false, nodesConnectable: config.nodesConnectable !== false, elementsSelectable: config.elementsSelectable !== false, panOnDrag: config.enablePan !== false, zoomOnScroll: config.enableZoom !== false, minZoom: 0.1, maxZoom: 2, children: [_jsx(Background, { color: "#ccc" }), config.enableControls !== false && _jsx(Controls, {}), config.enableMiniMap !== false && (_jsx(MiniMap, { nodeColor: "#666", nodeStrokeWidth: 2, position: "bottom-right" }))] }), loading && (_jsx("div", { style: {
                    position: 'absolute',
                    top: '10px',
                    right: '10px',
                    background: 'rgba(255, 255, 255, 0.9)',
                    padding: '8px 12px',
                    borderRadius: '4px',
                    border: '1px solid #ddd',
                    fontSize: '12px',
                    color: '#666'
                }, children: "\uD83D\uDD04 Updating..." }))] }));
}
//# sourceMappingURL=GraphFlow.js.map