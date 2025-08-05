/**
 * Create styled node from raw node data
 */
export function createStyledNode(node: any, colorPalette?: string, hierarchyData?: any, nodeTypeConfig?: any): any;
/**
 * Create styled edge from raw edge data
 */
export function createStyledEdge(edge: any): {
    id: any;
    source: any;
    target: any;
    label: any;
    type: string;
    animated: boolean;
    style: {
        strokeWidth: number;
        stroke: string;
    };
    markerEnd: {
        type: string;
        width: number;
        height: number;
        color: string;
    };
};
/**
 * Get node color for MiniMap
 */
export function getMiniMapNodeColor(node: any, colorPalette?: string, nodeTypeConfig?: any): any;
/**
 * Process backtrace data into hierarchy structure
/**
 * Process hierarchy data and assign hierarchy paths to nodes
 */
export function processHierarchy(graphData: any, selectedGrouping?: string): any;
/**
 * Process graph data into styled nodes and edges
 */
export function processGraphData(graphData: any, colorPalette: any, currentLayout: any, applyLayout: any, currentGrouping?: string): Promise<{
    nodes: any;
    edges: any;
}>;
export namespace REACTFLOW_CONFIG {
    let fitView: boolean;
    let nodesDraggable: boolean;
    let nodesConnectable: boolean;
    let elementsSelectable: boolean;
    let maxZoom: number;
    let minZoom: number;
    let nodeOrigin: number[];
    let elevateEdgesOnSelect: boolean;
    let disableKeyboardA11y: boolean;
    let translateExtent: number[][];
}
export namespace DEFAULT_VIEWPORT {
    let x: number;
    let y: number;
    let zoom: number;
}
export namespace FIT_VIEW_CONFIG {
    export let padding: number;
    export let duration: number;
    let minZoom_1: number;
    export { minZoom_1 as minZoom };
    let maxZoom_1: number;
    export { maxZoom_1 as maxZoom };
}
export namespace MINIMAP_CONFIG {
    let nodeStrokeWidth: number;
    let nodeStrokeColor: string;
    let maskColor: string;
}
export namespace BACKGROUND_CONFIG {
    let color: string;
    let gap: number;
}
export namespace DEFAULT_EDGE_OPTIONS {
    let type: string;
    let animated: boolean;
    namespace style {
        let strokeWidth: number;
        let stroke: string;
    }
    namespace markerEnd {
        let type_1: string;
        export { type_1 as type };
        export let width: number;
        export let height: number;
        let color_1: string;
        export { color_1 as color };
    }
}
export namespace DEFAULT_NODE_STYLE {
    export let borderRadius: string;
    let padding_1: string;
    export { padding_1 as padding };
    let color_2: string;
    export { color_2 as color };
    export let fontSize: string;
    export let fontWeight: string;
    let width_1: number;
    export { width_1 as width };
    let height_1: number;
    export { height_1 as height };
    export let display: string;
    export let alignItems: string;
    export let justifyContent: string;
    export let textAlign: string;
}
//# sourceMappingURL=reactFlowConfig.d.ts.map