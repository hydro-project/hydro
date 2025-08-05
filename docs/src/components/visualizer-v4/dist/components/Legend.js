import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { generateNodeColors } from '../shared/colorUtils';
import { COLOR_PALETTES, COMPONENT_COLORS } from '../shared/config';
export function Legend({ legendData, colorPalette = 'Set3', nodeTypeConfig, title, compact = false, className = '', style }) {
    const displayTitle = title || legendData.title || 'Legend';
    const paletteKey = (colorPalette in COLOR_PALETTES) ? colorPalette : 'Set3';
    const legendStyle = {
        fontSize: compact ? '9px' : '10px',
        ...style
    };
    const itemStyle = {
        display: 'flex',
        alignItems: 'center',
        margin: compact ? '2px 0' : '3px 0',
        fontSize: compact ? '9px' : '10px'
    };
    const colorBoxStyle = (colors) => ({
        width: compact ? '10px' : '12px',
        height: compact ? '10px' : '12px',
        borderRadius: '2px',
        marginRight: compact ? '4px' : '6px',
        border: `1px solid ${COMPONENT_COLORS.BORDER_MEDIUM}`,
        flexShrink: 0,
        backgroundColor: colors.primary,
        borderColor: colors.border
    });
    return (_jsxs("div", { className: `legend ${className}`, style: legendStyle, children: [!compact && displayTitle && (_jsx("div", { style: {
                    fontWeight: 'bold',
                    marginBottom: '6px',
                    color: COMPONENT_COLORS.TEXT_PRIMARY,
                    fontSize: '11px'
                }, children: displayTitle })), legendData.items.map(item => {
                const colors = generateNodeColors([item.type], paletteKey, nodeTypeConfig);
                return (_jsxs("div", { style: itemStyle, title: item.description || `${item.label} nodes`, children: [_jsx("div", { style: colorBoxStyle(colors) }), _jsx("span", { style: { color: COMPONENT_COLORS.TEXT_PRIMARY }, children: item.label })] }, item.type));
            })] }));
}
//# sourceMappingURL=Legend.js.map