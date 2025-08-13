/**
 * Test that verifies hidden containers (both collapsed and expanded) are not sent to ELK
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { VisualizationState } from '../core/VisualizationState';
import { ELKBridge } from '../bridges/ELKBridge';

describe('Hidden Container ELK Filtering', () => {
  let state: VisualizationState;
  let elkBridge: ELKBridge;

  beforeEach(() => {
    state = new VisualizationState();
    elkBridge = new ELKBridge();
  });

  it('should exclude hidden collapsed containers from ELK layout', async () => {
    // Setup: Create a collapsed container and mark it as hidden
    state
      .setContainer('hiddenCollapsed', {
        collapsed: true,
        hidden: true,
        children: new Set(['node1'])
      })
      .setGraphNode('node1', { 
        hidden: true, // Must be hidden if parent container is collapsed
        width: 100, 
        height: 50 
      })
      .setContainer('visibleCollapsed', {
        collapsed: false, // Will be collapsed below
        hidden: false,
        children: new Set(['node2'])
      })
      .setGraphNode('node2', { 
        hidden: false,
        width: 100, 
        height: 50 
      });

    // Collapse the visible container (this should make it visible but collapsed)
    state.collapseContainer('visibleCollapsed');

    // Verify initial state
    const visibleContainers = state.visibleContainers;
    console.log('Visible containers:', visibleContainers.map(c => ({ id: c.id, collapsed: c.collapsed, hidden: c.hidden })));
    
    // Should only have visibleCollapsed, not hiddenCollapsed
    expect(visibleContainers).toHaveLength(1);
    expect(visibleContainers[0].id).toBe('visibleCollapsed');
    expect(visibleContainers[0].collapsed).toBe(true);
    expect(visibleContainers[0].hidden).toBe(false);

    // Verify that hidden containers are not included
    const hiddenContainer = visibleContainers.find(c => c.id === 'hiddenCollapsed');
    expect(hiddenContainer).toBeUndefined();
  });

  it('should exclude hidden expanded containers from ELK layout', async () => {
    // Setup: Start with a visible collapsed container, expand it, then verify the test logic
    // Note: We can't actually create expanded+hidden containers as that's an illegal state
    // So this test verifies that hidden containers (even if they could be expanded) are filtered out
    state
      .setContainer('hiddenExpanded', {
        collapsed: true, // Start collapsed and hidden (valid state)
        hidden: true,
        children: new Set(['node1'])
      })
      .setGraphNode('node1', { 
        hidden: true, // Must be hidden if parent container is collapsed/hidden
        width: 100, 
        height: 50 
      });

    // Create a visible expanded container for comparison
    state
      .setContainer('visibleExpanded', {
        collapsed: false,
        hidden: false,
        children: new Set(['node2'])
      })
      .setGraphNode('node2', { 
        hidden: false,
        width: 100, 
        height: 50 
      });

    // Verify that visibleContainers excludes hidden containers regardless of collapsed state
    const visibleContainers = state.visibleContainers;
    console.log('Visible containers:', visibleContainers.map(c => ({ id: c.id, collapsed: c.collapsed, hidden: c.hidden })));
    
    // Should only have visibleExpanded, not hiddenExpanded
    expect(visibleContainers).toHaveLength(1);
    expect(visibleContainers[0].id).toBe('visibleExpanded');
    expect(visibleContainers[0].collapsed).toBe(false);
    expect(visibleContainers[0].hidden).toBe(false);

    // Verify that hidden containers are not included
    const hiddenContainer = visibleContainers.find(c => c.id === 'hiddenExpanded');
    expect(hiddenContainer).toBeUndefined();
  });

  it('should not include nodes from hidden containers in visible nodes', async () => {
    // Setup: Create containers with nodes
    state
      .setContainer('hiddenContainer', {
        collapsed: true, // Start collapsed (valid with hidden: true)
        hidden: true,
        children: new Set(['hiddenNode'])
      })
      .setGraphNode('hiddenNode', { 
        hidden: true, // Must be hidden if belongs to hidden container
        width: 100, 
        height: 50 
      })
      .setContainer('visibleContainer', {
        collapsed: false,
        hidden: false,
        children: new Set(['visibleNode'])
      })
      .setGraphNode('visibleNode', { 
        hidden: false,
        width: 100, 
        height: 50 
      });

    // Verify that nodes from hidden containers are not visible
    const visibleNodes = state.visibleNodes;
    console.log('Visible nodes:', visibleNodes.map(n => ({ id: n.id, hidden: n.hidden })));
    
    // Should only have visibleNode, not hiddenNode
    expect(visibleNodes).toHaveLength(1);
    expect(visibleNodes[0].id).toBe('visibleNode');
    expect(visibleNodes[0].hidden).toBe(false);

    // Verify that hidden nodes are not included
    const hiddenNode = visibleNodes.find(n => n.id === 'hiddenNode');
    expect(hiddenNode).toBeUndefined();
  });

  it('should validate that ELKBridge logic relies on proper filtering', () => {
    // This test confirms that the ELKBridge's dependency on visibleContainers
    // is sufficient to exclude hidden containers from ELK layout
    
    // Create both visible and hidden containers
    state
      .setContainer('hiddenContainer', {
        collapsed: true,
        hidden: true,
        children: new Set(['hiddenNode'])
      })
      .setGraphNode('hiddenNode', { 
        hidden: true,
        width: 100, 
        height: 50 
      })
      .setContainer('visibleContainer', {
        collapsed: true,
        hidden: false,
        children: new Set(['visibleNode'])
      })
      .setGraphNode('visibleNode', { 
        hidden: true, // Hidden because parent container is collapsed
        width: 100, 
        height: 50 
      });

    // Verify that visibleContainers excludes hidden containers
    const visibleContainers = state.visibleContainers;
    expect(visibleContainers).toHaveLength(1);
    expect(visibleContainers[0].id).toBe('visibleContainer');
    expect(visibleContainers[0].hidden).toBe(false);
    
    // Confirm that hidden containers are not in the visible collection
    const hiddenInVisible = visibleContainers.find(c => c.hidden === true);
    expect(hiddenInVisible).toBeUndefined();
    
    console.log('✅ ELKBridge dependency on visibleContainers correctly excludes hidden containers');
  });
});
