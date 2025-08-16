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
    color: '#374151',
    display: 'flex',
    alignItems: 'center',
    gap: '4px'
  };

  const sectionTitleStyle: React.CSSProperties = {
    fontSize: '13px',
    fontWeight: '600',
    color: '#1f2937',
    margin: '12px 0 8px 0',
    padding: '0 0 4px 0',
    borderBottom: '1px solid #e5e7eb'
  };

  const valueDisplayStyle: React.CSSProperties = {
    fontSize: '10px',
    color: '#6b7280',
    marginLeft: '4px'
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
        <div style={sectionTitleStyle}>Edge Appearance</div>
        
        <div style={rowStyle}>
          <label style={labelStyle} title="Choose how edges are drawn between nodes">
            Edge Style
          </label>
          <select
            value={local.edgeStyle || 'bezier'}
            style={inputStyle}
            onChange={(e) => update({ edgeStyle: e.target.value as EdgeStyleKind })}
            title="Bezier: curved edges, Straight: direct lines, SmoothStep: stepped curves"
          >
            <option value="bezier">Bezier</option>
            <option value="straight">Straight</option>
            <option value="smoothstep">SmoothStep</option>
          </select>
        </div>

        <div style={rowStyle}>
          <label style={labelStyle} title="Color for edges and arrowheads">
            Edge Color
          </label>
          <input
            type="color"
            value={local.edgeColor || '#1976d2'}
            style={{ ...inputStyle, padding: 0, height: 28 }}
            onChange={(e) => update({ edgeColor: e.target.value })}
          />
        </div>

        <div style={rowStyle}>
          <label style={labelStyle} title="Thickness of edge lines">
            Edge Width
            <span style={valueDisplayStyle}>{local.edgeWidth ?? 2}px</span>
          </label>
          <input
            type="range" min={1} max={8}
            value={local.edgeWidth ?? 2}
            onChange={(e) => update({ edgeWidth: parseInt(e.target.value, 10) })}
          />
        </div>

        <div style={rowStyle}>
          <label style={labelStyle} title="Draw edges with dashed lines">
            Edge Dashed
          </label>
          <input
            type="checkbox"
            checked={!!local.edgeDashed}
            onChange={(e) => update({ edgeDashed: e.target.checked })}
          />
        </div>

        <div style={sectionTitleStyle}>Node Appearance</div>

        <div style={rowStyle}>
          <label style={labelStyle} title="Roundness of node corners">
            Border Radius
            <span style={valueDisplayStyle}>{local.nodeBorderRadius ?? 4}px</span>
          </label>
          <input
            type="range" min={0} max={24}
            value={local.nodeBorderRadius ?? 4}
            onChange={(e) => update({ nodeBorderRadius: parseInt(e.target.value, 10) })}
          />
        </div>

        <div style={rowStyle}>
          <label style={labelStyle} title="Inner spacing within nodes">
            Padding
            <span style={valueDisplayStyle}>{local.nodePadding ?? 12}px</span>
          </label>
          <input
            type="range" min={4} max={32}
            value={local.nodePadding ?? 12}
            onChange={(e) => update({ nodePadding: parseInt(e.target.value, 10) })}
          />
        </div>

        <div style={rowStyle}>
          <label style={labelStyle} title="Size of text within nodes">
            Font Size
            <span style={valueDisplayStyle}>{local.nodeFontSize ?? 12}px</span>
          </label>
          <input
            type="range" min={10} max={20}
            value={local.nodeFontSize ?? 12}
            onChange={(e) => update({ nodeFontSize: parseInt(e.target.value, 10) })}
          />
        </div>

        <div style={sectionTitleStyle}>Container Appearance</div>

        <div style={rowStyle}>
          <label style={labelStyle} title="Roundness of container corners">
            Border Radius
            <span style={valueDisplayStyle}>{local.containerBorderRadius ?? 8}px</span>
          </label>
          <input
            type="range" min={0} max={24}
            value={local.containerBorderRadius ?? 8}
            onChange={(e) => update({ containerBorderRadius: parseInt(e.target.value, 10) })}
          />
        </div>

        <div style={rowStyle}>
          <label style={labelStyle} title="Thickness of container borders">
            Border Width
            <span style={valueDisplayStyle}>{local.containerBorderWidth ?? 2}px</span>
          </label>
          <input
            type="range" min={1} max={6}
            value={local.containerBorderWidth ?? 2}
            onChange={(e) => update({ containerBorderWidth: parseInt(e.target.value, 10) })}
          />
        </div>

        <div style={rowStyle}>
          <label style={labelStyle} title="Drop shadow intensity">
            Shadow
          </label>
          <select
            value={local.containerShadow || 'LIGHT'}
            style={inputStyle}
            onChange={(e) => update({ containerShadow: e.target.value as any })}
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
