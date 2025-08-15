/**
 * @fileoverview Visual Configuration Panel
 * 
 * Interactive dockable panel for controlling visual constants from shared/config.ts.
 * Provides real-time visual controls for node styles, colors, typography, and layout.
 */

import React, { useState, useCallback } from 'react';
import { DockablePanel } from './DockablePanel';
import { CollapsibleSection } from './CollapsibleSection';
import { DropdownControl, SliderControl, RadioGroupControl } from './controls';
import { PANEL_POSITIONS } from './types';
import { 
  NODE_STYLES, 
  EDGE_STYLES, 
  CONTAINER_STYLES, 
  COLOR_PALETTES,
  SHADOWS,
  SIZES,
  TYPOGRAPHY,
  COMPONENT_COLORS
} from '../shared/config';

// ReactFlow edge types (commonly used types)
const REACTFLOW_EDGE_TYPES = {
  DEFAULT: 'default',
  STRAIGHT: 'straight', 
  STEP: 'step',
  SMOOTHSTEP: 'smoothstep',
  BEZIER: 'bezier'
} as const;

export interface VisualConfigState {
  // Style selections
  nodeStyle: string;
  edgeStyle: string;
  containerStyle: string;
  edgeType: string;
  
  // Color and palette
  colorPalette: string;
  
  // Typography
  typographyScale: number;
  infoPanelFontSize: number;
  
  // Visual styling
  shadowIntensity: string;
  borderRadius: number;
  
  // Container sizing
  collapsedContainerWidth: number;
  collapsedContainerHeight: number;
}

export interface VisualConfigPanelProps {
  id?: string;
  title?: string;
  defaultConfig?: Partial<VisualConfigState>;
  onConfigChange?: (config: VisualConfigState) => void;
  onPositionChange?: (panelId: string, position: string) => void;
  className?: string;
  style?: React.CSSProperties;
}

const DEFAULT_CONFIG: VisualConfigState = {
  nodeStyle: NODE_STYLES.DEFAULT,
  edgeStyle: EDGE_STYLES.DEFAULT,
  containerStyle: CONTAINER_STYLES.DEFAULT,
  edgeType: REACTFLOW_EDGE_TYPES.DEFAULT,
  colorPalette: 'Set3',
  typographyScale: 1.0,
  infoPanelFontSize: 14,
  shadowIntensity: 'MEDIUM',
  borderRadius: 6,
  collapsedContainerWidth: SIZES.COLLAPSED_CONTAINER_WIDTH,
  collapsedContainerHeight: SIZES.COLLAPSED_CONTAINER_HEIGHT
};

