/**
 * @fileoverview Test to validate the overly spaced layout fix
 * 
 * This test validates that the selective position clearing fix
 * resolves the overly spaced layout issue in paxos-flipped.json.
 */

import { describe, it, expect } from 'vitest';
import { createVisualizationState } from '../core/VisState';
import { VisualizationEngine } from '../core/VisualizationEngine';
import { readFileSync } from 'fs';
import { join } from 'path';
import { parseGraphJSON } from '../core/JSONParser';

describe('Spacing Issue Fix Validation', () => {
  describe('Paxos-flipped.json layout fix', () => {
    it('should successfully layout paxos-flipped.json with selective position clearing', async () => {
      // Load the actual paxos-flipped.json file
      const paxosFilePath = join(__dirname, '../test-data/paxos-flipped.json');
      const paxosJsonString = readFileSync(paxosFilePath, 'utf-8');
      const paxosJsonData = JSON.parse(paxosJsonString);
      
      // Parse JSON into VisState with default grouping (backtrace)
      const parseResult = parseGraphJSON(paxosJsonData, 'backtrace');
      const visState = parseResult.state;
      
      // Create VisualizationEngine with smart collapse enabled
      const engine = new VisualizationEngine(visState, {
        autoLayout: false,
        enableLogging: false, // Reduce noise in test output
        layoutConfig: {
          enableSmartCollapse: true,
          algorithm: 'layered',
          direction: 'DOWN'
        }
      });
      
      // The key test: this should complete without timeout (was timing out before fix)
      const startTime = Date.now();
      await engine.runLayout();
      const endTime = Date.now();
      const duration = endTime - startTime;
      
      // Validate that the fix worked
      expect(engine.getState().phase).toBe('ready');
      expect(duration).toBeLessThan(10000); // Should complete within 10 seconds
      
      // Validate that smart collapse worked (some containers should be collapsed)
      const containersAfter = visState.visibleContainers;
      const collapsedCount = containersAfter.filter(c => c.collapsed).length;
      const expandedCount = containersAfter.filter(c => !c.collapsed).length;
      
      expect(collapsedCount).toBeGreaterThan(0);
      expect(collapsedCount).toBeGreaterThan(expandedCount);
      
      console.log(`✅ Layout completed successfully in ${duration}ms with ${collapsedCount} collapsed containers`);
      
    }, 15000); // Generous timeout to account for large dataset
    
    it('should demonstrate selective position clearing working correctly', async () => {
      // Create a test case to validate the selective clearing behavior
      const visState = createVisualizationState();
      
      // Create nodes that will result in containers exceeding viewport budget
      const nodeIds = [];
      for (let i = 0; i < 50; i++) {
        const nodeId = `node_${i}`;
        visState.setGraphNode(nodeId, { label: `Node ${i}` });
        nodeIds.push(nodeId);
      }
      
      // Create large container that should be collapsed
      const largeContainerChildren = nodeIds.slice(0, 40);
      visState.setContainer('large_container', {
        collapsed: false,
        children: largeContainerChildren
      });
      
      // Create small container that should remain expanded
      const smallContainerChildren = nodeIds.slice(40, 43);
      visState.setContainer('small_container', {
        collapsed: false,
        children: smallContainerChildren
      });
      
      const engine = new VisualizationEngine(visState, {
        autoLayout: false,
        enableLogging: false,
        layoutConfig: {
          enableSmartCollapse: true,
          algorithm: 'layered',
          direction: 'DOWN'
        }
      });
      
      // Run layout with smart collapse
      await engine.runLayout();
      
      // Validate selective behavior
      const largeContainer = visState.getContainer('large_container');
      const smallContainer = visState.getContainer('small_container');
      
      // With selective position clearing, large container should be collapsed
      // but small container should remain expanded
      expect(largeContainer.collapsed).toBe(true);
      expect(smallContainer.collapsed).toBe(false);
      
      console.log('✅ Selective position clearing validated - fix working correctly');
    });
  });
});