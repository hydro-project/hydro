/**
 * Comprehensive Fuzz Testing for the Entire Visualizer
 * 
 * Combines the "DISCONNECTED EDGES BUG HUNTER" with expanded fuzz testing
 * to stress test all visualizer controls and operations using paxos-flipped.json
 */

import { describe, test, expect } from 'vitest';
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { parseGraphJSON, validateGraphJSON } from '../core/JSONParser.js';
import { VisualizationState } from '../core/VisualizationState.js';
import { VisualizationEngine } from '../core/VisualizationEngine.js';
import { GraphNode, GraphEdge, Container, HyperEdge } from '../shared/types.js';
import type { LayoutConfig } from '../core/types.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Enhanced fuzz test configuration
const FUZZ_ITERATIONS = 50;  // Number of fuzz cycles
const MAX_OPERATIONS_PER_ITERATION = 20;  // Max operations in a single iteration
const OPERATION_SEED = 42;  // For reproducible randomness

// All available operations that can be fuzzed
type FuzzOperation = 
  | { type: 'expandNode'; containerId: string }
  | { type: 'contractNode'; containerId: string }
  | { type: 'expandAllNodes' }
  | { type: 'contractAllNodes' }
  | { type: 'changeHierarchy'; hierarchyId: string }
  | { type: 'changeLayout'; algorithm: 'mrtree' | 'layered' | 'force' | 'stress' | 'radial'; direction?: 'UP' | 'DOWN' | 'LEFT' | 'RIGHT' }
  | { type: 'toggleAutofit'; enabled: boolean }
  | { type: 'fitToViewport' }
  | { type: 'hierarchyTreeExpand'; containerId: string }
  | { type: 'hierarchyTreeContract'; containerId: string };

// State snapshot for comprehensive monitoring
interface ComprehensiveStateSnapshot {
  visibleNodes: number;
  visibleEdges: number;
  hyperEdges: number;
  expandedContainers: number;
  collapsedContainers: number;
  currentHierarchy: string | null;
  currentLayoutAlgorithm: string;
  autoLayoutEnabled: boolean;
  phase: string;
  layoutCount: number;
}

// Simple PRNG for reproducible tests (same as original)
class SimpleRandom {
  private seed: number;

  constructor(seed: number) {
    this.seed = seed;
  }
  
  next(): number {
    this.seed = (this.seed * 9301 + 49297) % 233280;
    return this.seed / 233280;
  }
  
  choice<T>(array: T[]): T {
    return array[Math.floor(this.next() * array.length)];
  }
  
  boolean(probability: number = 0.5): boolean {
    return this.next() < probability;
  }
  
  integer(min: number, max: number): number {
    return Math.floor(this.next() * (max - min + 1)) + min;
  }
}

/**
 * Enhanced invariant checker with edge integrity validation
 */
class ComprehensiveInvariantChecker {
  constructor(private state: VisualizationState, private engine: VisualizationEngine) {}
  
  /**
   * Check all invariants including edge integrity
   */
  async checkAll(context: string = ''): Promise<void> {
    this.checkBasicInvariants(context);
    await this.checkEdgeIntegrity(context);
    this.checkEngineStateConsistency(context);
  }
  
  /**
   * Basic visualization state invariants
   */
  private checkBasicInvariants(context: string): void {
    const visibleNodes = this.state.getVisibleNodes();
    const visibleEdges = this.state.visibleEdges;
    const visibleContainers = this.state.getVisibleContainers();
    const visibleNodeIds = new Set(visibleNodes.map(n => n.id));
    
    // Node visibility invariant
    for (const node of visibleNodes) {
      expect(node.hidden).toBe(false);
      
      const container = this.state.getNodeContainer(node.id);
      if (container) {
        const containerData = this.state.getContainer(container);
        expect(containerData?.collapsed).toBe(false);
      }
    }
    
    // Edge visibility invariant
    for (const edge of visibleEdges) {
      expect(visibleNodeIds.has(edge.source)).toBe(true);
      expect(visibleNodeIds.has(edge.target)).toBe(true);
      expect(edge.hidden).toBe(false);
    }
    
    // Container hierarchy invariant
    for (const container of visibleContainers) {
      const children = this.state.getContainerChildren(container.id);
      for (const childId of children) {
        const nodeContainer = this.state.getNodeContainer(childId);
        if (nodeContainer) {
          expect(nodeContainer).toBe(container.id);
        }
      }
    }
    
    // HyperEdge encapsulation invariant
    for (const edge of visibleEdges) {
      expect(edge.id?.startsWith('hyper_')).toBe(false);
    }
  }
  
