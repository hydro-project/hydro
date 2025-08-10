/**
 * Debug test to validate smart collapse hypothesis
 */

import { VisualizationState, createVisualizationState } from '../VisState';
import { VisualizationEngine } from '../VisualizationEngine';

describe('Debug Smart Collapse', () => {
  let visState: VisualizationState;
  let engine: VisualizationEngine;

  beforeEach(() => {
    visState = createVisualizationState();
    engine = new VisualizationEngine(visState, {
      enableLogging: true, // FORCE LOGGING ON
      autoLayout: true,
      layoutConfig: {
        enableSmartCollapse: true,
        algorithm: 'layered',
        direction: 'DOWN'
      }
    });
  });

  test('should debug smart collapse conditions', async () => {
    // Create many children to make container naturally large
    const childNodes = [];
    for (let i = 0; i < 50; i++) {
      const childId = `child_${i}`;
      visState.setGraphNode(childId, { 
        label: `Child ${i}`, 
        width: 180, 
        height: 60 
      });
      childNodes.push(childId);
    }

    // Create container with many children - ELK will calculate large dimensions
    visState.setContainer('large_container', {
      collapsed: false,  // Start expanded
      hidden: false,
      children: childNodes,
      // Don't set width/height - let ELK calculate based on children
    });

    console.log('[TEST] About to call engine.runLayout()');
    
    // This should trigger smart collapse
    await engine.runLayout();

    console.log('[TEST] Layout complete, checking container state');
    
    const container = visState.getContainer('large_container');
    console.log(`[TEST] Container collapsed: ${container.collapsed}`);
    console.log(`[TEST] Container dimensions: ${container.width}x${container.height}`);
    
    // With 50 children, the container should be large enough to trigger smart collapse
    expect(container.collapsed).toBe(true);
  });
});