export function VisualConfigPanel({
  id = 'visual-config-panel',
  title = 'Visual Configuration',
  defaultConfig = {},
  onConfigChange,
  onPositionChange,
  className = '',
  style = {}
}: VisualConfigPanelProps) {
  const [config, setConfig] = useState<VisualConfigState>({
    ...DEFAULT_CONFIG,
    ...defaultConfig
  });
  
  // Section collapse states
  const [stylesCollapsed, setStylesCollapsed] = useState(false);
  const [colorsCollapsed, setColorsCollapsed] = useState(false);
  const [typographyCollapsed, setTypographyCollapsed] = useState(false);
  const [visualStylingCollapsed, setVisualStylingCollapsed] = useState(false);
  const [containerSizingCollapsed, setContainerSizingCollapsed] = useState(true);

  // Update configuration and notify parent
  const updateConfig = useCallback((updates: Partial<VisualConfigState>) => {
    const newConfig = { ...config, ...updates };
    setConfig(newConfig);
    
    if (onConfigChange) {
      onConfigChange(newConfig);
    }
  }, [config, onConfigChange]);

  // Prepare dropdown options
  const nodeStyleOptions = Object.values(NODE_STYLES).map(style => ({
    value: style,
    label: style.charAt(0).toUpperCase() + style.slice(1),
    description: `Node style: ${style}`
  }));

  const edgeStyleOptions = Object.values(EDGE_STYLES).map(style => ({
    value: style,
    label: style.charAt(0).toUpperCase() + style.slice(1),
    description: `Edge style: ${style}`
  }));

  const containerStyleOptions = Object.values(CONTAINER_STYLES).map(style => ({
    value: style,
    label: style.charAt(0).toUpperCase() + style.slice(1),
    description: `Container style: ${style}`
  }));

  const edgeTypeOptions = Object.values(REACTFLOW_EDGE_TYPES).map(type => ({
    value: type,
    label: type.charAt(0).toUpperCase() + type.slice(1),
    description: `ReactFlow edge type: ${type}`
  }));

  const colorPaletteOptions = Object.keys(COLOR_PALETTES).map(palette => ({
    value: palette,
    label: palette,
    description: `Color palette: ${palette}`
  }));

  const shadowIntensityOptions = Object.keys(SHADOWS).filter(key => 
    ['LIGHT', 'MEDIUM', 'LARGE'].includes(key)
  ).map(intensity => ({
    value: intensity,
    label: intensity.charAt(0).toUpperCase() + intensity.slice(1).toLowerCase(),
    description: `Shadow intensity: ${intensity.toLowerCase()}`
  }));

  return (
    <DockablePanel
      id={id}
      title={title}
      defaultPosition={PANEL_POSITIONS.TOP_LEFT}
      defaultDocked={true}
      defaultCollapsed={false}
      onPositionChange={onPositionChange}
      minWidth={280}
      minHeight={200}
      maxWidth={350}
      maxHeight={700}
      className={className}
      style={style}
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
        {/* Visual Controls Section */}
        <CollapsibleSection
          title="Visual Controls"
          isCollapsed={stylesCollapsed}
          onToggle={() => setStylesCollapsed(!stylesCollapsed)}
        >
          <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
            <DropdownControl
              label="Node Style"
              value={config.nodeStyle}
              options={nodeStyleOptions}
              onChange={(value) => updateConfig({ nodeStyle: value })}
            />
            
            <DropdownControl
              label="Edge Style"
              value={config.edgeStyle}
              options={edgeStyleOptions}
              onChange={(value) => updateConfig({ edgeStyle: value })}
            />
            
            <DropdownControl
              label="Container Style"
              value={config.containerStyle}
              options={containerStyleOptions}
              onChange={(value) => updateConfig({ containerStyle: value })}
            />
            
            <DropdownControl
              label="Edge Type"
              value={config.edgeType}
              options={edgeTypeOptions}
              onChange={(value) => updateConfig({ edgeType: value })}
            />
          </div>
        </CollapsibleSection>

        {/* Color & Typography Section */}
        <CollapsibleSection
          title="Color & Typography"
          isCollapsed={colorsCollapsed}
          onToggle={() => setColorsCollapsed(!colorsCollapsed)}
        >
          <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
            <RadioGroupControl
              label="Color Palette"
              value={config.colorPalette}
              options={colorPaletteOptions}
              onChange={(value) => updateConfig({ colorPalette: value })}
            />
            
            <SliderControl
              label="Typography Scale"
              value={config.typographyScale}
              min={0.8}
              max={1.5}
              step={0.1}
              onChange={(value) => updateConfig({ typographyScale: value })}
              formatValue={(val) => `${val.toFixed(1)}x`}
            />
            
            <SliderControl
              label="InfoPanel Font Size"
              value={config.infoPanelFontSize}
              min={10}
              max={18}
              step={1}
              onChange={(value) => updateConfig({ infoPanelFontSize: value })}
              formatValue={(val) => `${val}px`}
            />
          </div>
        </CollapsibleSection>

        {/* Visual Styling Section */}
        <CollapsibleSection
          title="Visual Styling"
          isCollapsed={visualStylingCollapsed}
          onToggle={() => setVisualStylingCollapsed(!visualStylingCollapsed)}
        >
          <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
            <DropdownControl
              label="Shadow Intensity"
              value={config.shadowIntensity}
              options={shadowIntensityOptions}
              onChange={(value) => updateConfig({ shadowIntensity: value })}
            />
            
            <SliderControl
              label="Border Radius"
              value={config.borderRadius}
              min={0}
              max={20}
              step={1}
              onChange={(value) => updateConfig({ borderRadius: value })}
              formatValue={(val) => `${val}px`}
            />
          </div>
        </CollapsibleSection>

        {/* Container Sizing Section */}
        <CollapsibleSection
          title="Container Sizing"
          isCollapsed={containerSizingCollapsed}
          onToggle={() => setContainerSizingCollapsed(!containerSizingCollapsed)}
        >
          <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
            <SliderControl
              label="Collapsed Width"
              value={config.collapsedContainerWidth}
              min={150}
              max={300}
              step={10}
              onChange={(value) => updateConfig({ collapsedContainerWidth: value })}
              formatValue={(val) => `${val}px`}
            />
            
            <SliderControl
              label="Collapsed Height"
              value={config.collapsedContainerHeight}
              min={100}
              max={200}
              step={10}
              onChange={(value) => updateConfig({ collapsedContainerHeight: value })}
              formatValue={(val) => `${val}px`}
            />
          </div>
        </CollapsibleSection>
      </div>
    </DockablePanel>
  );
}