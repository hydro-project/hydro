import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { COMPONENT_COLORS } from '../shared/config.js';
export function CollapsibleSection({ title, isCollapsed, onToggle, children, level = 0, showIcon = true, disabled = false, className = '', style }) {
    const handleClick = () => {
        if (!disabled) {
            onToggle();
        }
    };
    const sectionStyle = {
        marginBottom: '12px',
        ...style
    };
    const headerStyle = {
        display: 'flex',
        alignItems: 'center',
        cursor: disabled ? 'default' : 'pointer',
        fontSize: '11px',
        fontWeight: 'bold',
        marginBottom: isCollapsed ? '0' : '6px',
        color: disabled ? COMPONENT_COLORS.TEXT_DISABLED : COMPONENT_COLORS.TEXT_PRIMARY,
        paddingLeft: `${level * 8}px`,
        padding: '4px 0',
        borderRadius: '2px',
        transition: 'background-color 0.15s ease',
    };
    const contentStyle = {
        paddingLeft: '12px',
        paddingTop: '4px',
    };
    return (_jsxs("div", { className: `collapsible-section ${className}`, style: sectionStyle, children: [_jsxs("div", { style: headerStyle, onClick: handleClick, onMouseEnter: (e) => {
                    if (!disabled) {
                        e.currentTarget.style.backgroundColor = COMPONENT_COLORS.BUTTON_HOVER_BACKGROUND;
                    }
                }, onMouseLeave: (e) => {
                    e.currentTarget.style.backgroundColor = 'transparent';
                }, title: disabled ? undefined : `${isCollapsed ? 'Expand' : 'Collapse'} ${title}`, children: [showIcon && (_jsx("span", { style: {
                            marginRight: '6px',
                            fontSize: '10px',
                            color: disabled ? COMPONENT_COLORS.TEXT_DISABLED : COMPONENT_COLORS.TEXT_SECONDARY,
                            transition: 'transform 0.15s ease',
                            transform: isCollapsed ? 'rotate(0deg)' : 'rotate(90deg)'
                        }, children: "\u25B6" })), _jsx("span", { children: title })] }), !isCollapsed && (_jsx("div", { style: contentStyle, children: children }))] }));
}
//# sourceMappingURL=CollapsibleSection.js.map