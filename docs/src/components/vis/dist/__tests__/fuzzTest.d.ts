/**
 * Fuzz Testing for VisualizationState (TypeScript Version)
 *
 * Performs randomized collapse/expand operations on parsed JSON data
 * and validates all system invariants throughout the process.
 */
import { VisualizationState } from '../core/VisState.js';
/**
 * System invariants that must always hold
 */
declare class InvariantChecker {
    private state;
    constructor(state: VisualizationState);
    /**
     * Check all invariants and throw if any are violated
     */
    checkAll(context?: string): void;
    /**
     * Invariant: A node is visible iff it's not hidden and no parent container is collapsed
     */
    private checkNodeVisibilityInvariant;
    /**
     * Invariant: An edge is visible iff both its endpoints are visible
     */
    private checkEdgeVisibilityInvariant;
    /**
     * Invariant: Container hierarchy relationships are consistent
     */
    private checkContainerHierarchyInvariant;
    /**
     * Invariant: HyperEdges exist only for visible, collapsed containers and connect to visible endpoints
     * NOTE: HyperEdges are now encapsulated within VisState - external code should not see them
     */
    private checkHyperEdgeConsistency;
    /**
     * Invariant: Visible collections contain exactly the items that should be visible
     */
    private checkCollectionConsistency;
}
/**
 * Fuzz test runner
 */
declare class FuzzTester {
    private testData;
    private testName;
    private random;
    constructor(testData: any, testName: string);
    /**
     * Run the fuzz test with the given grouping
     */
    runTest(groupingId?: string | null): Promise<void>;
    /**
     * Generate a random collapse or expand operation
     */
    private generateRandomOperation;
    /**
     * Execute a collapse or expand operation
     */
    private executeOperation;
    /**
     * Capture a snapshot of the current state for debugging
     */
    private captureStateSnapshot;
}
/**
 * Run a focused fuzz test on specific data
 */
export declare function runFuzzTest(testData: any, testName?: string, groupingId?: string | null, iterations?: number): Promise<void>;
export { FuzzTester, InvariantChecker };
//# sourceMappingURL=fuzzTest.d.ts.map