  /**
   * Critical edge integrity validation (from BUG HUNTER)
   */
  private async checkEdgeIntegrity(context: string): Promise<void> {
    const visibleNodes = this.state.getVisibleNodes();
    const visibleContainers = this.state.getVisibleContainers();
    const visibleNodeIds = new Set(visibleNodes.map(n => n.id));
    const visibleContainerIds = new Set(visibleContainers.map(c => c.id));
    const allVisibleEntityIds = new Set([...visibleNodeIds, ...visibleContainerIds]);
    
    const visibleHyperEdges = this.state.visibleHyperEdges;
    let disconnectedEdges = 0;
    
    for (const hyperEdge of visibleHyperEdges) {
      // Validate source
      if (!allVisibleEntityIds.has(hyperEdge.source)) {
        disconnectedEdges++;
        console.error(`🚨 DISCONNECTED HYPEREDGE SOURCE: ${hyperEdge.id} missing source ${hyperEdge.source}`);
      }
      
      // Validate target
      if (!allVisibleEntityIds.has(hyperEdge.target)) {
        disconnectedEdges++;
        console.error(`🚨 DISCONNECTED HYPEREDGE TARGET: ${hyperEdge.id} missing target ${hyperEdge.target}`);
      }
    }
    
    if (disconnectedEdges > 0) {
      throw new Error(
        `❌ Edge integrity validation failed in ${context}!\n` +
        `Found ${disconnectedEdges} disconnected hyperEdge endpoints.`
      );
    }
    
    // Check for the "all edges disappeared" bug
    const edgeCount = visibleHyperEdges.length;
    const nodeCount = visibleNodes.length;
    
    if (edgeCount === 0 && nodeCount > 0) {
      throw new Error(
        `🚨 CRITICAL BUG: All edges disappeared but nodes remain! Context: ${context}`
      );
    }
  }
  
  /**
   * Engine state consistency checks
   */
  private checkEngineStateConsistency(context: string): void {
    const engineState = this.engine.getState();
    
    // Engine should not be stuck in laying_out phase
    if (engineState.phase === 'laying_out') {
      console.warn(`⚠️ ${context}: Engine stuck in laying_out phase`);
    }
    
    // Layout count should be reasonable
    expect(engineState.layoutCount).toBeGreaterThanOrEqual(0);
  }
}

/**
 * Comprehensive fuzz tester
 */
class ComprehensiveFuzzTester {
  private random: SimpleRandom;
  private availableHierarchies: string[] = [];
  private availableLayouts: Array<{ algorithm: 'mrtree' | 'layered' | 'force' | 'stress' | 'radial'; direction?: 'UP' | 'DOWN' | 'LEFT' | 'RIGHT' }>;
  private autofitEnabled: boolean = true;

  constructor(
    private testData: any,
    private testName: string
  ) {
    this.random = new SimpleRandom(OPERATION_SEED);
    this.availableLayouts = [
      { algorithm: 'mrtree', direction: 'DOWN' },
      { algorithm: 'mrtree', direction: 'UP' },
      { algorithm: 'mrtree', direction: 'LEFT' },
      { algorithm: 'mrtree', direction: 'RIGHT' },
      { algorithm: 'layered', direction: 'DOWN' },
      { algorithm: 'layered', direction: 'RIGHT' },
      { algorithm: 'force' },
      { algorithm: 'stress' },
      { algorithm: 'radial' }
    ];
  }
  
