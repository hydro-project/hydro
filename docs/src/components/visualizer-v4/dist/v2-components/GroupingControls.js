import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
/**
 * Grouping Controls Component
 *
 * Dropdown control for selecting grouping hierarchy (Location, Backtrace, etc.)
 */
import React from 'react';
import styles from '../../../pages/visualizer.module.css';
export function GroupingControls({ hierarchyChoices, currentGrouping, onGroupingChange, compact = false // New prop to enable compact styling for Info Panel
 }) {
    // If there's only one choice or no choices, show disabled dropdown
    const isDisabled = !hierarchyChoices || hierarchyChoices.length <= 1;
    const containerClass = compact ? styles.infoPanelGroupingControls : styles.groupingControls;
    return (_jsxs("div", { className: containerClass, children: [_jsx("label", { className: styles.controlLabel, children: "Group by:" }), _jsx("select", { className: `${styles.groupingSelect} ${isDisabled ? styles.disabled : ''}`, value: currentGrouping || '', onChange: (e) => onGroupingChange(e.target.value), disabled: isDisabled, children: hierarchyChoices && hierarchyChoices.length > 0 ? (hierarchyChoices.map((choice) => (_jsx("option", { value: choice.id, children: choice.name }, choice.id)))) : (_jsx("option", { value: "", children: "No grouping available" })) })] }));
}
//# sourceMappingURL=GroupingControls.js.map