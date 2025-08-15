/**
 * @fileoverview Visual Configuration Panel Tests
 * 
 * Basic tests to verify the Visual Configuration Panel functionality.
 */

import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { VisualConfigPanel } from '../VisualConfigPanel';
import type { VisualConfigState } from '../types';

// Mock the imports since we're testing in isolation
vi.mock('../DockablePanel', () => ({
  DockablePanel: ({ children, title }: { children: React.ReactNode; title: string }) => (
    <div data-testid="dockable-panel">
      <div data-testid="panel-title">{title}</div>
      <div data-testid="panel-content">{children}</div>
    </div>
  )
}));

vi.mock('../CollapsibleSection', () => ({
  CollapsibleSection: ({ children, title, isCollapsed, onToggle }: {
    children: React.ReactNode;
    title: string;
    isCollapsed: boolean;
    onToggle: () => void;
  }) => (
    <div data-testid={`section-${title.toLowerCase().replace(/\s+/g, '-')}`}>
      <button onClick={onToggle} data-testid={`toggle-${title.toLowerCase().replace(/\s+/g, '-')}`}>
        {title} {isCollapsed ? '▼' : '▲'}
      </button>
      {!isCollapsed && <div data-testid="section-content">{children}</div>}
    </div>
  )
}));

vi.mock('../controls', () => ({
  DropdownControl: ({ label, value, onChange, options }: {
    label: string;
    value: string;
    onChange: (value: string) => void;
    options: Array<{ value: string; label: string }>;
  }) => (
    <div data-testid={`dropdown-${label.toLowerCase().replace(/\s+/g, '-')}`}>
      <label>{label}</label>
      <select value={value} onChange={(e) => onChange(e.target.value)} data-testid={`select-${label.toLowerCase().replace(/\s+/g, '-')}`}>
        {options.map(opt => (
          <option key={opt.value} value={opt.value}>{opt.label}</option>
        ))}
      </select>
    </div>
  ),
  SliderControl: ({ label, value, onChange, min, max }: {
    label: string;
    value: number;
    onChange: (value: number) => void;
    min: number;
    max: number;
  }) => (
    <div data-testid={`slider-${label.toLowerCase().replace(/\s+/g, '-')}`}>
      <label>{label}: {value}</label>
      <input 
        type="range" 
        min={min} 
        max={max} 
        value={value} 
        onChange={(e) => onChange(parseFloat(e.target.value))}
        data-testid={`range-${label.toLowerCase().replace(/\s+/g, '-')}`}
      />
    </div>
  ),
  RadioGroupControl: ({ label, value, onChange, options }: {
    label: string;
    value: string;
    onChange: (value: string) => void;
    options: Array<{ value: string; label: string }>;
  }) => (
    <div data-testid={`radio-${label.toLowerCase().replace(/\s+/g, '-')}`}>
      <fieldset>
        <legend>{label}</legend>
        {options.map(opt => (
          <label key={opt.value}>
            <input 
              type="radio" 
              name={label} 
              value={opt.value} 
              checked={value === opt.value}
              onChange={(e) => onChange(e.target.value)}
              data-testid={`radio-${opt.value}`}
            />
            {opt.label}
          </label>
        ))}
      </fieldset>
    </div>
  )
}));

describe('VisualConfigPanel', () => {
  it('should render with default configuration', () => {
    render(<VisualConfigPanel />);
    
    expect(screen.getByTestId('dockable-panel')).toBeInTheDocument();
    expect(screen.getByTestId('panel-title')).toHaveTextContent('Visual Configuration');
  });

  it('should render all configuration sections', () => {
    render(<VisualConfigPanel />);
    
    // Check that all main sections are present
    expect(screen.getByTestId('section-visual-controls')).toBeInTheDocument();
    expect(screen.getByTestId('section-color-&-typography')).toBeInTheDocument();
    expect(screen.getByTestId('section-visual-styling')).toBeInTheDocument();
    expect(screen.getByTestId('section-container-sizing')).toBeInTheDocument();
  });

  it('should call onConfigChange when controls are modified', () => {
    const mockOnConfigChange = vi.fn();
    render(<VisualConfigPanel onConfigChange={mockOnConfigChange} />);
    
    // Change node style dropdown
    const nodeStyleSelect = screen.getByTestId('select-node-style');
    fireEvent.change(nodeStyleSelect, { target: { value: 'highlighted' } });
    
    expect(mockOnConfigChange).toHaveBeenCalledWith(
      expect.objectContaining({
        nodeStyle: 'highlighted'
      })
    );
  });

  it('should update slider values correctly', () => {
    const mockOnConfigChange = vi.fn();
    render(<VisualConfigPanel onConfigChange={mockOnConfigChange} />);
    
    // Expand Typography section first
    const typographyToggle = screen.getByTestId('toggle-color-&-typography');
    fireEvent.click(typographyToggle);
    
    // Change typography scale slider
    const typographySlider = screen.getByTestId('range-typography-scale');
    fireEvent.change(typographySlider, { target: { value: '1.2' } });
    
    expect(mockOnConfigChange).toHaveBeenCalledWith(
      expect.objectContaining({
        typographyScale: 1.2
      })
    );
  });

  it('should handle radio button selection for color palette', () => {
    const mockOnConfigChange = vi.fn();
    render(<VisualConfigPanel onConfigChange={mockOnConfigChange} />);
    
    // Expand Color & Typography section
    const colorToggle = screen.getByTestId('toggle-color-&-typography');
    fireEvent.click(colorToggle);
    
    // Select different color palette
    const paletteRadio = screen.getByTestId('radio-Pastel1');
    fireEvent.click(paletteRadio);
    
    expect(mockOnConfigChange).toHaveBeenCalledWith(
      expect.objectContaining({
        colorPalette: 'Pastel1'
      })
    );
  });

  it('should accept custom default configuration', () => {
    const customConfig = {
      nodeStyle: 'highlighted',
      typographyScale: 1.3,
      shadowIntensity: 'LARGE'
    };
    
    const mockOnConfigChange = vi.fn();
    render(
      <VisualConfigPanel 
        defaultConfig={customConfig}
        onConfigChange={mockOnConfigChange}
      />
    );
    
    // The component should start with custom values
    // We can verify this by triggering a change and seeing the base state
    const nodeStyleSelect = screen.getByTestId('select-node-style');
    expect(nodeStyleSelect).toHaveValue('highlighted');
  });

  it('should toggle section visibility correctly', () => {
    render(<VisualConfigPanel />);
    
    // Visual Controls section should be expanded by default
    expect(screen.getByTestId('section-visual-controls')).toBeInTheDocument();
    
    // Container Sizing should be collapsed by default
    const containerToggle = screen.getByTestId('toggle-container-sizing');
    fireEvent.click(containerToggle);
    
    // Should now show container sizing controls
    expect(screen.getByTestId('section-content')).toBeInTheDocument();
  });
});