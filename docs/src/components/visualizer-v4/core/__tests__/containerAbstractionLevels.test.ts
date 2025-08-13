/**
 * Container Abstraction Level Tests
 * 
 * Tests the bidirectional operations between different levels of abstraction:
 * - LIFTING: Making things more abstract (collapse containers → hide details)
 * - GROUNDING: Making things more concrete (expand containers → reveal details)
 * 
 * These are inverse operations that should be perfectly symmetric.
 */

import { describe, it, expect } from 'vitest';
import { createVisualizationState, VisualizationState } from '../VisualizationState';

describe('Container Abstraction Level Tests', () => {
  
  describe('Basic Lifting/Grounding Symmetry', () => {
    /**
     * LIFTING: Collapse a container to create a higher-level abstraction
     * Test the process of hiding implementation details behind a container abstraction
     */
    it('should perform basic lifting (collapse container to hide details)', () => {
      const state: VisualizationState = createVisualizationState();
      
      // Create a concrete scenario: container with internal node connected to external node
      state.setGraphNode('internal', { label: 'Internal Implementation' });
      state.setGraphNode('external', { label: 'External Interface' });
      
      state.setContainer('abstractModule', {
        children: ['internal'],
        label: 'Abstract Module'
      });
      
      state.setGraphEdge('implementation_edge', { source: 'internal', target: 'external' });
      
      // Verify initial concrete state (grounded)
      const internalNode = state.getGraphNode('internal');
      const externalNode = state.getGraphNode('external');
      const implementationEdge = state.getGraphEdge('implementation_edge');
      
      expect(internalNode?.hidden).toBe(false);
      expect(externalNode?.hidden).toBe(false);
      expect(implementationEdge?.hidden).toBe(false);
      
      // No abstractions should exist yet
      const initialAbstractions = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
      expect(initialAbstractions.length).toBe(0);
      
      // // console.log((('  Initial concrete state verified')));
      
      // PERFORM LIFTING: Collapse container to create abstraction
      state.collapseContainer('abstractModule');
      
      // Verify lifted state (abstract)
      const internalAfterLifting = state.getGraphNode('internal');
      const externalAfterLifting = state.getGraphNode('external');
      const edgeAfterLifting = state.getGraphEdge('implementation_edge');
      
      expect(internalAfterLifting?.hidden).toBe(true); // Implementation hidden
      expect(externalAfterLifting?.hidden).toBe(false); // Interface still visible
      expect(edgeAfterLifting?.hidden).toBe(true); // Implementation edge hidden
      
      // Abstract representation should be created
      const abstractions = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
      expect(abstractions.length).toBe(1);
      
      const abstraction = abstractions[0];
      expect(abstraction.id).toBe('hyper_abstractModule_to_external');
      
      // // console.log((('  ✅ LIFTING successful: Implementation details abstracted away')));
    });

    /**
     * GROUNDING: Expand a container to reveal concrete implementation details
     * Test the inverse process of making abstractions concrete
     */
    it('should perform basic grounding (expand container to reveal details)', () => {
      const state: VisualizationState = createVisualizationState();
      
      // Start with a collapsed (abstract) state
      state.setGraphNode('internal', { label: 'Internal Implementation' });
      state.setGraphNode('external', { label: 'External Interface' });
      
      state.setContainer('abstractModule', {
        children: ['internal'],
        label: 'Abstract Module'
      });
      
      state.setGraphEdge('implementation_edge', { source: 'internal', target: 'external' });
      
      // Start in lifted (abstract) state
      state.collapseContainer('abstractModule');
      
      // Verify we're in abstract state
      const internalBeforeGrounding = state.getGraphNode('internal');
      const edgeBeforeGrounding = state.getGraphEdge('implementation_edge');
      expect(internalBeforeGrounding?.hidden).toBe(true);
      expect(edgeBeforeGrounding?.hidden).toBe(true);
      
      const abstractionsBefore = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
      expect(abstractionsBefore.length).toBe(1);
      
      // // console.log((('  Starting from abstract state')));
      
      // PERFORM GROUNDING: Expand container to reveal implementation
      state.expandContainer('abstractModule');
      
      // Verify grounded state (concrete)
      const internalAfterGrounding = state.getGraphNode('internal');
      const externalAfterGrounding = state.getGraphNode('external');
      const edgeAfterGrounding = state.getGraphEdge('implementation_edge');
      
      expect(internalAfterGrounding?.hidden).toBe(false); // Implementation revealed
      expect(externalAfterGrounding?.hidden).toBe(false); // Interface still visible
      expect(edgeAfterGrounding?.hidden).toBe(false); // Implementation edge revealed
      
      // Abstractions should be removed
      const abstractionsAfter = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
      expect(abstractionsAfter.length).toBe(0);
      
      // // console.log((('  ✅ GROUNDING successful: Implementation details revealed')));
    });

    /**
     * SYMMETRY TEST: Lifting followed by Grounding should return to original state
     */
    it('should maintain perfect symmetry: Lift → Ground = Identity', () => {
      const state: VisualizationState = createVisualizationState();
      
      // Create initial concrete state
      state.setGraphNode('internal', { label: 'Internal Implementation' });
      state.setGraphNode('external', { label: 'External Interface' });
      state.setContainer('module', { children: ['internal'] });
      state.setGraphEdge('edge', { source: 'internal', target: 'external' });
      
      // Capture initial state
      const initialNodes = state.visibleNodes.length;
      const initialEdges = state.visibleEdges.length;
      const initialAbstractions = state.visibleEdges.filter(e => e.id?.startsWith('hyper_')).length;
      
      // Apply Lifting → Grounding sequence
      state.collapseContainer('module');  // LIFT (create abstraction)
      state.expandContainer('module');    // GROUND (reveal details)
      
      // Verify we're back to original state
      expect(state.visibleNodes.length).toBe(initialNodes);
      expect(state.visibleEdges.length).toBe(initialEdges);
      
      const finalAbstractions = state.visibleEdges.filter(e => e.id?.startsWith('hyper_')).length;
      expect(finalAbstractions).toBe(initialAbstractions);
      
      // All nodes should be visible again
      expect(state.getGraphNode('internal')?.hidden).toBe(false);
      expect(state.getGraphNode('external')?.hidden).toBe(false);
      expect(state.getGraphEdge('edge')?.hidden).toBe(false);
      
      // // console.log((('  ✅ SYMMETRY verified: Lift → Ground = Identity')));
    });
  });

  describe('Multi-Container Lifting/Grounding', () => {
    /**
     * PROGRESSIVE LIFTING: Collapse multiple containers to create layered abstractions
     */
    it('should handle progressive lifting (multiple containers → layered abstractions)', () => {
      const state: VisualizationState = createVisualizationState();
      
      // Create complex concrete scenario
      state.setGraphNode('impl1', { label: 'Implementation 1' });
      state.setGraphNode('impl2', { label: 'Implementation 2' });
      state.setGraphNode('impl3', { label: 'Implementation 3' });
      state.setGraphNode('impl4', { label: 'Implementation 4' });
      state.setGraphNode('client', { label: 'Client' });
      
      state.setContainer('moduleA', {
        children: ['impl1', 'impl2'],
        label: 'Module A'
      });
      
      state.setContainer('moduleB', {
        children: ['impl3', 'impl4'],
        label: 'Module B'
      });
      
      // Create implementation edges and client interfaces
      state.setGraphEdge('internal_A', { source: 'impl1', target: 'impl2' });
      state.setGraphEdge('internal_B', { source: 'impl3', target: 'impl4' });
      state.setGraphEdge('cross_module', { source: 'impl1', target: 'impl3' });
      state.setGraphEdge('client_A', { source: 'impl2', target: 'client' });
      state.setGraphEdge('client_B', { source: 'impl4', target: 'client' });
      
      // Verify initial concrete state
      expect(state.visibleNodes.length).toBe(5);
      expect(state.visibleEdges.length).toBe(5);
      expect(state.visibleEdges.filter(e => e.id?.startsWith('hyper_')).length).toBe(0);
      
      // PROGRESSIVE LIFTING: First level of abstraction
      state.collapseContainer('moduleA');
      
      expect(state.visibleNodes.length).toBe(3); // impl3, impl4, client (moduleA abstracted)
      const firstLevelAbstractions = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
      expect(firstLevelAbstractions.length).toBeGreaterThan(0);
      
      // PROGRESSIVE LIFTING: Second level of abstraction  
      state.collapseContainer('moduleB');
      
      expect(state.visibleNodes.length).toBe(1); // Just client (both modules abstracted)
      const secondLevelAbstractions = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
      expect(secondLevelAbstractions.length).toBeGreaterThan(0);
      
      // // console.log((('  ✅ PROGRESSIVE LIFTING successful: Multiple abstraction levels created')));
    });

    /**
     * PROGRESSIVE GROUNDING: Expand multiple containers to reveal layered implementations
     */
    it('should handle progressive grounding (layered abstractions → detailed implementations)', () => {
      const state: VisualizationState = createVisualizationState();
      
      // Set up scenario and start in fully abstract state
      state.setGraphNode('impl1', { label: 'Implementation 1' });
      state.setGraphNode('impl2', { label: 'Implementation 2' });
      state.setGraphNode('impl3', { label: 'Implementation 3' });
      state.setGraphNode('impl4', { label: 'Implementation 4' });
      state.setGraphNode('client', { label: 'Client' });
      
      state.setContainer('moduleA', { children: ['impl1', 'impl2'] });
      state.setContainer('moduleB', { children: ['impl3', 'impl4'] });
      
      state.setGraphEdge('internal_A', { source: 'impl1', target: 'impl2' });
      state.setGraphEdge('internal_B', { source: 'impl3', target: 'impl4' });
      state.setGraphEdge('cross_module', { source: 'impl1', target: 'impl3' });
      state.setGraphEdge('client_A', { source: 'impl2', target: 'client' });
      state.setGraphEdge('client_B', { source: 'impl4', target: 'client' });
      
      // Start in fully abstract state
      state.collapseContainer('moduleA');
      state.collapseContainer('moduleB');
      
      expect(state.visibleNodes.length).toBe(1); // Just client
      
      // PROGRESSIVE GROUNDING: First level of detail
      state.expandContainer('moduleA');
      
      expect(state.visibleNodes.length).toBe(3); // impl1, impl2, client (moduleA grounded)
      
      // PROGRESSIVE GROUNDING: Full detail level
      state.expandContainer('moduleB');
      
      expect(state.visibleNodes.length).toBe(5); // All implementations visible
      expect(state.visibleEdges.length).toBe(5); // All implementation edges visible
      expect(state.visibleEdges.filter(e => e.id?.startsWith('hyper_')).length).toBe(0);
      
      // // console.log((('  ✅ PROGRESSIVE GROUNDING successful: Full implementation details revealed')));
    });
  });

  describe('Nested Container Lifting/Grounding', () => {
    /**
     * HIERARCHICAL LIFTING: Collapse nested containers to create hierarchical abstractions
     */
    it('should handle hierarchical lifting (nested containers → hierarchical abstractions)', () => {
      const state: VisualizationState = createVisualizationState();
      
      // Create hierarchical concrete structure
      state.setGraphNode('coreImpl1', { label: 'Core Implementation 1' });
      state.setGraphNode('coreImpl2', { label: 'Core Implementation 2' });
      state.setGraphNode('consumer', { label: 'Consumer' });
      
      state.setContainer('coreModule', {
        children: ['coreImpl1', 'coreImpl2'],
        label: 'Core Module'
      });
      
      state.setContainer('systemModule', {
        children: ['coreModule'], // Container contains another container
        label: 'System Module'
      });
      
      state.setGraphEdge('core_internal', { source: 'coreImpl1', target: 'coreImpl2' });
      state.setGraphEdge('system_interface', { source: 'coreImpl1', target: 'consumer' });
      
      // HIERARCHICAL LIFTING: Collapse the outer container
      state.collapseContainer('systemModule');
      
      // The entire hierarchy should be abstracted away
      expect(state.visibleNodes.length).toBe(1); // Just consumer
      
      const hierarchicalAbstractions = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
      expect(hierarchicalAbstractions.length).toBe(1);
      
      // // console.log((('  ✅ HIERARCHICAL LIFTING successful: Nested structure abstracted')));
    });

    /**
     * HIERARCHICAL GROUNDING: Expand nested containers to reveal hierarchical implementations  
     */
    it('should handle hierarchical grounding (hierarchical abstractions → nested implementations)', () => {
      const state: VisualizationState = createVisualizationState();
      
      // Set up hierarchical structure and start abstract
      state.setGraphNode('coreImpl1', { label: 'Core Implementation 1' });
      state.setGraphNode('coreImpl2', { label: 'Core Implementation 2' });
      state.setGraphNode('consumer', { label: 'Consumer' });
      
      state.setContainer('coreModule', { children: ['coreImpl1', 'coreImpl2'] });
      state.setContainer('systemModule', { children: ['coreModule'] });
      
      state.setGraphEdge('core_internal', { source: 'coreImpl1', target: 'coreImpl2' });
      state.setGraphEdge('system_interface', { source: 'coreImpl1', target: 'consumer' });
      
      // Start in abstract state
      state.collapseContainer('systemModule');
      expect(state.visibleNodes.length).toBe(1);
      
      // HIERARCHICAL GROUNDING: Expand the outer container recursively
      state.expandContainerRecursive('systemModule');
      
      // Inner details should be revealed (coreModule expanded by default)
      expect(state.visibleNodes.length).toBe(3); // coreImpl1, coreImpl2, consumer
      expect(state.visibleEdges.length).toBe(2); // Both edges visible
      expect(state.visibleEdges.filter(e => e.id?.startsWith('hyper_')).length).toBe(0);
      
      // // console.log((('  ✅ HIERARCHICAL GROUNDING successful: Nested implementation revealed')));
    });

    /**
     * NESTED SYMMETRY TEST: Complex Lift → Ground should preserve nested structure
     */
    it('should maintain nested symmetry: Complex Lift → Ground = Identity', () => {
      const state: VisualizationState = createVisualizationState();
      
      // Create complex nested structure
      state.setGraphNode('core1', { label: 'Core 1' });
      state.setGraphNode('core2', { label: 'Core 2' });
      state.setGraphNode('external', { label: 'External' });
      
      state.setContainer('inner', { children: ['core1', 'core2'] });
      state.setContainer('outer', { children: ['inner'] });
      
      state.setGraphEdge('core_edge', { source: 'core1', target: 'core2' });
      state.setGraphEdge('external_edge', { source: 'core1', target: 'external' });
      
      // Capture initial state
      const initialNodes = state.visibleNodes.length;
      const initialEdges = state.visibleEdges.length;
      
      // Complex Lifting → Grounding sequence
      state.collapseContainer('outer');  // LIFT to highest abstraction
      state.expandContainerRecursive('outer');    // GROUND to reveal structure recursively
      
      // Should return to original nested concrete state
      expect(state.visibleNodes.length).toBe(initialNodes);
      expect(state.visibleEdges.length).toBe(initialEdges);
      expect(state.visibleEdges.filter(e => e.id?.startsWith('hyper_')).length).toBe(0);
      
      // // console.log((('  ✅ NESTED SYMMETRY verified: Complex Lift → Ground = Identity')));
    });
  });
});
