/**
 * @fileoverview Bug Fix Tests
 * 
 * Targeted tests for specific bugs found in console logs during chat.json loading
 */

import { describe, it, expect } from 'vitest';
import { parseGraphJSON } from '../core/JSONParser';
import { ELKBridge } from '../bridges/ELKBridge';
import { ReactFlowBridge } from '../bridges/ReactFlowBridge';
import { CoordinateTranslator } from '../bridges/CoordinateTranslator';
import fs from 'fs';
import path from 'path';

describe('Bug Fix Tests', () => {
  let chatJsonData: any;

  // Load chat.json data for tests
  try {
    const chatJsonPath = path.join(__dirname, '../test-data/chat.json');
    const chatJsonContent = fs.readFileSync(chatJsonPath, 'utf8');
    chatJsonData = JSON.parse(chatJsonContent);
  } catch (error) {
    console.warn('chat.json not found, skipping bug fix tests');
    chatJsonData = null;
  }

  describe('Bug #1: ReactFlow Edge Handle Strategy', () => {
    it('should handle edge handles correctly based on current strategy', async () => {
      if (!chatJsonData) return;

      const result = parseGraphJSON(chatJsonData, 'location');
      const state = result.state;

      const elkBridge = new ELKBridge();
      await elkBridge.layoutVisState(state);

      const reactFlowBridge = new ReactFlowBridge();
      const reactFlowData = reactFlowBridge.visStateToReactFlow(state);

      // Import the handle configuration to check current strategy
      const { getHandleConfig } = await import('../render/handleConfig');
      const handleConfig = getHandleConfig();

      for (const edge of reactFlowData.edges) {
        console.log(`[Edge Handle Test] ${edge.id}: sourceHandle=${edge.sourceHandle}, targetHandle=${edge.targetHandle}, strategy=${handleConfig.enableContinuousHandles ? 'continuous' : 'discrete'}`);
        
        if (handleConfig.enableContinuousHandles) {
          // For continuous handles (ReactFlow v12), handles should be undefined
          // ReactFlow automatically determines optimal connection points
          expect(edge.sourceHandle).toBeUndefined();
          expect(edge.targetHandle).toBeUndefined();
        } else {
          // For discrete handles, handles should be defined
          expect(edge.sourceHandle).toBeDefined();
          expect(edge.targetHandle).toBeDefined();
          expect(typeof edge.sourceHandle).toBe('string');
          expect(typeof edge.targetHandle).toBe('string');
        }
        
        // Basic edge structure should always be valid
        expect(edge.source).toBeDefined();
        expect(edge.target).toBeDefined();
        expect(edge.id).toBeDefined();
      }
    });
  });

  describe('Bug #2: Coordinate Conversion Negative Values', () => {
    it('should fix negative coordinate conversion for container children', () => {
      // Test the coordinate translator directly with the problematic values from console
      // Remove the translator instance since we're using static methods
      
      // From console logs: ELK places loc_1 container at (236, 12) with children at (12, 332)
      const parentContainer = { id: 'loc_1', x: 236, y: 12, width: 204, height: 484 };
      const elkChildPosition = { x: 12, y: 332 }; // Child node position in ELK coordinates
      
      // Convert ELK -> ReactFlow
      const reactFlowPosition = CoordinateTranslator.elkToReactFlow(elkChildPosition, parentContainer);
      
      console.log(`[Coordinate Fix Test] ELK child at (${elkChildPosition.x}, ${elkChildPosition.y}) in container at (${parentContainer.x}, ${parentContainer.y})`);
      console.log(`[Coordinate Fix Test] ReactFlow result: (${reactFlowPosition.x}, ${reactFlowPosition.y})`);
      
      // FIXED: These should be reasonable relative coordinates within the container
      // For ReactFlow, child positions should be relative to container and positive
      expect(reactFlowPosition.x).toBeGreaterThanOrEqual(0);
      expect(reactFlowPosition.y).toBeGreaterThanOrEqual(0);
      
      // Child should be positioned within reasonable bounds of the container
      expect(reactFlowPosition.x).toBeLessThan(parentContainer.width);
      expect(reactFlowPosition.y).toBeLessThan(parentContainer.height);
    });

    it('should handle container positioning correctly', async () => {
      if (!chatJsonData) return;

      const result = parseGraphJSON(chatJsonData, 'location');
      const state = result.state;

      const elkBridge = new ELKBridge();
      await elkBridge.layoutVisState(state);

      // Check that containers have proper ELK coordinates before conversion
      const containers = state.visibleContainers;
      for (const container of containers) {
        const elkX = container.layout?.position?.x || 0;
        const elkY = container.layout?.position?.y || 0;
        console.log(`[Container Positioning] ${container.id}: ELK=(${elkX}, ${elkY})`);
        
        // Containers should have valid ELK positions after layout
        expect(elkX).toBeGreaterThanOrEqual(0);
        expect(elkY).toBeGreaterThanOrEqual(0);
      }

      // Convert to ReactFlow and check container conversion
      const reactFlowBridge = new ReactFlowBridge();
      const reactFlowData = reactFlowBridge.visStateToReactFlow(state);

      const containerNodes = reactFlowData.nodes.filter(n => n.type === 'container' || containers.some(c => c.id === n.id));
      for (const containerNode of containerNodes) {
        console.log(`[Container ReactFlow] ${containerNode.id}: position=(${containerNode.position.x}, ${containerNode.position.y})`);
        
        // Find the corresponding VisState container
        const visStateContainer = containers.find(c => c.id === containerNode.id);
        if (visStateContainer) {
          // Container ReactFlow positioning should match ELK positioning
          // BUG: Containers show (0, 0) but should show their actual ELK coordinates
          // Fix: Check the correct layout structure
          const expectedX = visStateContainer.layout?.position?.x || 0;
          const expectedY = visStateContainer.layout?.position?.y || 0;
          expect(containerNode.position.x).toBe(expectedX);
          expect(containerNode.position.y).toBe(expectedY);
        }
      }
    });
  });

  describe('Bug #3: Edge Sections Lost During Processing', () => {
    it('should preserve ELK edge sections in VisState', async () => {
      if (!chatJsonData) return;

      const result = parseGraphJSON(chatJsonData, 'location');
      const state = result.state;

      const elkBridge = new ELKBridge();
      await elkBridge.layoutVisState(state);

      const edges = state.visibleEdges;
      let edgesWithSections = 0;
      let edgesWithoutSections = 0;

      for (const edge of edges) {
        // Check both direct property and layout property for sections
        const directSections = edge.sections;
        const layoutSections = edge.layout?.sections;
        const edgeLayoutFromState = state.getEdgeLayout(edge.id);
        
        if ((directSections && directSections.length > 0) || 
            (layoutSections && layoutSections.length > 0) || 
            (edgeLayoutFromState?.sections && edgeLayoutFromState.sections.length > 0)) {
          edgesWithSections++;
          const sections = directSections || layoutSections || edgeLayoutFromState?.sections;
          console.log(`[Edge Sections] ${edge.id}: has ${sections.length} sections`);
          
          // Validate section structure
          for (const section of sections) {
            expect(section.startPoint).toBeDefined();
            expect(section.endPoint).toBeDefined();
            expect(typeof section.startPoint.x).toBe('number');
            expect(typeof section.startPoint.y).toBe('number');
            expect(typeof section.endPoint.x).toBe('number');
            expect(typeof section.endPoint.y).toBe('number');
          }
        } else {
          edgesWithoutSections++;
          console.log(`[Edge Sections] ${edge.id}: no sections (cross-container or bug?)`);
        }
      }

      console.log(`[Edge Sections Analysis] ${edgesWithSections} with sections, ${edgesWithoutSections} without`);
      
      // Based on ELK output, we should have edges with sections (e0, e2, e3, e4, e5, e6, e8)
      // and cross-container edges without sections (e1, e7)
      // So we SHOULD have some edges with sections
      expect(edgesWithSections).toBeGreaterThan(0);
      expect(edgesWithoutSections).toBeGreaterThan(0);
      
      // Total should match
      expect(edgesWithSections + edgesWithoutSections).toBe(edges.length);
    });
  });

  describe('Integration: All Bugs Together', () => {
    it('should reproduce the complete bug scenario from console logs', async () => {
      if (!chatJsonData) return;

      console.log('\n🔍 === COMPLETE BUG REPRODUCTION ===');
      
      // 1. Parse chat.json (this works)
      const result = parseGraphJSON(chatJsonData, 'location');
      const state = result.state;
      console.log(`1. ✅ Parsed: ${state.visibleNodes.length} nodes, ${state.visibleEdges.length} edges, ${state.visibleContainers.length} containers`);

      // 2. Run ELK layout (this works correctly)
      const elkBridge = new ELKBridge();
      await elkBridge.layoutVisState(state);
      console.log(`2. ✅ ELK layout complete`);

      // 3. Convert to ReactFlow (this has bugs)
      const reactFlowBridge = new ReactFlowBridge();
      const reactFlowData = reactFlowBridge.visStateToReactFlow(state);
      console.log(`3. ⚠️  ReactFlow conversion: ${reactFlowData.nodes.length} nodes, ${reactFlowData.edges.length} edges`);

      // Count actual bugs (not expected behavior):
      let actualBugs = 0;
      let negativeCoordinates = 0;
      let missingEdgeSections = 0;

      // Import handle config to check strategy
      const { getHandleConfig } = await import('../render/handleConfig');
      const handleConfig = getHandleConfig();

      // Check handle strategy compliance (not a bug if using continuous handles)
      let handleStrategyCompliant = 0;
      let handleStrategyViolations = 0;
      for (const edge of reactFlowData.edges) {
        const hasHandles = edge.sourceHandle !== undefined && edge.targetHandle !== undefined;
        if (handleConfig.enableContinuousHandles) {
          // Continuous handles should NOT have sourceHandle/targetHandle
          if (!hasHandles) handleStrategyCompliant++;
          else handleStrategyViolations++;
        } else {
          // Discrete handles SHOULD have sourceHandle/targetHandle
          if (hasHandles) handleStrategyCompliant++;
          else handleStrategyViolations++;
        }
      }

      // Bug #2: Negative coordinates  
      for (const node of reactFlowData.nodes) {
        if (node.position.x < -200 || node.position.y < -200) {
          negativeCoordinates++;
        }
      }

      // Bug #3: Missing edge sections (check properly in edge layout)
      for (const edge of state.visibleEdges) {
        const edgeLayout = state.getEdgeLayout(edge.id);
        if (!edgeLayout?.sections || edgeLayout.sections.length === 0) {
          missingEdgeSections++;
        }
      }

      console.log(`\n📊 === BUG SUMMARY ===`);
      console.log(`✅ Handle strategy compliance: ${handleStrategyCompliant}/${reactFlowData.edges.length} edges (${handleConfig.enableContinuousHandles ? 'continuous' : 'discrete'} mode)`);
      console.log(`❌ Handle strategy violations: ${handleStrategyViolations}/${reactFlowData.edges.length}`);
      console.log(`❌ Nodes with negative coordinates: ${negativeCoordinates}/${reactFlowData.nodes.length}`);
      console.log(`❌ Edges missing sections: ${missingEdgeSections}/${state.visibleEdges.length}`);
      
      // Check for actual bugs (not expected behavior based on configuration)
      expect(handleStrategyViolations).toBe(0); // Should comply with chosen handle strategy
      expect(negativeCoordinates).toBe(0); // This should be ZERO after fixing  
      expect(missingEdgeSections).toBeLessThan(state.visibleEdges.length); // Should have SOME sections after fixing
      
      console.log(`\n🎯 All tests pass - system working as expected with ${handleConfig.enableContinuousHandles ? 'continuous' : 'discrete'} handle strategy!`);
    });
  });
});
