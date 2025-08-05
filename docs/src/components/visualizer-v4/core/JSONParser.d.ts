/**
 * JSON Parser for Graph Data
 *
 * Framework-independent JSON parser that converts graph data into a VisualizationState.
 * Handles nodes, edges, hierarchies, and grouping assignments.
 */
import { VisualizationState } from './VisState';
import { NodeStyle, EdgeStyle } from '../shared/constants';
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
        nodeTypeConfig?: {
            defaultType?: string;
            types?: Array<{
                id: string;
                label: string;
                colorIndex: number;
            }>;
        };
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
    nodeTypeConfig?: {
        defaultType?: string;
        types?: Array<{
            id: string;
            label: string;
            colorIndex: number;
        }>;
    };
    metadata?: Record<string, any>;
}
/**
 * Parse graph JSON and populate a VisualizationState
 *
 * @param jsonData - Raw graph data or JSON string
 * @param grouping - Optional hierarchy grouping to apply
 * @returns Object containing the populated state and parsing metadata
 *
 * @example
 * ```javascript
 * const { state, metadata } = parseGraphJSON(graphData, 'myGrouping');
 * console.log('Parsed', metadata.nodeCount, 'nodes');
 * ```
 */
export declare function parseGraphJSON(jsonData: RawGraphData | string, selectedGrouping?: string): ParseResult;
/**
 * Create a reusable parser instance for processing multiple graph datasets.
 *
 * @param options - Parser configuration options
 * @returns Configured parser instance with parse method
 *
 * @example
 * ```javascript
 * const parser = createGraphParser({
 *   enableValidation: true,
 *   defaultStyle: 'highlighted'
 * });
 *
 * const result1 = parser.parse(data1);
 * const result2 = parser.parse(data2);
 * ```
 */
export declare function createGraphParser(options?: ParserOptions): {
    parse: (data: RawGraphData | string, grouping?: string) => ParseResult;
}; /**
 * Extract available hierarchical groupings from graph JSON data.
 *
 * @param jsonData - Raw graph data or JSON string
 * @returns Array of available grouping options
 *
 * @example
 * ```javascript
 * const groupings = getAvailableGroupings(graphData);
 * console.log('Available groupings:', groupings.map(g => g.name));
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
export declare function validateGraphJSON(jsonData: RawGraphData | string): ValidationResult;
export {};
//# sourceMappingURL=JSONParser.d.ts.map