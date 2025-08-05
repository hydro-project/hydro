import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
/**
 * Layout Controls Component
 *
 * Dropdown controls for layout algorithm and color palette
 */
import React from 'react';
import { getLayoutOptions } from '../utils/elkConfig.js';
import styles from '../../../pages/visualizer.module.css';
// SVG Icons for pack/unpack operations
const PackIcon = () => (_jsx("svg", { width: "20", height: "20", viewBox: "0 0 66 66", fill: "currentColor", children: _jsx("path", { d: "M63.6,28.1l-6.5-6.9c0,0-.3-.2-.4-.2l-14.8-5.8h0c.2-.3.3-.7,0-1-.2-.3-.5-.6-.8-.6h-3.6V3.4c0-.6-.4-.9-.9-.9h-7.7c-.6,0-.9.4-.9.9v10.1h-3.6c-.4,0-.7.2-.8.6-.2.3,0,.7,0,.9l-13.9,5.6c0,0-.2,0-.3.2h0l-7.3,7.4c-.2.2-.3.6-.3.8,0,.3.3.6.6.7l6.7,2.7v20.8c0,.4.2.7.6.8l22.5,10.1h0c0,0,.2,0,.4,0h0c0,0,.2,0,.3,0h0l23.8-9.8c.4-.2.6-.5.6-.8v-21.6l5.8-2.3c.3,0,.5-.4.6-.7,0-.4,0-.7-.2-.8ZM29.2,15.4c.6,0,.9-.4.9-.9V4.4h5.8v10.1c0,.6.4.9.9.9h2.5l-6.4,8.2-6.4-8.2h2.5ZM25.1,16.6l7.1,9.2c.2.2.5.4.7.4s.6,0,.7-.4l7.1-9.1,13.1,5.2-21.2,8-20-8.2,12.3-5.1ZM10.5,22.7l20.3,8.3-7.4,5.4c-14.5-6.1-10.6-4.5-18.8-7.8l5.9-6ZM11.3,33.3l12,5.1c.3,0,.7,0,.9,0l7.7-5.6v29.3l-20.6-9.3s0-19.4,0-19.4ZM55.6,52.9l-21.9,9.1v-29.3l6.7,5.5c.3.2.7.3.9.2l14.2-5.7v20.2ZM41.2,36.4l-6.6-5.3,21.7-8.2,5.1,5.4-20.2,8.1Z" }) }));
const UnpackIcon = () => (_jsxs("svg", { width: "20", height: "20", viewBox: "0 0 66.1 65.8", fill: "currentColor", children: [_jsx("path", { d: "M56.6,27.1l5.3-2.6c.3-.2.5-.5.5-.8s-.2-.6-.4-.8l-16.3-8.9c-.4-.3-1,0-1.2.4-.3.5,0,1,.4,1.3l14.8,8-4.6,2.3-11.8-5.9c-.4-.2-1,0-1.2.5s0,1,.4,1.2l10.6,5.2-19.8,9.7-19.6-9.7,11.1-5.4c.4-.2.6-.7.4-1.2s-.7-.6-1.2-.5l-12.5,6.1-4.6-2.3,15.2-8c.4-.3.6-.8.4-1.2-.3-.5-.8-.6-1.2-.4L4.5,22.9c-.3.2-.4.5-.4.8s.2.6.5.8l5.3,2.5-5.8,4.1c-.4.2-.4.5-.4.8s.2.6.4.7l6.6,3.4v17.5c0,.4.3.7.5.8l21.6,8.2c.2,0,.4,0,.6,0l21.6-8.1c.4,0,.6-.5.6-.8v-17.3l6.1-3.3c.3-.2.4-.5.4-.7s0-.6-.4-.8l-5.3-4.4ZM11.6,27.9l20.1,9.9-4.3,4.9L6.2,31.7l5.4-3.8ZM12.4,37l14.9,7.7c0,0,.3,0,.4,0,.3,0,.5,0,.7-.3l3.9-4.5v20.6l-19.9-7.6v-16ZM54,53.1l-19.8,7.4v-20.9l4.9,4.9c.2.2.4.3.6.3s.3,0,.4,0l13.9-7.4v15.8ZM39.8,42.8l-5-5.1,20-9.8,5.1,4.1-20.1,10.7Z" }), _jsx("path", { d: "M22,18.5c.2.3.4.5.8.5h4.7v12.8c0,.3,0,.5.3.6s.4.3.6.3h9.6c.5,0,.9-.4.9-.9v-12.8h4.8c.4,0,.6-.2.8-.5.2-.3,0-.6,0-.9l-10.5-14.1c-.4-.5-1.1-.5-1.4,0l-10.4,14.1c-.2.3-.3.6,0,.9ZM33.2,5.5l8.7,11.7h-3.8c-.5,0-.9.4-.9.9v12.8h-7.8v-12.8c0-.3,0-.5-.3-.6-.2-.2-.4-.3-.6-.3h-3.8l8.5-11.7Z" })] }));
// Get layout options from centralized ELK config
const layoutOptions = getLayoutOptions();
const paletteOptions = {
    Set3: 'Set3',
    Pastel1: 'Pastel1',
    Dark2: 'Dark2'
};
export function LayoutControls({ currentLayout, onLayoutChange, colorPalette, onPaletteChange, hasCollapsedContainers, onCollapseAll, onExpandAll, autoFit, onAutoFitToggle, onFitView }) {
    return (_jsxs("div", { className: styles.layoutControls, children: [_jsx("select", { className: styles.layoutSelect, value: currentLayout, onChange: (e) => onLayoutChange(e.target.value), children: Object.entries(layoutOptions).map(([key, label]) => (_jsx("option", { value: key, children: label }, key))) }), _jsx("select", { className: styles.paletteSelect, value: colorPalette, onChange: (e) => onPaletteChange(e.target.value), children: Object.entries(paletteOptions).map(([key, label]) => (_jsx("option", { value: key, children: label }, key))) }), _jsx("button", { className: styles.containerButton, onClick: hasCollapsedContainers ? onExpandAll : onCollapseAll, title: hasCollapsedContainers ? 'Unpack All Containers' : 'Pack All Containers', children: hasCollapsedContainers ? _jsx(UnpackIcon, {}) : _jsx(PackIcon, {}) }), _jsx("div", { className: styles.separator }), _jsx("button", { className: styles.containerButton, onClick: onFitView, title: "Fit graph to viewport", children: "\u26F6" }), _jsx("label", { className: styles.autoFitLabel, children: _jsx("input", { type: "checkbox", checked: autoFit, onChange: (e) => onAutoFitToggle(e.target.checked), className: styles.autoFitCheckbox, title: "Auto Fit" }) }), _jsx("div", { className: styles.separator })] }));
}
//# sourceMappingURL=LayoutControls.js.map