/**
 * Smart Collapse Guardrails - Regression tests
 *
 * Verifies that smart collapse only influences the initial layout (or after
 * a layout algorithm change) and does not override user-initiated expands,
 * including ExpandAll.
 */

import { describe, it, expect } from 'vitest';
import paxosFlippedData from '../../test-data/paxos-flipped.json';
import { parseGraphJSON } from '../../core/JSONParser';
import { createVisualizationEngine } from '../../core/VisualizationEngine';

describe('Smart Collapse Guardrails', () => {
  it('does not re-collapse a container after manual expand followed by re-layout', async () => {
    const { state } = parseGraphJSON(paxosFlippedData);

    const engine = createVisualizationEngine(state, {
      enableLogging: false,
      layoutConfig: { enableSmartCollapse: true, algorithm: 'mrtree', direction: 'DOWN' }
    });

    // Initial layout runs smart collapse once
    await engine.runLayout();

    // Pick a collapsed top-level container
    const topLevel = state.getTopLevelContainers();
    const collapsedTopLevel = topLevel.filter(c => c.collapsed);
    expect(collapsedTopLevel.length).toBeGreaterThan(0);
    const targetId = collapsedTopLevel[0].id;

    // User manually expands a container
    state.expandContainer(targetId);

    // Re-run layout: should NOT re-run smart collapse (layoutCount > 0)
    await engine.runLayout();

    const container = state.getContainer(targetId)!;
    expect(container).toBeDefined();
    expect(container.collapsed).toBe(false);
  });

  it('keeps containers expanded after ExpandAll followed by re-layout', async () => {
    const { state } = parseGraphJSON(paxosFlippedData);

    const engine = createVisualizationEngine(state, {
      enableLogging: false,
      layoutConfig: { enableSmartCollapse: true, algorithm: 'mrtree', direction: 'DOWN' }
    });

    // Initial layout + smart collapse
    await engine.runLayout();

    // User clicks ExpandAll
    state.expandAllContainers();

    // Re-run layout: should respect user action
    await engine.runLayout();

    const topAfter = state.getTopLevelContainers();
    expect(topAfter.length).toBeGreaterThan(0);
    // All top-level containers should remain expanded
    expect(topAfter.every(c => c.collapsed === false)).toBe(true);
  });
});