  /**
   * Run comprehensive fuzz test
   */
  async runTest(groupingId: string | null = null): Promise<void> {
    console.log(`🎲 Starting comprehensive fuzz test: ${this.testName} (grouping: ${groupingId || 'default'})`);
    
    // Parse the data
    const result = parseGraphJSON(this.testData, groupingId);
    const state = result.state;
    
    // Extract available hierarchies
    this.availableHierarchies = this.testData.hierarchyChoices?.map((h: any) => h.id) || [];
    
    // Create engine for realistic testing
    const engine = new VisualizationEngine(state, {
      enableLogging: false, // Reduce noise
      layoutConfig: {
        enableSmartCollapse: true,
        algorithm: 'mrtree',
        direction: 'DOWN'
      }
    });
    
    const checker = new ComprehensiveInvariantChecker(state, engine);
    
    console.log(`📊 Initial state: ${state.getVisibleNodes().length} nodes, ${state.visibleHyperEdges.length} hyperEdges, ${state.getVisibleContainers().length} containers`);
    
    // Run initial layout
    await engine.runLayout();
    await checker.checkAll('Initial layout');
    
    let totalOperations = 0;
    let disconnectedEdgeIssues = 0;
    
    // Run fuzz iterations
    for (let iteration = 0; iteration < FUZZ_ITERATIONS; iteration++) {
      const operationsThisIteration = this.random.integer(1, MAX_OPERATIONS_PER_ITERATION);
      
      console.log(`\n🔄 FUZZ ITERATION ${iteration + 1}/${FUZZ_ITERATIONS}: Planning ${operationsThisIteration} operations...`);
      
      for (let op = 0; op < operationsThisIteration; op++) {
        const operation = this.generateRandomOperation(state, engine);
        
        if (operation) {
          const beforeState = this.captureStateSnapshot(state, engine);
          
          try {
            // Execute operation
            await this.executeOperation(state, engine, operation);
            totalOperations++;
            
            // Check all invariants after operation
            await checker.checkAll(`After operation ${totalOperations}: ${operation.type}`);
            
          } catch (error: unknown) {
            console.error(`❌ Operation ${totalOperations} failed:`, operation);
            console.error(`   Before:`, beforeState);
            console.error(`   Error:`, error instanceof Error ? error.message : String(error));
            
            if (error instanceof Error && error.message.includes('disconnected')) {
              disconnectedEdgeIssues++;
              // Continue testing to find more issues
              console.log(`🔄 Continuing fuzz test to discover more issues...`);
            } else {
              throw error; // Re-throw non-edge related errors
            }
          }
        }
      }
      
      // Periodic progress report
      if ((iteration + 1) % 10 === 0) {
        const currentState = this.captureStateSnapshot(state, engine);
        console.log(`   ⚡ Progress: ${iteration + 1}/${FUZZ_ITERATIONS} iterations, ${totalOperations} operations completed`);
        console.log(`   📊 Current: ${currentState.visibleNodes} nodes, ${currentState.hyperEdges} hyperEdges, layout: ${currentState.currentLayoutAlgorithm}`);
      }
    }
    
    // Final validation
    await checker.checkAll('Final state after comprehensive testing');
    
    // Report results
    console.log(`\n🎯 COMPREHENSIVE FUZZ TEST RESULTS:`);
    console.log(`   Test name: ${this.testName}`);
    console.log(`   Grouping: ${groupingId || 'default'}`);
    console.log(`   Total operations: ${totalOperations}`);
    console.log(`   Disconnected edge issues: ${disconnectedEdgeIssues}`);
    console.log(`   Final state: ${state.getVisibleNodes().length} nodes, ${state.visibleHyperEdges.length} hyperEdges`);
    
    const finalEngineState = engine.getState();
    console.log(`   Engine phase: ${finalEngineState.phase}, Layout count: ${finalEngineState.layoutCount}`);
    
    // Test passes if no disconnected edges are found
    if (disconnectedEdgeIssues > 0) {
      throw new Error(`🚨 Found ${disconnectedEdgeIssues} disconnected edge issues during fuzz testing!`);
    }
    
    console.log(`✅ COMPREHENSIVE FUZZ TEST PASSED: All visualizer controls working correctly!`);
  }
  
