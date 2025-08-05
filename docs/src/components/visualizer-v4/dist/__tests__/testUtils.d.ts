/**
 * @fileoverview Shared Test Utilities
 *
 * Common utilities for testing the visualization components
 */
import type { VisualizationState } from '../core/VisState';
export interface TestDataResult {
    rawData: any;
    state: VisualizationState;
    metadata: any;
}
/**
 * Load and parse the chat.json test data
 * @param grouping - Optional grouping to apply (e.g., 'location', 'filename')
 * @returns Parsed test data or null if file not found
 */
export declare function loadChatJsonTestData(grouping?: string | null): TestDataResult | null;
/**
 * Skip test with message if chat.json is not available
 */
export declare function skipIfNoTestData(testData: TestDataResult | null, testName?: string): boolean;
/**
 * Create a mock VisualizationState with container hierarchy for testing
 * Useful for unit tests that don't need the full chat.json data
 */
export declare function createMockVisStateWithContainers(): VisualizationState;
//# sourceMappingURL=testUtils.d.ts.map