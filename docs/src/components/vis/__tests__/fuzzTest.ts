/**
 * Fuzz Testing for VisualizationState (TypeScript Version)
 * 
 * Performs randomized collapse/expand operations on parsed JSON data
 * and validates all system invariants throughout the process.
 */

import assert from 'assert';
import { readFile } from 'fs/promises';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { parseGraphJSON, validateGraphJSON, ValidationResult } from '../dist/core/JSONParser.js';
import { VisualizationState } from '../dist/core/VisState.js';
import { GraphNode, GraphEdge, Container, HyperEdge } from '../shared/types.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Fuzz test configuration
const FUZZ_ITERATIONS = 100;  // Number of random operations per test
const MAX_OPERATIONS_PER_ITERATION = 50;  // Max operations in a single iteration
const OPERATION_SEED = 42;  // For reproducible randomness

// Operation type definition
interface Operation {
  type: 'collapse' | 'expand';
  containerId: string;
}

// State snapshot for debugging
interface StateSnapshot {
  visibleNodes: number;
  visibleEdges: number;
  hyperEdges: number;
  expandedContainers: number;
  collapsedContainers: number;
}

// Simple PRNG for reproducible tests
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
}

/**
 * System invariants that must always hold
 */
class InvariantChecker {
  constructor(private state: VisualizationState) {}
  
  /**
   * Check all invariants and throw if any are violated
   */
  checkAll(context: string = ''): void {
    this.checkNodeVisibilityInvariant(context);
    this.checkEdgeVisibilityInvariant(context);
    this.checkContainerHierarchyInvariant(context);
    this.checkHyperEdgeConsistency(context);
    this.checkCollectionConsistency(context);
  }
  
  /**
   * Invariant: A node is visible iff it's not hidden and no parent container is collapsed
   */
  private checkNodeVisibilityInvariant(context: string): void {
    const visibleNodes = this.state.visibleNodes;
    const visibleNodeIds = new Set(visibleNodes.map(n => n.id));
    
    // Check all nodes through public API
    for (const node of visibleNodes) {
      // If a node is in visibleNodes, it should not be hidden and not in collapsed container
      assert.strictEqual(
        node.hidden,
        false,
        `${context}: Visible node ${node.id} should not be hidden`
      );
      
      const container = this.state.getNodeContainer(node.id);
      if (container) {
        const containerData = this.state.getContainer(container);
        assert.strictEqual(
          containerData?.collapsed,
          false,
          `${context}: Visible node ${node.id} should not be in collapsed container ${container}`
        );
      }
    }
  }
  
  /**
   * Invariant: An edge is visible iff both its endpoints are visible
   */
  private checkEdgeVisibilityInvariant(context: string): void {
    const visibleEdges = this.state.visibleEdges;
    const visibleNodes = this.state.visibleNodes;
    const visibleNodeIds = new Set(visibleNodes.map(n => n.id));
    
    for (const edge of visibleEdges) {
      // Both endpoints of visible edges must be visible
      assert(
        visibleNodeIds.has(edge.source),
        `${context}: Edge ${edge.id} source ${edge.source} should be visible`
      );
      assert(
        visibleNodeIds.has(edge.target),
        `${context}: Edge ${edge.id} target ${edge.target} should be visible`
      );
      
      // Edge should not be hidden
      assert.strictEqual(
        edge.hidden,
        false,
        `${context}: Visible edge ${edge.id} should not be hidden`
      );
    }
  }
  
  /**
   * Invariant: Container hierarchy relationships are consistent
   */
  private checkContainerHierarchyInvariant(context: string): void {
    const allContainers = this.state.visibleContainers;
    
    for (const container of allContainers) {
      const children = this.state.getContainerChildren(container.id);
      
      // Check that all children reference this container as parent
      for (const childId of children) {
        const nodeContainer = this.state.getNodeContainer(childId);
        if (nodeContainer) {
          assert.strictEqual(
            nodeContainer,
            container.id,
            `${context}: Node ${childId} should have container ${container.id} as parent`
          );
        }
      }
    }
  }
  