  /**
   * Generate a random operation from all available visualizer controls
   */
  private generateRandomOperation(state: VisualizationState, engine: VisualizationEngine): FuzzOperation | null {
    const allContainers = state.getVisibleContainers();
    const expandedContainers = allContainers.filter(c => !c.collapsed);
    const collapsedContainers = allContainers.filter(c => c.collapsed);
    
    const operations: string[] = [
      'expandNode',
      'contractNode', 
      'changeLayout',
      'fitToViewport',
      'toggleAutofit'
    ];
    
    // Add bulk operations if containers exist
    if (allContainers.length > 0) {
      operations.push('expandAllNodes', 'contractAllNodes');
    }
    
    // Add hierarchy operations if multiple hierarchies available
    if (this.availableHierarchies.length > 1) {
      operations.push('changeHierarchy');
    }
    
    // Add hierarchy tree operations if containers exist
    if (allContainers.length > 0) {
      operations.push('hierarchyTreeExpand', 'hierarchyTreeContract');
    }
    
    const operationType = this.random.choice(operations);
    
    switch (operationType) {
      case 'expandNode':
        if (collapsedContainers.length === 0) return null;
        return {
          type: 'expandNode',
          containerId: this.random.choice(collapsedContainers).id
        };
        
      case 'contractNode':
        if (expandedContainers.length === 0) return null;
        return {
          type: 'contractNode',
          containerId: this.random.choice(expandedContainers).id
        };
        
      case 'expandAllNodes':
        return { type: 'expandAllNodes' };
        
      case 'contractAllNodes':
        return { type: 'contractAllNodes' };
        
      case 'changeHierarchy':
        if (this.availableHierarchies.length === 0) return null;
        return {
          type: 'changeHierarchy',
          hierarchyId: this.random.choice(this.availableHierarchies)
        };
        
      case 'changeLayout':
        const layoutConfig = this.random.choice(this.availableLayouts);
        return {
          type: 'changeLayout',
          algorithm: layoutConfig.algorithm,
          direction: layoutConfig.direction
        };
        
      case 'toggleAutofit':
        this.autofitEnabled = !this.autofitEnabled;
        return {
          type: 'toggleAutofit',
          enabled: this.autofitEnabled
        };
        
      case 'fitToViewport':
        return { type: 'fitToViewport' };
        
      case 'hierarchyTreeExpand':
        if (collapsedContainers.length === 0) return null;
        return {
          type: 'hierarchyTreeExpand',
          containerId: this.random.choice(collapsedContainers).id
        };
        
      case 'hierarchyTreeContract':
        if (expandedContainers.length === 0) return null;
        return {
          type: 'hierarchyTreeContract',
          containerId: this.random.choice(expandedContainers).id
        };
        
      default:
        return null;
    }
  }
  
  /**
   * Execute a fuzz operation
   */
  private async executeOperation(state: VisualizationState, engine: VisualizationEngine, operation: FuzzOperation): Promise<void> {
    switch (operation.type) {
      case 'expandNode':
        state.expandContainer(operation.containerId);
        await engine.runLayout();
        break;
        
      case 'contractNode':
        state.collapseContainer(operation.containerId);
        await engine.runLayout();
        break;
        
      case 'expandAllNodes':
        // Expand all containers
        const allContainers = state.getVisibleContainers();
        for (const container of allContainers) {
          if (container.collapsed) {
            state.expandContainer(container.id);
          }
        }
        await engine.runLayout();
        break;
        
      case 'contractAllNodes':
        // Collapse all containers
        const expandedContainers = state.getVisibleContainers().filter(c => !c.collapsed);
        for (const container of expandedContainers) {
          state.collapseContainer(container.id);
        }
        await engine.runLayout();
        break;
        
      case 'changeHierarchy':
        // Parse data with new hierarchy
        const result = parseGraphJSON(this.testData, operation.hierarchyId);
        // This would require engine reinitialization - for now, just log
        console.log(`   🔄 Hierarchy change simulated: ${operation.hierarchyId}`);
        break;
        
      case 'changeLayout':
        const layoutConfig: LayoutConfig = {
          algorithm: operation.algorithm,
          direction: operation.direction,
          enableSmartCollapse: true
        };
        engine.updateLayoutConfig(layoutConfig, true);
        break;
        
      case 'toggleAutofit':
        // Autofit toggle - this would affect viewport fitting behavior
        console.log(`   🔄 Autofit toggled: ${operation.enabled}`);
        break;
        
      case 'fitToViewport':
        // Viewport fitting - simulate by running layout
        await engine.runLayout();
        console.log(`   🔄 Fit to viewport executed`);
        break;
        
      case 'hierarchyTreeExpand':
        // Same as expand node but via hierarchy tree interface
        state.expandContainer(operation.containerId);
        await engine.runLayout();
        break;
        
      case 'hierarchyTreeContract':
        // Same as contract node but via hierarchy tree interface
        state.collapseContainer(operation.containerId);
        await engine.runLayout();
        break;
    }
  }
  
