/**
 * @fileoverview New Bridge-Based Layout Engine
 *
 * Complete replacement for alpha ELKLayoutEngine using our bridge architecture.
 * Maintains identical API while using the new VisualizationEngine internally.
 */
import { createVisualizationEngine } from '../core/VisualizationEngine';
import { createVisualizationState } from '../core/VisState';
export class ELKLayoutEngine {
    constructor(config = {}) {
        this.callbacks = new Map();
        this.lastStatistics = null;
        this.config = { ...DEFAULT_LAYOUT_CONFIG, ...config };
    }
    /**
     * Run layout - SAME API as alpha
     */
    async layout(nodes, edges, containers, config) {
        // ...existing code...
        const finalConfig = { ...this.config, ...config };
        const startTime = Date.now();
        try {
            // Create temporary VisState and load data
            const visState = createVisualizationState();
            // Load nodes
            nodes.forEach(node => {
                visState.setGraphNode(node.id, {
                    label: node.label,
                    hidden: node.hidden || false,
                    style: (node.style || 'default'),
                    ...node
                });
            });
            // Load edges
            edges.forEach(edge => {
                visState.setGraphEdge(edge.id, {
                    source: edge.source,
                    target: edge.target,
                    hidden: edge.hidden || false,
                    style: (edge.style || 'default')
                });
            });
            // Load containers
            containers.forEach(container => {
                visState.setContainer(container.id, {
                    collapsed: container.collapsed || false,
                    hidden: container.hidden || false,
                    children: Array.from(container.children || new Set()),
                    style: container.style || 'default'
                });
            });
            // Create engine and run layout
            const engine = createVisualizationEngine(visState, {
                autoLayout: false,
                enableLogging: false
            });
            // Emit start event
            this.emit('layout', { type: 'start' });
            // Use our bridge-based engine
            await engine.runLayout();
            const endTime = Date.now();
            const duration = endTime - startTime;
            // Convert results back to alpha format
            const result = {
                nodes: this.convertNodes(visState.visibleNodes),
                edges: this.convertEdges(visState.visibleEdges),
                containers: this.convertContainers(visState.visibleContainers)
            };
            // Update statistics
            this.lastStatistics = {
                totalNodes: result.nodes.length,
                totalEdges: result.edges.length,
                totalContainers: result.containers.length,
                layoutDuration: duration
            };
            // Emit completion event
            this.emit('layout', {
                type: 'complete',
                statistics: this.lastStatistics
            });
            // ...existing code...
            engine.dispose();
            return result;
        }
        catch (error) {
            const errorData = {
                type: 'error',
                error: error instanceof Error ? error : new Error(String(error))
            };
            this.emit('layout', errorData);
            throw error;
        }
    }
    /**
     * Layout with changed container - compatibility method
     */
    async layoutWithChangedContainer(nodes, edges, containers, config, changedContainerId, visualizationState) {
        // For now, just call regular layout - the bridge architecture handles changes efficiently
        return this.layout(nodes, edges, containers, config);
    }
    /**
     * Convert nodes to positioned format
     */
    convertNodes(nodes) {
        return nodes.map(node => ({
            ...node,
            x: node.x || 0,
            y: node.y || 0,
            width: node.width || 180,
            height: node.height || 60
        }));
    }
    /**
     * Convert edges to positioned format
     */
    convertEdges(edges) {
        return edges.map(edge => ({
            ...edge,
            points: [] // ELK doesn't provide edge routing in our simple implementation
        }));
    }
    /**
     * Convert containers to positioned format
     */
    convertContainers(containers) {
        return containers.map(container => ({
            ...container,
            x: container.x || 0,
            y: container.y || 0,
            width: container.width || 400,
            height: container.height || 300
        }));
    }
    /**
     * Emit event to listeners
     */
    emit(event, data) {
        const callback = this.callbacks.get(event);
        if (callback) {
            try {
                callback(data);
            }
            catch (error) {
                console.error(`[ELKLayoutEngine] Event callback error:`, error);
            }
        }
    }
    /**
     * Get last layout statistics
     */
    getLastLayoutStatistics() {
        return this.lastStatistics;
    }
    /**
     * Add event listener
     */
    on(event, callback) {
        this.callbacks.set(event, callback);
    }
    /**
     * Remove event listener
     */
    off(event, callback) {
        this.callbacks.delete(event);
    }
}
/**
 * Default layout configuration - MRTree as default for better hierarchical display
 */
export const DEFAULT_LAYOUT_CONFIG = {
    algorithm: 'mrtree',
    direction: 'DOWN',
    spacing: 100,
    nodeSize: { width: 180, height: 60 }
};
/**
 * Create ELK state manager - compatibility wrapper
 */
export function createELKStateManager() {
    // ...existing code...
    return {
        updatePositions: () => { },
        dispose: () => { }
    };
}
//# sourceMappingURL=ELKLayoutEngine.js.map