  /**
   * Invariant: HyperEdges exist only for visible, collapsed containers and connect to visible endpoints
   */
  private checkHyperEdgeConsistency(context: string): void {
    const hyperEdges = this.state.allHyperEdges;
    const visibleNodes = this.state.visibleNodes;
    const visibleNodeIds = new Set(visibleNodes.map(n => n.id));
    const visibleContainers = this.state.visibleContainers;
    const collapsedContainerIds = new Set(
      visibleContainers.filter(c => c.collapsed).map(c => c.id)
    );
    
    // Check that hyperEdges only connect to valid endpoints
    for (const hyperEdge of hyperEdges) {
      const sourceIsCollapsedContainer = collapsedContainerIds.has(hyperEdge.source);
      const targetIsCollapsedContainer = collapsedContainerIds.has(hyperEdge.target);
      const sourceIsVisibleNode = visibleNodeIds.has(hyperEdge.source);
      const targetIsVisibleNode = visibleNodeIds.has(hyperEdge.target);
      
      // At least one endpoint should be a collapsed container
      assert(
        sourceIsCollapsedContainer || targetIsCollapsedContainer,
        `${context}: HyperEdge ${hyperEdge.id} should connect to at least one collapsed container`
      );
      
      // All endpoints should be either collapsed containers or visible nodes
      assert(
        (sourceIsCollapsedContainer || sourceIsVisibleNode) && 
        (targetIsCollapsedContainer || targetIsVisibleNode),
        `${context}: HyperEdge ${hyperEdge.id} endpoints should be either collapsed containers or visible nodes`
      );
    }
  }
  
  /**
   * Invariant: Visible collections contain exactly the items that should be visible
   */
  private checkCollectionConsistency(context: string): void {
    // Check that expanded containers collection is consistent
    const expandedContainers = this.state.expandedContainers;
    const visibleContainers = this.state.visibleContainers;
    const expectedExpandedCount = visibleContainers.filter(c => !c.collapsed).length;
    
    assert.strictEqual(
      expandedContainers.length,
      expectedExpandedCount,
      `${context}: Expanded containers collection size mismatch. Expected: ${expectedExpandedCount}, Actual: ${expandedContainers.length}`
    );
    
    // Check that all expanded containers are indeed not collapsed
    for (const container of expandedContainers) {
      assert.strictEqual(
        container.collapsed,
        false,
        `${context}: Container ${container.id} in expandedContainers should not be collapsed`
      );
    }
  }
}

/**
 * Fuzz test runner
 */
class FuzzTester {
  private random: SimpleRandom;

  constructor(
    private testData: any,
    private testName: string
  ) {
    this.random = new SimpleRandom(OPERATION_SEED);
  }
  
  /**
   * Run the fuzz test with the given grouping
   */
  async runTest(groupingId: string | null = null): Promise<void> {
    console.log(`🎲 Running fuzz test on ${this.testName} with grouping: ${groupingId || 'default'}`);
    
    // Parse the data
    const result = parseGraphJSON(this.testData, groupingId);
    const state = result.state;
    const checker = new InvariantChecker(state);
    
    const containers = state.visibleContainers;
    if (containers.length === 0) {
      console.log(`⚠️  No containers found, skipping fuzz test for ${this.testName}`);
      return;
    }
    
    console.log(`   📊 Initial state: ${state.visibleNodes.length} nodes, ${state.visibleEdges.length} edges, ${containers.length} containers`);
    
    // Check initial invariants
    checker.checkAll('Initial state');
    
    let totalOperations = 0;
    
    // Run fuzz iterations
    for (let iteration = 0; iteration < FUZZ_ITERATIONS; iteration++) {
      const operationsThisIteration = Math.floor(this.random.next() * MAX_OPERATIONS_PER_ITERATION) + 1;
      
      for (let op = 0; op < operationsThisIteration; op++) {
        const operation = this.generateRandomOperation(state);
        
        if (operation) {
          // Record state before operation
          const beforeState = this.captureStateSnapshot(state);
          
          try {
            // Execute operation
            this.executeOperation(state, operation);
            totalOperations++;
            
            // Check invariants after operation
            checker.checkAll(`After operation ${totalOperations}: ${operation.type} ${operation.containerId}`);
            
          } catch (error: unknown) {
            console.error(`❌ Operation ${totalOperations} failed:`, operation);
            console.error(`   Before:`, beforeState);
            console.error(`   Error:`, error instanceof Error ? error.message : String(error));
            throw error;
          }
        }
      }
      
      // Periodic progress update
      if ((iteration + 1) % 20 === 0) {
        console.log(`   ⚡ Completed ${iteration + 1}/${FUZZ_ITERATIONS} iterations (${totalOperations} operations)`);
      }
    }
    
    console.log(`✅ Fuzz test completed: ${totalOperations} operations, all invariants maintained`);
    
    // Final state summary
    const finalNodes = state.visibleNodes.length;
    const finalEdges = state.visibleEdges.length;
    const finalHyperEdges = state.allHyperEdges.length;
    const collapsedContainers = state.visibleContainers.filter(c => c.collapsed).length;
    
    console.log(`   📈 Final state: ${finalNodes} visible nodes, ${finalEdges} visible edges, ${finalHyperEdges} hyperEdges, ${collapsedContainers} collapsed containers`);
  }
  
