/**
 * Run a focused fuzz test on specific data
 */
export function runFuzzTest(testData: any, testName?: string, groupingId?: any, iterations?: number): Promise<void>;
/**
 * Fuzz test runner
 */
export class FuzzTester {
    constructor(testData: any, testName: any);
    testData: any;
    testName: any;
    random: SimpleRandom;
    /**
     * Run the fuzz test with the given grouping
     */
    runTest(groupingId?: any): Promise<void>;
    /**
     * Generate a random collapse or expand operation
     */
    generateRandomOperation(state: any): {
        type: string;
        containerId: any;
    };
    /**
     * Execute a collapse or expand operation
     */
    executeOperation(state: any, operation: any): void;
    /**
     * Capture a snapshot of the current state for debugging
     */
    captureStateSnapshot(state: any): {
        visibleNodes: any;
        visibleEdges: any;
        hyperEdges: any;
        expandedContainers: any;
        collapsedContainers: number;
    };
}
/**
 * System invariants that must always hold
 */
export class InvariantChecker {
    constructor(state: any);
    state: any;
    /**
     * Check all invariants and throw if any are violated
     */
    checkAll(context?: string): void;
    /**
     * Invariant: A node is visible iff it's not hidden and no parent container is collapsed
     */
    checkNodeVisibilityInvariant(context: any): void;
    /**
     * Invariant: An edge is visible iff both its endpoints are visible
     */
    checkEdgeVisibilityInvariant(context: any): void;
    /**
     * Invariant: Container hierarchy relationships are consistent
     */
    checkContainerHierarchyInvariant(context: any): void;
    /**
     * Invariant: HyperEdges exist only for visible, collapsed containers and connect to visible endpoints
     */
    checkHyperEdgeConsistency(context: any): void;
    /**
     * Check that hidden and expanded containers have no adjacent hyperEdges
     */
    _checkContainerHyperEdgeConstraints(context: any, hiddenContainerIds: any, visibleExpandedContainerIds: any): void;
    /**
     * Invariant: Visible collections contain exactly the items that should be visible
     */
    checkCollectionConsistency(context: any): void;
    /**
     * Invariant: nodeToEdges mapping is consistent with actual edges
     */
    checkNodeToEdgeMappingInvariant(context: any): void;
    /**
     * Helper: Check if a node is in a collapsed container
     */
    isNodeInCollapsedContainer(nodeId: any): any;
}
declare class SimpleRandom {
    constructor(seed: any);
    seed: any;
    next(): number;
    choice(array: any): any;
    boolean(probability?: number): boolean;
}
export {};
//# sourceMappingURL=fuzzTest.d.ts.map