import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
/**
 * ReactFlow Inner Component
 *
 * Core ReactFlow integration with custom node types
 */
import React, { useCallback, useMemo, useRef, useEffect } from 'react';
import { ReactFlow, ReactFlowProvider, Controls, Background, MiniMap, Handle, Position, addEdge, useReactFlow, } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { DEFAULT_EDGE_OPTIONS, REACTFLOW_CONFIG, BACKGROUND_CONFIG, MINIMAP_CONFIG, DEFAULT_VIEWPORT, getMiniMapNodeColor } from '../utils/reactFlowConfig.js';
import { GroupNode } from './GroupNode.js';
import { CollapsedContainerNode } from '../containers/CollapsedContainerNode.js';
import { enforceHandleConsistency, REQUIRED_HANDLE_IDS } from '../utils/handleValidation.js';
import { COLORS, COMPONENT_COLORS } from '../utils/constants.js';
import styles from '../../../pages/visualizer.module.css';
export function ReactFlowInner({ nodes, edges, onNodesChange, onEdgesChange, colorPalette, onNodeClick }) {
    const onConnect = useCallback((connection) => {
        onEdgesChange(addEdge(connection, edges));
    }, [onEdgesChange, edges]);
    // Component to handle ReactFlow instance setup - must be inside ReactFlow provider
    const ReactFlowInstanceHandler = React.memo(() => {
        const reactFlowInstance = useReactFlow();
        useEffect(() => {
            let lastFitViewCall = 0;
            const MIN_FIT_VIEW_INTERVAL = 100; // Minimum time between fitView calls
            // Handle custom fitView requests with proper debouncing
            const handleFitViewRequest = (event) => {
                const { padding, duration, minZoom, maxZoom, operationName, timestamp } = event.detail;
                const now = Date.now();
                // Debounce rapid fitView calls
                if (now - lastFitViewCall < MIN_FIT_VIEW_INTERVAL) {
                    return;
                }
                lastFitViewCall = now;
                try {
                    // Use a combination of requestAnimationFrame and setTimeout for maximum stability
                    requestAnimationFrame(() => {
                        setTimeout(() => {
                            try {
                                reactFlowInstance.fitView({
                                    padding,
                                    duration,
                                    minZoom,
                                    maxZoom
                                });
                            }
                            catch (error) {
                                console.warn(`[ReactFlowInner] fitView failed for ${operationName}:`, error);
                            }
                        }, 50); // Small additional delay
                    });
                }
                catch (error) {
                    console.warn(`[ReactFlowInner] fitView setup failed for ${operationName}:`, error);
                }
            };
            // Listen for custom fitView events
            window.addEventListener('fitViewRequest', handleFitViewRequest);
            // Keep the global reference for backwards compatibility
            window.reactFlowInstance = reactFlowInstance;
            return () => {
                window.removeEventListener('fitViewRequest', handleFitViewRequest);
                window.reactFlowInstance = null;
            };
        }, [reactFlowInstance]);
        return null; // This component doesn't render anything
    });
    // Custom default node component - simplified to fill the container
    const DefaultNode = useCallback((props) => {
        const { data } = props;
        const nodeStyle = data?.nodeStyle || props.style || {};
        // Use the background from nodeStyle
        const background = nodeStyle.gradient || nodeStyle.background || COMPONENT_COLORS.BACKGROUND_SECONDARY;
        return (_jsxs("div", { style: {
                background: background,
                width: '100%',
                height: '100%',
                borderRadius: '8px',
                color: nodeStyle.color || COMPONENT_COLORS.TEXT_INVERSE,
                fontSize: '13px',
                fontWeight: '600',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                textAlign: 'center',
                cursor: 'grab',
                padding: '6px 10px',
                boxSizing: 'border-box',
            }, children: [data?.label || 'Node', _jsx(Handle, { id: REQUIRED_HANDLE_IDS.source, type: "source", position: Position.Right, style: { background: COMPONENT_COLORS.HANDLE_DEFAULT, border: 'none', width: 8, height: 8 } }), _jsx(Handle, { id: REQUIRED_HANDLE_IDS.target, type: "target", position: Position.Left, style: { background: COMPONENT_COLORS.HANDLE_DEFAULT, border: 'none', width: 8, height: 8 } }), _jsx(Handle, { id: REQUIRED_HANDLE_IDS.sourceBottom, type: "source", position: Position.Bottom, style: { background: COMPONENT_COLORS.HANDLE_DEFAULT, border: 'none', width: 8, height: 8 } }), _jsx(Handle, { id: REQUIRED_HANDLE_IDS.targetTop, type: "target", position: Position.Top, style: { background: COMPONENT_COLORS.HANDLE_DEFAULT, border: 'none', width: 8, height: 8 } })] }));
    }, []);
    const nodeTypes = useMemo(() => {
        const types = {
            group: GroupNode,
            collapsedContainer: CollapsedContainerNode,
            default: DefaultNode,
        };
        // CRITICAL: Log handle requirements during development
        // This helps prevent ReactFlow handle errors by documenting the requirements
        if (process.env.NODE_ENV === 'development') {
            enforceHandleConsistency(types);
        }
        return types;
    }, [DefaultNode]);
    // MiniMap node color configuration
    const miniMapNodeColor = useCallback((node) => {
        return getMiniMapNodeColor(node, colorPalette);
    }, [colorPalette]);
    return (_jsx("div", { className: styles.reactflowWrapper, children: _jsx(ReactFlowProvider, { children: _jsxs(ReactFlow, { nodes: nodes, edges: edges, onNodesChange: onNodesChange, onEdgesChange: onEdgesChange, onConnect: onConnect, onNodeClick: onNodeClick, nodeTypes: nodeTypes, defaultEdgeOptions: DEFAULT_EDGE_OPTIONS, defaultViewport: DEFAULT_VIEWPORT, ...REACTFLOW_CONFIG, children: [_jsx(ReactFlowInstanceHandler, {}), _jsx(Controls, {}), _jsx(MiniMap, { ...MINIMAP_CONFIG, nodeColor: miniMapNodeColor }), _jsx(Background, { ...BACKGROUND_CONFIG })] }) }) }));
}
//# sourceMappingURL=ReactFlowInner.js.map