  /**
   * Generate a random collapse or expand operation
   */
  private generateRandomOperation(state: VisualizationState): Operation | null {
    const allContainers = state.visibleContainers;
    if (allContainers.length === 0) return null;
    
    const expandedContainers = allContainers.filter(c => !c.collapsed);
    const collapsedContainers = allContainers.filter(c => c.collapsed);
    
    // Choose operation type based on available containers
    let operationType: 'collapse' | 'expand';
    if (expandedContainers.length === 0) {
      operationType = 'expand';
    } else if (collapsedContainers.length === 0) {
      operationType = 'collapse';
    } else {
      operationType = this.random.boolean() ? 'collapse' : 'expand';
    }
    
    // Choose container
    const targetContainers = operationType === 'collapse' ? expandedContainers : collapsedContainers;
    if (targetContainers.length === 0) return null;
    
    const container = this.random.choice(targetContainers);
    
    return {
      type: operationType,
      containerId: container.id
    };
  }
  
  /**
   * Execute a collapse or expand operation
   */
  private executeOperation(state: VisualizationState, operation: Operation): void {
    if (operation.type === 'collapse') {
      state.collapseContainer(operation.containerId);
    } else if (operation.type === 'expand') {
      state.expandContainer(operation.containerId);
    }
  }
  
  /**
   * Capture a snapshot of the current state for debugging
   */
  private captureStateSnapshot(state: VisualizationState): StateSnapshot {
    return {
      visibleNodes: state.visibleNodes.length,
      visibleEdges: state.visibleEdges.length,
      hyperEdges: state.allHyperEdges.length,
      expandedContainers: state.expandedContainers.length,
      collapsedContainers: state.visibleContainers.filter(c => c.collapsed).length
    };
  }
}

/**
 * Load test data and run fuzz tests
 */
async function runFuzzTests(): Promise<void> {
  console.log('🧪 Starting Fuzz Testing Suite\n');
  console.log('==============================\n');
  
  const testFiles = ['chat.json', 'paxos.json'];
  
  for (const filename of testFiles) {
    try {
      console.log(`📁 Loading ${filename}...`);
      
      const filePath = join(__dirname, '../test-data', filename);
      const jsonData = await readFile(filePath, 'utf-8');
      
      // Validate the data first
      const validation = validateGraphJSON(jsonData);
      if (!validation.isValid) {
        console.error(`❌ ${filename} failed validation:`, validation.errors);
        continue;
      }
      
      console.log(`✅ ${filename} loaded and validated (${validation.nodeCount} nodes, ${validation.edgeCount} edges)`);
      
      // Parse to get available groupings
      const data = JSON.parse(jsonData);
      const groupings = data.hierarchyChoices || [];
      
      if (groupings.length === 0) {
        console.log(`⚠️  No groupings found in ${filename}, testing with flat structure`);
        const tester = new FuzzTester(data, filename);
        await tester.runTest();
      } else {
        console.log(`📊 Found ${groupings.length} groupings: ${groupings.map((g: any) => g.name).join(', ')}`);
        
        // Test each grouping
        for (const grouping of groupings) {
          const tester = new FuzzTester(data, filename);
          await tester.runTest(grouping.id);
        }
      }
      
      console.log(''); // Blank line between files
      
    } catch (error: unknown) {
      console.error(`❌ Error testing ${filename}:`, error instanceof Error ? error.message : String(error));
      throw error;
    }
  }
  
  console.log('🎉 All fuzz tests completed successfully!');
}

/**
 * Run a focused fuzz test on specific data
 */
export async function runFuzzTest(
  testData: any, 
  testName: string = 'Custom', 
  groupingId: string | null = null, 
  iterations: number = FUZZ_ITERATIONS
): Promise<void> {
  const tester = new FuzzTester(testData, testName);
  await tester.runTest(groupingId);
}

// Export components for use in other tests
export { FuzzTester, InvariantChecker };

// Run tests if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runFuzzTests().catch((error: unknown) => {
    console.error('Fuzz testing failed:', error instanceof Error ? error.message : String(error));
    process.exit(1);
  });
}
