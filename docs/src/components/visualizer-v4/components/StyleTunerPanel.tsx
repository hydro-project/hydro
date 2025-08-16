import React, { useState, useEffect } from 'react';
import { DockablePanel, PANEL_POSITIONS } from './DockablePanel';

type EdgeStyleKind = 'bezier' | 'straight' | 'smoothstep';

export interface StyleTunerPanelProps {
  // Feed and control the FlowGraph RenderConfig style fields
  value: {
    edgeStyle?: EdgeStyleKind;
    edgeColor?: string;
    edgeWidth?: number;
    edgeDashed?: boolean;
    nodeBorderRadius?: number;
    nodePadding?: number;
    nodeFontSize?: number;
    containerBorderRadius?: number;
    containerBorderWidth?: number;
    containerShadow?: 'LIGHT' | 'MEDIUM' | 'LARGE' | 'NONE';
  };
  onChange: (next: StyleTunerPanelProps['value']) => void;
  defaultCollapsed?: boolean;
}

export function StyleTunerPanel({ value, onChange, defaultCollapsed = false }: StyleTunerPanelProps) {
  const [local, setLocal] = useState(value);

  useEffect(() => setLocal(value), [value]);

  const update = (patch: Partial<typeof local>) => {
    const next = { ...local, ...patch };
    setLocal(next);
    onChange(next);
  };

  const inputStyle: React.CSSProperties = {
    padding: '4px 8px',
    border: '1px solid #ced4da',
    borderRadius: '4px',
    backgroundColor: '#fff',
    fontSize: '12px',
    width: '100%'
  };

  const rowStyle: React.CSSProperties = {
    display: 'grid',
    gridTemplateColumns: '1fr 120px',
    alignItems: 'center',
    gap: '8px',
    marginBottom: '8px'
  };

  const labelStyle: React.CSSProperties = {
    fontSize: '12px',
    fontWeight: '500',
    color: '#444'
  };

  return (
    <DockablePanel
      id="style-tuner"
      title="Style Tuner"
      defaultPosition={PANEL_POSITIONS.TOP_RIGHT}
      defaultDocked={true}
      defaultCollapsed={defaultCollapsed}
      minWidth={280}
      minHeight={200}
    >
      <div style={{ fontSize: '12px' }}>
        <div style={rowStyle}>
          <label style={labelStyle} title="Choose how edges are drawn between nodes">Edge Style</label>
          <select
            value={local.edgeStyle || 'bezier'}
            style={inputStyle}
            onChange={(e) => update({ edgeStyle: e.target.value as EdgeStyleKind })}
            title="Select edge routing style"
          >
            <option value="bezier">Bezier</option>
            <option value="straight">Straight</option>
            <option value="smoothstep">SmoothStep</option>
          </select>
        </div>

        <div style={rowStyle}>
          <label style={labelStyle} title="Color of edges and arrowheads">Edge Color</label>
          <input
            type="color"
            value={local.edgeColor || '#1976d2'}
            style={{ ...inputStyle, padding: 0, height: 28 }}
            onChange={(e) => update({ edgeColor: e.target.value })}
            title="Pick edge and arrowhead color"
          />
        </div>

        <div style={rowStyle}>
          <label style={labelStyle} title="Thickness of edge lines">Edge Width</label>
          <input
            type="range" min={1} max={8}
            value={local.edgeWidth ?? 2}
            onChange={(e) => update({ edgeWidth: parseInt(e.target.value, 10) })}
            title={`Edge width: ${local.edgeWidth ?? 2}px`}
          />
        </div>

        <div style={rowStyle}>
          <label style={labelStyle} title="Draw edges with dashed lines">Edge Dashed</label>
          <input
            type="checkbox"
            checked={!!local.edgeDashed}
            onChange={(e) => update({ edgeDashed: e.target.checked })}
            title="Toggle dashed edge style"
          />
        </div>

        <hr />

        <div style={rowStyle}>
          <label style={labelStyle} title="Roundness of node corners">Node Border Radius</label>
          <input
            type="range" min={0} max={24}
            value={local.nodeBorderRadius ?? 4}
            onChange={(e) => update({ nodeBorderRadius: parseInt(e.target.value, 10) })}
            title={`Border radius: ${local.nodeBorderRadius ?? 4}px`}
          />
        </div>

        <div style={rowStyle}>
          <label style={labelStyle} title="Internal spacing within nodes">Node Padding</label>
          <input
            type="range" min={4} max={32}
            value={local.nodePadding ?? 12}
            onChange={(e) => update({ nodePadding: parseInt(e.target.value, 10) })}
            title={`Padding: ${local.nodePadding ?? 12}px`}
          />
        </div>

        <div style={rowStyle}>
          <label style={labelStyle} title="Size of text in nodes">Node Font Size</label>
          <input
            type="range" min={10} max={20}
            value={local.nodeFontSize ?? 12}
            onChange={(e) => update({ nodeFontSize: parseInt(e.target.value, 10) })}
            title={`Font size: ${local.nodeFontSize ?? 12}px`}
          />
        </div>

        <hr />

        <div style={rowStyle}>
          <label style={labelStyle} title="Roundness of container corners">Container Border Radius</label>
          <input
            type="range" min={0} max={24}
            value={local.containerBorderRadius ?? 8}
            onChange={(e) => update({ containerBorderRadius: parseInt(e.target.value, 10) })}
            title={`Border radius: ${local.containerBorderRadius ?? 8}px`}
          />
        </div>

        <div style={rowStyle}>
          <label style={labelStyle} title="Thickness of container borders">Container Border Width</label>
          <input
            type="range" min={1} max={6}
            value={local.containerBorderWidth ?? 2}
            onChange={(e) => update({ containerBorderWidth: parseInt(e.target.value, 10) })}
            title={`Border width: ${local.containerBorderWidth ?? 2}px`}
          />
        </div>

        <div style={rowStyle}>
          <label style={labelStyle} title="Drop shadow effect for containers">Container Shadow</label>
          <select
            value={local.containerShadow || 'LIGHT'}
            style={inputStyle}
            onChange={(e) => update({ containerShadow: e.target.value as any })}
            title="Choose shadow intensity"
          >
            <option value="NONE">None</option>
            <option value="LIGHT">Light</option>
            <option value="MEDIUM">Medium</option>
            <option value="LARGE">Large</option>
          </select>
        </div>
      </div>
    </DockablePanel>
  );
}

export default StyleTunerPanel;
