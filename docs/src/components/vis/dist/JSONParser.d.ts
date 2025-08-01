/**
 * JSON Parser for Hydro Graph Data
 *
 * Converts the old visualizer's JSON format into the new VisualizationState format.
 * Handles nodes, edges, hierarchies, and grouping assignments.
 */
import { VisualizationState } from './VisState.js';
import { NodeStyle, EdgeStyle } from './constants.js';
export interface GroupingOption {
    id: string;
    name: string;
}
export interface ParseResult {
    state: VisualizationState;
    metadata: {
        selectedGrouping: string | null;
        nodeCount: number;
        edgeCount: number;
        containerCount: number;
        availableGroupings: GroupingOption[];
    };
}
export interface ValidationResult {
    isValid: boolean;
    errors: string[];
    warnings: string[];
    nodeCount: number;
    edgeCount: number;
    hierarchyCount: number;
}
export interface ParserOptions {
    validateData?: boolean;
    strictMode?: boolean;
    defaultNodeStyle?: NodeStyle;
    defaultEdgeStyle?: EdgeStyle;
}
interface RawNode {
    id: string;
    label?: string;
    style?: string;
    hidden?: boolean;
    [key: string]: any;
}
interface RawEdge {
    id: string;
    source: string;
    target: string;
    style?: string;
    hidden?: boolean;
    [key: string]: any;
}
interface RawHierarchy {
    id: string;
    name: string;
    groups: Record<string, string[]>;
}
interface RawHierarchyChoice {
    id: string;
    name: string;
    hierarchy: RawHierarchyItem[];
}
interface RawHierarchyItem {
    id: string;
    name: string;
    children?: RawHierarchyItem[];
}
interface RawGraphData {
    nodes: RawNode[];
    edges: RawEdge[];
    hierarchies?: RawHierarchy[];
    hierarchyChoices?: RawHierarchyChoice[];
    nodeAssignments?: Record<string, Record<string, string>>;
    metadata?: Record<string, any>;
}
/**
 * Parse Hydro graph JSON and populate a VisualizationState
 *
 * @param jsonData - The JSON data (object or JSON string)
 * @param selectedGrouping - Which hierarchy grouping to use (defaults to first available)
 * @returns Object containing the populated state and metadata
 * @throws {Error} When JSON data is invalid or malformed
 * @example
 * ```typescript
 * const { state, metadata } = parseHydroGraphJSON(hydroData, 'myGrouping');
 * console.log(`Parsed ${state.getVisibleNodes().length} nodes`);
 * console.log(`Used grouping: ${metadata.selectedGrouping}`);
 * ```
 */
export declare function parseHydroGraphJSON(jsonData: RawGraphData | string, selectedGrouping?: string | null): ParseResult;
/**
 * Create a reusable parser instance for processing multiple Hydro graph datasets.
 * Useful when parsing multiple graphs with similar structure/settings.
 *
 * @param options - Parser configuration options
 * @returns Parser function that accepts JSON data
 * @example
 * ```typescript
 * const parser = createHydroGraphParser({
 *   validateData: true,
 *   defaultNodeStyle: NODE_STYLES.HIGHLIGHTED
 * });
 *
 * const result1 = parser(graphData1);
 * const result2 = parser(graphData2);
 * ```
 */
export declare function createHydroGraphParser(options?: ParserOptions): {
    parse: (data: RawGraphData | string, grouping?: string) => ParseResult;
}; /**
 * Extract available hierarchical groupings from Hydro graph JSON data.
 * Useful for presenting grouping options to users before parsing.
 *
 * @param jsonData - The JSON data (object or JSON string)
 * @returns Array of available grouping objects
 * @example
 * ```typescript
 * const groupings = getAvailableGroupings(hydroData);
 * groupings.forEach(g => console.log(`${g.name} (${g.id})`));
 * ```
 */
export declare function getAvailableGroupings(jsonData: RawGraphData | string): GroupingOption[];
/**
 * Validate Hydro graph JSON data structure and content.
 * Provides detailed validation results including errors and warnings.
 *
 * @param jsonData - The JSON data (object or JSON string)
 * @returns Validation result object
 * @example
 * ```typescript
 * const validation = validateHydroGraphJSON(suspiciousData);
 * if (!validation.isValid) {
 *   console.error('Validation failed:', validation.errors);
 *   return;
 * }
 * if (validation.warnings.length > 0) {
 *   console.warn('Warnings found:', validation.warnings);
 * }
 * ```
 */
export declare function validateHydroGraphJSON(jsonData: RawGraphData | string): ValidationResult;
export {};
//# sourceMappingURL=JSONParser.d.ts.map