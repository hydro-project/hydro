import React from 'react';
import { describe, it, expect } from 'vitest';
import { renderToString } from 'react-dom/server';
import { LayoutControls } from '../components/LayoutControls';
import { createVisualizationState } from '../core/VisualizationState';

// SSR-only smoke tests to validate rendering and basic prop wiring without a DOM
describe('LayoutControls (SSR smoke)', () => {
  it('renders with empty state and shows layout options + auto-fit label', () => {
    const visState = createVisualizationState();

    const html = renderToString(
      <LayoutControls
        visualizationState={visState}
        currentLayout="layered"
        autoFit={false}
      />
    );

    // Basic content checks
    expect(html).toContain('Layered');
    expect(html).toContain('Force-Directed');
    expect(html).toContain('Auto Fit');

    // With no containers, both buttons should be disabled (SSR includes disabled attribute)
    const disabledCount = (html.match(/<button[^>]*disabled/gi) || []).length;
    expect(disabledCount).toBe(2);
  });

  it('enables collapse/expand when containers exist in mixed states', () => {
    const visState = createVisualizationState();
    // One expanded, one collapsed
    visState.addContainer('c1', { label: 'C1', collapsed: false });
    visState.addContainer('c2', { label: 'C2', collapsed: true });

    const html = renderToString(
      <LayoutControls
        visualizationState={visState}
        currentLayout="layered"
        autoFit={false}
      />
    );

    // No disabled buttons expected now (both actions are possible)
    const disabledCount = (html.match(/<button[^>]*disabled/gi) || []).length;
    expect(disabledCount).toBe(0);
  });

  it('reflects autoFit prop via checkbox checked attribute', () => {
    const visState = createVisualizationState();

    const htmlChecked = renderToString(
      <LayoutControls
        visualizationState={visState}
        currentLayout="layered"
        autoFit={true}
      />
    );
    const htmlUnchecked = renderToString(
      <LayoutControls
        visualizationState={visState}
        currentLayout="layered"
        autoFit={false}
      />
    );

    expect(/<input[^>]*type="checkbox"[^>]*checked/gi.test(htmlChecked)).toBe(true);
    expect(/<input[^>]*type="checkbox"[^>]*checked/gi.test(htmlUnchecked)).toBe(false);
  });
});
