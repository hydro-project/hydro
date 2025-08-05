import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
/**
 * Custom Group Node Component for ReactFlow
 *
 * Displays group nodes with labels for hierarchy containers
 * Avoids ReactFlow's built-in group styling that causes shadows
 */
import React from 'react';
import { Handle, Position } from '@xyflow/react';
import { COLORS, COMPONENT_COLORS } from '../utils/constants.js';
import { REQUIRED_HANDLE_IDS } from '../utils/handleValidation.js';
import { truncateContainerName } from '../utils/utils.js';
import { getContainerHandles } from '../utils/handleStyles.js';
export function GroupNode(props) {
    // In ReactFlow v12, custom components receive: id, data, width, height
    // No style prop! Get styling from data.nodeStyle
    const { id, data, width, height } = props;
    // Truncate the container label for display
    const fullLabel = data?.label || 'Container';
    const displayLabel = truncateContainerName(fullLabel, 15, {
        side: 'left',
        splitOnDelimiter: true,
        delimiterPenalty: 0.2
    });
    const showTooltip = fullLabel !== displayLabel;
    // Get style from data.nodeStyle where we stored it
    const nodeStyle = data?.nodeStyle || {};
    const effectiveWidth = width || nodeStyle.width || 300;
    const effectiveHeight = height || nodeStyle.height || 200;
    if (!effectiveWidth || !effectiveHeight || !data) {
        console.warn('[GroupNode] Missing required props:', { width: effectiveWidth, height: effectiveHeight, data: !!data, nodeId: id });
        // Return a simple fallback instead of null to see what's happening
        return (_jsx("div", { style: {
                width: 200,
                height: 100,
                background: 'red',
                border: '2px solid black',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color: 'white',
                fontWeight: 'bold'
            }, children: "ERROR: Missing Props" }));
    }
    // Use the style object directly from our processed nodes, with fallbacks
    const containerStyle = {
        width: effectiveWidth,
        height: effectiveHeight,
        // Use hardcoded styles based on node ID since ReactFlow isn't passing them through
        background: getBackgroundColor(id),
        border: getBorderColor(id),
        borderRadius: '8px',
        // Remove padding to test if this is causing the inset
        fontSize: '14px',
        fontWeight: 'bold',
        color: getTextColor(id),
        zIndex: 1,
        boxSizing: 'border-box',
        position: 'relative',
        display: 'flex',
        alignItems: 'flex-end',
        justifyContent: 'flex-end',
        minWidth: '200px',
        minHeight: '100px',
    };
    // Helper functions to get colors based on node ID
    function getBackgroundColor(nodeId) {
        if (nodeId === 'cloud')
            return COLORS.CONTAINER_L0;
        if (nodeId === 'region')
            return COLORS.CONTAINER_L1;
        if (nodeId?.startsWith('az'))
            return COLORS.CONTAINER_L2;
        return COLORS.CONTAINER_L0; // default
    }
    function getBorderColor(nodeId) {
        if (nodeId === 'cloud')
            return `3px solid ${COLORS.CONTAINER_BORDER_L0}`;
        if (nodeId === 'region')
            return `3px solid ${COLORS.CONTAINER_BORDER_L1}`;
        if (nodeId?.startsWith('az'))
            return `3px solid ${COLORS.CONTAINER_BORDER_L2}`;
        return `3px solid ${COLORS.CONTAINER_BORDER_L0}`; // default
    }
    function getTextColor(nodeId) {
        if (nodeId === 'cloud')
            return COLORS.CONTAINER_BORDER_L0;
        if (nodeId === 'region')
            return COLORS.CONTAINER_BORDER_L1;
        if (nodeId?.startsWith('az'))
            return COLORS.CONTAINER_BORDER_L2;
        return COLORS.CONTAINER_BORDER_L0; // default
    }
    return (_jsxs("div", { style: containerStyle, children: [_jsx("div", { style: {
                    position: 'absolute',
                    bottom: '4px',
                    right: '4px',
                    fontSize: '14px',
                    fontWeight: 'bold',
                    color: getTextColor(id),
                    backgroundColor: COLORS.GRAY_50,
                    padding: '4px 8px',
                    borderRadius: '4px',
                    border: `1px solid ${getTextColor(id)}`,
                    zIndex: 10,
                }, title: showTooltip ? fullLabel : undefined, children: displayLabel }), _jsx("div", { style: {
                    position: 'absolute',
                    top: '4px',
                    right: '4px',
                    width: '16px',
                    height: '16px',
                    background: getTextColor(id),
                    color: COMPONENT_COLORS.TEXT_INVERSE,
                    borderRadius: '50%',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    fontSize: '12px',
                    fontWeight: 'bold',
                    cursor: 'pointer',
                    zIndex: 10,
                }, title: "Click to collapse", children: "\u2212" }), getContainerHandles().map(handleProps => (_jsx(Handle, { ...handleProps }, handleProps.id)))] }));
}
//# sourceMappingURL=GroupNode.js.map