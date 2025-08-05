/**
 * Symmetric Inverse Tests (TypeScript Version)
 *
 * Tests that verify all symmetric function pairs are true inverses of each other.
 * These tests ensure that applying a function followed by its inverse returns
 * the system to exactly the original state.
 */
/**
 * Test that container collapse and expand are true inverses
 */
declare function testCollapseExpandInverse(): void;
/**
 * Test multiple collapse/expand cycles
 */
declare function testMultipleCollapseExpandCycles(): void;
/**
 * Test nested container collapse/expand inverse
 */
declare function testNestedContainerInverse(): void;
/**
 * Test that hide/show operations are inverses
 */
declare function testHideShowInverse(): void;
/**
 * Test clear and rebuild operations
 */
declare function testClearRebuildInverse(): void;
declare function runAllTests(): Promise<void>;
export { testCollapseExpandInverse, testMultipleCollapseExpandCycles, testNestedContainerInverse, testHideShowInverse, testClearRebuildInverse, runAllTests };
//# sourceMappingURL=symmetricInverse.test.d.ts.map