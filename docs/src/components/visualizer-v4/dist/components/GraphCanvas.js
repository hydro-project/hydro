import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
/**
 * Graph Canvas Component
 *
 * Advanced graph visualizer with layout controls and state management
 */
import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { useNodesState, useEdgesState, applyNodeChanges, applyEdgeChanges } from '@xyflow/react';
import { applyLayout } from '../utils/layout.js';
import { LayoutControls } from './LayoutControls.js';
import { Legend } from './Legend.js';
import { ReactFlowInner } from './ReactFlowInner.js';
import { processGraphData } from '../utils/reactFlowConfig.js';
import styles from '../../../pages/visualizer.module.css';
export function GraphCanvas({ graphData, maxVisibleNodes = 50 }) {
    const [nodes, setNodes] = useState([]);
    const [edges, setEdges] = useState([]);
    const [currentLayout, setCurrentLayout] = useState('mrtree');
    const [colorPalette, setColorPalette] = useState('Set3');
    // Simple change handlers
    const onNodesChange = useCallback((changes) => {
        setNodes((nds) => {
            // Filter out automatic dimensions during layout
            const validChanges = changes.filter(change => change.type !== 'dimensions' || change.resizing);
            return validChanges.length > 0 ? applyNodeChanges(validChanges, nds) : nds;
        });
    }, []);
    const onEdgesChange = useCallback((changes) => {
        setEdges((eds) => applyEdgeChanges(changes, eds));
    }, []);
    // Optional: Keep locationData for internal tracking but remove from visualization components
    const locationData = useMemo(() => {
        const locations = new Map();
        // Only process location data if it exists in the graph data
        if (graphData?.locations) {
            graphData.locations.forEach(location => {
                if (location && typeof location.id !== 'undefined') {
                    locations.set(parseInt(location.id, 10), location);
                }
            });
        }
        // Optional: process node location data if present
        (graphData?.nodes || []).forEach(node => {
            if (node.data?.locationId !== undefined && node.data?.location && !locations.has(node.data.locationId)) {
                locations.set(node.data.locationId, { id: node.data.locationId, label: node.data.location });
            }
        });
        return locations;
    }, [graphData]);
    // Process graph data when data changes
    useEffect(() => {
        if (!graphData) {
            return;
        }
        const processData = async () => {
            try {
                const result = await processGraphData(graphData, colorPalette, currentLayout, applyLayout);
                setNodes(result.nodes);
                setEdges(result.edges);
            }
            catch (error) {
                console.error('🚨 LAYOUT ERROR:', error);
                // Fallback to original data
                setNodes(graphData.nodes || []);
                setEdges(graphData.edges || []);
            }
        };
        processData();
    }, [graphData, currentLayout, colorPalette]);
    const handleLayoutChange = useCallback((newLayout) => {
        setCurrentLayout(newLayout);
    }, []);
    const handlePaletteChange = useCallback((newPalette) => {
        setColorPalette(newPalette);
    }, []);
    if (!nodes.length && graphData?.nodes?.length) {
        return _jsx("div", { className: styles.loading, children: "Preparing visualization..." });
    }
    return (_jsxs("div", { className: styles.visualizationWrapper, children: [_jsx(LayoutControls, { currentLayout: currentLayout, onLayoutChange: handleLayoutChange, colorPalette: colorPalette, onPaletteChange: handlePaletteChange }), _jsx(Legend, { colorPalette: colorPalette, graphData: graphData }), _jsx(ReactFlowInner, { nodes: nodes, edges: edges, onNodesChange: onNodesChange, onEdgesChange: onEdgesChange, colorPalette: colorPalette })] }));
}
//# sourceMappingURL=GraphCanvas.js.map