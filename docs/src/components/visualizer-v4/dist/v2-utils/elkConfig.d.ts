/**
 * Get the full ELK configuration for a specific layout type
 * @param {string} layoutType - The layout algorithm to use
 * @returns {Object} Complete ELK configuration object
 */
export function getELKConfig(layoutType?: string): any;
/**
 * Get ELK configuration for container nodes
 * @param {string} layoutType - The layout algorithm to use
 * @param {string} containerType - Type of container ('hierarchy', 'collapsed', 'root')
 * @returns {Object} Container-specific ELK configuration
 */
export function getContainerELKConfig(layoutType?: string, containerType?: string): any;
/**
 * Create ELK node sizing options for fixed positioning
 * @param {number} x - X position to fix
 * @param {number} y - Y position to fix
 * @returns {Object} ELK layout options for fixed positioning
 */
export function createFixedPositionOptions(x: number, y: number): any;
/**
 * Create ELK node sizing options for free positioning
 * @returns {Object} ELK layout options for free positioning
 */
export function createFreePositionOptions(): any;
/**
 * Get available layout options for UI controls
 * @returns {Object} Layout options formatted for UI consumption
 */
export function getLayoutOptions(): any;
export namespace ELK_LAYOUT_CONFIGS {
    let mrtree: {
        'elk.algorithm': string;
        'elk.direction': string;
        'elk.spacing.nodeNode': number;
        'elk.spacing.edgeNode': number;
    };
    let layered: {
        'elk.algorithm': string;
        'elk.direction': string;
        'elk.spacing.nodeNode': number;
        'elk.layered.spacing.nodeNodeBetweenLayers': number;
        'elk.layered.spacing.borderToNode': number;
    };
    let force: {
        'elk.algorithm': string;
        'elk.spacing.nodeNode': number;
    };
    let stress: {
        'elk.algorithm': string;
        'elk.spacing.nodeNode': number;
    };
    let radial: {
        'elk.algorithm': string;
        'elk.spacing.nodeNode': number;
    };
}
export namespace ELK_OPTIONS {
    namespace HIERARCHY_HANDLING {
        let INCLUDE_CHILDREN: string;
        let SEPARATE_CHILDREN: string;
    }
    namespace NODE_SIZE_CONSTRAINTS {
        let FREE: string;
        let FIXED_SIZE: string;
        let FIXED_POS: string;
        let MINIMUM_SIZE: string;
    }
    namespace DIRECTIONS {
        let UP: string;
        let DOWN: string;
        let LEFT: string;
        let RIGHT: string;
    }
    namespace SPACING {
        let EDGE_TO_NODE: number;
        let EDGE_TO_EDGE: number;
        let COMPONENT_TO_COMPONENT: number;
        let CONTAINER_PADDING: number;
        let ROOT_PADDING: number;
    }
}
export namespace ELK_CONTAINER_CONFIGS {
    let hierarchyContainer: {
        'elk.padding': string;
        'elk.spacing.nodeNode': number;
        'elk.spacing.edgeNode': number;
        'elk.spacing.edgeEdge': number;
    };
    let collapsedContainer: {
        'elk.spacing.nodeNode': number;
        'elk.spacing.componentComponent': number;
        'elk.partitioning.activate': string;
    };
    let rootLevel: {
        'elk.padding': string;
        'elk.hierarchyHandling': string;
        'elk.spacing.nodeNode': number;
        'elk.spacing.edgeNode': number;
        'elk.spacing.edgeEdge': number;
    };
}
export namespace LAYOUT_ALGORITHM_INFO {
    export namespace mrtree_1 {
        let name: string;
        let description: string;
        let bestFor: string[];
    }
    export { mrtree_1 as mrtree };
    export namespace layered_1 {
        let name_1: string;
        export { name_1 as name };
        let description_1: string;
        export { description_1 as description };
        let bestFor_1: string[];
        export { bestFor_1 as bestFor };
    }
    export { layered_1 as layered };
    export namespace force_1 {
        let name_2: string;
        export { name_2 as name };
        let description_2: string;
        export { description_2 as description };
        let bestFor_2: string[];
        export { bestFor_2 as bestFor };
    }
    export { force_1 as force };
    export namespace stress_1 {
        let name_3: string;
        export { name_3 as name };
        let description_3: string;
        export { description_3 as description };
        let bestFor_3: string[];
        export { bestFor_3 as bestFor };
    }
    export { stress_1 as stress };
    export namespace radial_1 {
        let name_4: string;
        export { name_4 as name };
        let description_4: string;
        export { description_4 as description };
        let bestFor_4: string[];
        export { bestFor_4 as bestFor };
    }
    export { radial_1 as radial };
}
//# sourceMappingURL=elkConfig.d.ts.map