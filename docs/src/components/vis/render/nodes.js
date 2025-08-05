import { jsx as _jsx, Fragment as _Fragment, jsxs as _jsxs } from "react/jsx-runtime";
import { Handle, Position } from '@xyflow/react';
import { getHandleConfig, CONTINUOUS_HANDLE_STYLE } from './handleConfig';
/**
 * Render handles based on current configuration
 */
function renderHandles() {
    const config = getHandleConfig();
    if (config.enableContinuousHandles) {
        // ReactFlow v12 continuous handles - connections anywhere on perimeter
        return (_jsxs(_Fragment, { children: [_jsx(Handle, { type: "source", position: Position.Top, style: CONTINUOUS_HANDLE_STYLE, isConnectable: true }), _jsx(Handle, { type: "target", position: Position.Top, style: CONTINUOUS_HANDLE_STYLE, isConnectable: true })] }));
    }
    // Discrete handles if configured
    return (_jsxs(_Fragment, { children: [config.sourceHandles.map(handle => (_jsx(Handle, { id: handle.id, type: "source", position: handle.position, style: handle.style, isConnectable: true }, handle.id))), config.targetHandles.map(handle => (_jsx(Handle, { id: handle.id, type: "target", position: handle.position, style: handle.style, isConnectable: true }, handle.id)))] }));
}
/**
 * Standard graph node component
 */
export function StandardNode({ id, data }) {
    return (_jsxs("div", { style: {
            padding: '12px 16px',
            background: '#e3f2fd',
            border: '1px solid #1976d2',
            borderRadius: '4px',
            fontSize: '12px',
            textAlign: 'center',
            minWidth: '120px',
            boxShadow: '0 2px 4px rgba(0,0,0,0.1)',
            position: 'relative'
        }, children: [renderHandles(), String(data.label || id)] }));
}
/**
 * Container node component
 */
export function ContainerNode({ id, data }) {
    // Use dimensions from ELK layout via ReactFlowBridge data
    const width = data.width || 180; // fallback to default
    const height = data.height || (data.collapsed ? 60 : 120); // fallback to default
    // Debug: Log container dimensions
    console.log(`[ContainerNode] 📏 Container ${id}: data.width=${data.width}, data.height=${data.height}, using ${width}x${height}`);
    return (_jsxs("div", { style: {
            padding: '16px',
            background: data.collapsed ? '#ffeb3b' : 'rgba(25, 118, 210, 0.1)',
            border: data.collapsed ? '2px solid #f57f17' : '2px solid #1976d2',
            borderRadius: '8px',
            fontSize: '12px',
            textAlign: 'center',
            width: `${width}px`, // Use ELK-calculated width
            height: `${height}px`, // Use ELK-calculated height
            position: 'relative',
            boxSizing: 'border-box' // Ensure padding is included in dimensions
        }, children: [renderHandles(), _jsx("strong", { children: String(data.label || id) }), data.collapsed && (_jsx("div", { style: { fontSize: '10px', color: '#666', marginTop: '4px' }, children: "(collapsed)" }))] }));
}
// Export map for ReactFlow nodeTypes
export const nodeTypes = {
    standard: StandardNode,
    container: ContainerNode
};
//# sourceMappingURL=nodes.js.map