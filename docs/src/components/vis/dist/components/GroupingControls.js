import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { COMPONENT_COLORS } from '../shared/config';
export function GroupingControls({ hierarchyChoices = [], currentGrouping, onGroupingChange, compact = false, disabled = false, className = '', style }) {
    if (!hierarchyChoices || hierarchyChoices.length === 0) {
        return (_jsx("div", { className: `grouping-controls-empty ${className}`, style: style, children: _jsx("span", { style: {
                    color: COMPONENT_COLORS.TEXT_DISABLED,
                    fontSize: compact ? '9px' : '10px',
                    fontStyle: 'italic'
                }, children: "No grouping options available" }) }));
    }
    if (hierarchyChoices.length === 1) {
        return (_jsx("div", { className: `grouping-controls-single ${className}`, style: style, children: _jsxs("span", { style: {
                    color: COMPONENT_COLORS.TEXT_PRIMARY,
                    fontSize: compact ? '9px' : '10px',
                    fontWeight: 'bold'
                }, children: ["Grouping: ", hierarchyChoices[0].name] }) }));
    }
    const handleChange = (event) => {
        if (!disabled && onGroupingChange) {
            onGroupingChange(event.target.value);
        }
    };
    const selectStyle = {
        fontSize: compact ? '9px' : '10px',
        padding: compact ? '2px 4px' : '4px 6px',
        border: `1px solid ${COMPONENT_COLORS.BORDER_MEDIUM}`,
        borderRadius: '3px',
        backgroundColor: disabled ? COMPONENT_COLORS.BACKGROUND_SECONDARY : COMPONENT_COLORS.BACKGROUND_PRIMARY,
        color: disabled ? COMPONENT_COLORS.TEXT_DISABLED : COMPONENT_COLORS.TEXT_PRIMARY,
        cursor: disabled ? 'not-allowed' : 'pointer',
        width: '100%',
        maxWidth: compact ? '120px' : '180px',
    };
    const labelStyle = {
        fontSize: compact ? '9px' : '10px',
        fontWeight: 'bold',
        color: COMPONENT_COLORS.TEXT_PRIMARY,
        marginBottom: '4px',
        display: 'block',
    };
    return (_jsxs("div", { className: `grouping-controls ${className}`, style: style, children: [!compact && (_jsx("label", { style: labelStyle, children: "Grouping:" })), _jsxs("select", { value: currentGrouping || '', onChange: handleChange, disabled: disabled, style: selectStyle, title: disabled ? 'Grouping controls are disabled' : 'Select a grouping method', children: [!currentGrouping && (_jsx("option", { value: "", disabled: true, children: "Select grouping..." })), hierarchyChoices.map(choice => (_jsx("option", { value: choice.id, children: choice.name }, choice.id)))] })] }));
}
//# sourceMappingURL=GroupingControls.js.map