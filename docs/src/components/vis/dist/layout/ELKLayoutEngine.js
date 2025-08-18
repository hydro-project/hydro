/**
 * @fileoverview ELK Layout Engine (Enhanced with working patterns)
 *
 * ELK-based automatic layout engine using proven patterns from the working visualizer.
 * Handles hierarchical layouts with proper container dimension management.
 */
import { createELKStateManager } from './ELKStateManager.js';
import { ELK_ALGORITHMS } from '../shared/config.js';
// ============ Constants ============
const ENGINE_CONSTANTS = {
    DEFAULT_ALGORITHM: ELK_ALGORITHMS.LAYERED,
    DEFAULT_CONTAINER_WIDTH: 400,
    DEFAULT_CONTAINER_HEIGHT: 300,
    DEFAULT_NODE_WIDTH: 180,
    DEFAULT_NODE_HEIGHT: 60,
};
const LOG_PREFIXES = {
    ENGINE: '[ELKLayoutEngine]',
    CACHING: '💾 CACHING:',
    SUCCESS: '✅',
    ERROR: '❌',
};
// ============ Dimension Cache Management ============
/**
 * Encapsulated cache for container dimensions with type safety
 */
class ContainerDimensionCache {
    constructor() {
        this.cache = new Map();
    }
    set(containerId, dimensions) {
        this.cache.set(containerId, { ...dimensions });
        console.log(`${LOG_PREFIXES.ENGINE} ${LOG_PREFIXES.CACHING} ${containerId} → ${dimensions.width}x${dimensions.height}`);
    }
    get(containerId) {
        const cached = this.cache.get(containerId);
        return cached ? { ...cached } : undefined;
    }
    clear() {
        this.cache.clear();
    }
    size() {
        return this.cache.size;
    }
}
// ============ Layout Result Converter ============
/**
 * Converts ELK state manager results to LayoutResult format
 */
class LayoutResultConverter {
    convert(elkResult, originalNodes, originalEdges, originalContainers, originalHyperEdges, dimensionCache) {
        return {
            nodes: this.convertNodes(elkResult.nodes, originalNodes),
            edges: this.convertEdges(originalEdges),
            containers: this.convertContainers(elkResult.nodes, originalContainers, dimensionCache),
            hyperEdges: this.convertHyperEdges(originalHyperEdges)
        };
    }
    convertNodes(elkNodes, originalNodes) {
        return elkNodes
            .filter(node => originalNodes.find(n => n.id === node.id)) // Only include actual nodes
            .map(node => {
            const originalNode = originalNodes.find(n => n.id === node.id);
            return {
                ...originalNode,
                x: node.position?.x || 0,
                y: node.position?.y || 0,
                width: node.width || node.dimensions?.width || ENGINE_CONSTANTS.DEFAULT_NODE_WIDTH,
                height: node.height || node.dimensions?.height || ENGINE_CONSTANTS.DEFAULT_NODE_HEIGHT
            };
        });
    }
    convertEdges(originalEdges) {
        return originalEdges.map(edge => ({
            ...edge,
            points: [] // ELK routing will be added later if needed
        }));
    }
    convertContainers(elkNodes, originalContainers, dimensionCache) {
        return originalContainers.map(container => {
            const layoutedNode = elkNodes.find(n => n.id === container.id);
            const cachedDimensions = dimensionCache.get(container.id);
            // Priority: ELK-calculated > cached > original > fallback
            const width = this.getDimension(layoutedNode?.width, layoutedNode?.dimensions?.width, cachedDimensions?.width, container.expandedDimensions?.width, ENGINE_CONSTANTS.DEFAULT_CONTAINER_WIDTH);
            const height = this.getDimension(layoutedNode?.height, layoutedNode?.dimensions?.height, cachedDimensions?.height, container.expandedDimensions?.height, ENGINE_CONSTANTS.DEFAULT_CONTAINER_HEIGHT);
            return {
                ...container,
                x: layoutedNode?.position?.x || 0,
                y: layoutedNode?.position?.y || 0,
                width,
                height
            };
        });
    }
    convertHyperEdges(originalHyperEdges) {
        return originalHyperEdges.map(hyperEdge => ({
            ...hyperEdge,
            points: []
        }));
    }
    getDimension(...values) {
        return values.find(v => v !== undefined) || ENGINE_CONSTANTS.DEFAULT_CONTAINER_WIDTH;
    }
}
// ============ ELK Layout Engine Implementation ============
export class ELKLayoutEngine {
    constructor() {
        this.dimensionCache = new ContainerDimensionCache();
        this.resultConverter = new LayoutResultConverter();
        this.elkStateManager = createELKStateManager();
    }
    async layout(nodes, edges, containers, hyperEdges, config = {}) {
        try {
            console.log(`${LOG_PREFIXES.ENGINE} Starting layout with proven approach...`);
            const algorithm = this.getLayoutAlgorithm(config.algorithm);
            // Use the proven ELK state manager approach
            const result = await this.elkStateManager.calculateFullLayout(nodes, edges, containers, algorithm);
            // Cache container dimensions for future use - use ELK's calculated dimensions
            this.cacheContainerDimensions(result.nodes, containers);
            // Convert to our LayoutResult format
            const layoutResult = this.resultConverter.convert(result, nodes, edges, containers, hyperEdges, this.dimensionCache);
            console.log(`${LOG_PREFIXES.ENGINE} Layout completed successfully`);
            return layoutResult;
        }
        catch (error) {
            console.error(`${LOG_PREFIXES.ENGINE} Layout failed:`, error);
            throw error;
        }
    }
    /**
     * Get cached container dimensions
     */
    getCachedDimensions(containerId) {
        return this.dimensionCache.get(containerId);
    }
    /**
     * Clear the dimensions cache
     */
    clearCache() {
        this.dimensionCache.clear();
    }
    /**
     * Get cache statistics
     */
    getCacheStats() {
        return { size: this.dimensionCache.size() };
    }
    getLayoutAlgorithm(algorithm) {
        if (algorithm && Object.values(ELK_ALGORITHMS).includes(algorithm)) {
            return algorithm;
        }
        return ENGINE_CONSTANTS.DEFAULT_ALGORITHM;
    }
    cacheContainerDimensions(elkNodes, containers) {
        elkNodes.forEach(node => {
            // Check if this node is actually a container
            const correspondingContainer = containers.find(c => c.id === node.id);
            if (correspondingContainer) {
                const dimensions = {
                    width: node.width || node.dimensions?.width || ENGINE_CONSTANTS.DEFAULT_CONTAINER_WIDTH,
                    height: node.height || node.dimensions?.height || ENGINE_CONSTANTS.DEFAULT_CONTAINER_HEIGHT
                };
                this.dimensionCache.set(node.id, dimensions);
            }
        });
    }
}
//# sourceMappingURL=ELKLayoutEngine.js.map