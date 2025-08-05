/**
 * @fileoverview Simple JSON Data Loader
 *
 * Loads graph data from JSON and converts it to VisState format
 * Minimal implementation for demonstration purposes
 */
import type { VisualizationState } from '../core/VisState';
export interface SimpleGraphData {
    nodes: Array<{
        id: string;
        label?: string;
        style?: string;
    }>;
    edges: Array<{
        id: string;
        source: string;
        target: string;
        style?: string;
    }>;
    containers?: Array<{
        id: string;
        children: string[];
        collapsed?: boolean;
        style?: string;
    }>;
}
/**
 * Convert simple JSON graph data to VisState
 */
export declare function loadGraphFromJSON(jsonData: SimpleGraphData, visState: VisualizationState): void;
/**
 * Sample data for testing
 */
export declare const SAMPLE_GRAPH_DATA: SimpleGraphData;
/**
 * Sample data with collapsed container (to test hyperedge fix)
 */
export declare const SAMPLE_COLLAPSED_GRAPH: SimpleGraphData;
//# sourceMappingURL=JSONLoader.d.ts.map