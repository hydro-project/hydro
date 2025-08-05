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
export function parseGraphJSON(jsonData: any, selectedGrouping: any): {
    state: import("./VisState").VisualizationState;
    metadata: {
        nodeCount: any;
        edgeCount: any;
        selectedGrouping: any;
        containerCount: number;
        availableGroupings: any;
        nodeTypeConfig: any;
    };
};
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
export function createGraphParser(options?: {}): {
    parse: (data: any, grouping: any) => {
        state: import("./VisState").VisualizationState;
        metadata: {
            nodeCount: any;
            edgeCount: any;
            selectedGrouping: any;
            containerCount: number;
            availableGroupings: any;
            nodeTypeConfig: any;
        };
    };
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
export function getAvailableGroupings(jsonData: any): any;
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
export function validateGraphJSON(jsonData: any): {
    isValid: boolean;
    errors: string[];
    warnings: any[];
    nodeCount: number;
    edgeCount: number;
    hierarchyCount: number;
} | {
    isValid: boolean;
    errors: string[];
    warnings: string[];
    nodeCount: any;
    edgeCount: any;
    hierarchyCount: number;
};
//# sourceMappingURL=JSONParser.d.ts.map