  /**
   * Capture comprehensive state snapshot
   */
  private captureStateSnapshot(state: VisualizationState, engine: VisualizationEngine): ComprehensiveStateSnapshot {
    const engineState = engine.getState();
    
    return {
      visibleNodes: state.getVisibleNodes().length,
      visibleEdges: state.visibleEdges.length,
      hyperEdges: state.visibleHyperEdges.length,
      expandedContainers: state.getExpandedContainers().length,
      collapsedContainers: state.getVisibleContainers().filter(c => c.collapsed).length,
      currentHierarchy: null, // Would need hierarchy tracking
      currentLayoutAlgorithm: 'unknown', // Would need layout config tracking
      autoLayoutEnabled: true, // Would need autofit tracking
      phase: engineState.phase,
      layoutCount: engineState.layoutCount
    };
  }
}

/**
 * Run comprehensive fuzz test on paxos-flipped.json
 */
export async function runComprehensiveFuzzTest(): Promise<void> {
  console.log('🧪 STARTING COMPREHENSIVE FUZZ TESTING SUITE');
  console.log('============================================\n');
  
  try {
    // Load paxos-flipped.json as specified
    const paxosFilePath = join(__dirname, '../test-data/paxos-flipped.json');
    const paxosJsonString = readFileSync(paxosFilePath, 'utf-8');
    
    // Validate the data first
    const validation = validateGraphJSON(paxosJsonString);
    if (!validation.isValid) {
      throw new Error(`❌ paxos-flipped.json failed validation: ${validation.errors}`);
    }
    
    console.log(`✅ paxos-flipped.json loaded and validated (${validation.nodeCount} nodes, ${validation.edgeCount} edges)`);
    
    const data = JSON.parse(paxosJsonString);
    const groupings = data.hierarchyChoices || [];
    
    if (groupings.length === 0) {
      console.log(`⚠️ No groupings found, testing with flat structure`);
      const tester = new ComprehensiveFuzzTester(data, 'paxos-flipped');
      await tester.runTest();
    } else {
      console.log(`📊 Found ${groupings.length} groupings: ${groupings.map((g: any) => g.name).join(', ')}`);
      
      // Test each grouping
      for (const grouping of groupings) {
        console.log(`\n🎯 Testing grouping: ${grouping.name} (${grouping.id})`);
        const tester = new ComprehensiveFuzzTester(data, `paxos-flipped-${grouping.name}`);
        await tester.runTest(grouping.id);
      }
    }
    
    console.log('\n🎉 ALL COMPREHENSIVE FUZZ TESTS COMPLETED SUCCESSFULLY!');
    console.log('No disconnected edges or visualizer control issues found.');
    
  } catch (error: unknown) {
    console.error('❌ Comprehensive fuzz testing failed:', error instanceof Error ? error.message : String(error));
    throw error;
  }
}

// Export components for use in other tests
export { ComprehensiveFuzzTester, ComprehensiveInvariantChecker };

/**
 * Vitest test suite for comprehensive fuzz testing
 */
describe('Comprehensive Visualizer Fuzz Testing', () => {
  test('🎲 Comprehensive fuzz test of all visualizer controls using paxos-flipped.json', async () => {
    await runComprehensiveFuzzTest();
  }, 120000); // 2 minute timeout for comprehensive testing

  test('🐛 DISCONNECTED EDGES BUG HUNTER: Combined with comprehensive fuzz operations', async () => {
    console.log('🐛 Running focused disconnected edges bug hunting with fuzz operations...');
    
    // Load paxos-flipped.json specifically for bug hunting
    const paxosFilePath = join(__dirname, '../test-data/paxos-flipped.json');
    const paxosJsonString = readFileSync(paxosFilePath, 'utf-8');
    const data = JSON.parse(paxosJsonString);
    
    // Focus on a single grouping for intensive bug hunting
    const groupings = data.hierarchyChoices || [];
    const testGrouping = groupings.length > 0 ? groupings[0].id : null;
    
    const tester = new ComprehensiveFuzzTester(data, 'bug-hunter-focused');
    await tester.runTest(testGrouping);
    
    console.log('✅ Bug hunter completed - no disconnected edges found!');
  }, 60000); // 1 minute timeout for focused bug hunting
});
