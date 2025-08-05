/**
 * @fileoverview Visualization Service - Encapsulation Layer
 *
 * This service enforces that ALL layout and rend    const layoutResult = await this.layoutEngine.layoutWithChangedContainer(
      visibleNodes,
      visibleEdges,
      visibleContainers, // Send all visible containers for hyperedge positioning
      hyperEdges,
      layoutConfig,
      null, // null = full layout
      visState // Pass VisState for centralized state management
    );rations go through VisState.
 * It provides statically-typed methods that prevent bypassing the single source of truth.
 *
 * CRITICAL: No external code should call ELK or ReactFlow directly - everything must go through this service.
 */
import { ELKLayoutEngine } from '../layout/ELKLayoutEngine';
import { ReactFlowConverter } from '../render/ReactFlowConverter';
/**
 * Private symbols to prevent external instantiation
 * This ensures all operations go through the service methods
 */
const VISSTATE_TOKEN = Symbol('VisStateToken');
const LAYOUT_TOKEN = Symbol('LayoutToken');
/**
 * Encapsulated layout request - can only be created by VisualizationService
 */
export class EncapsulatedLayoutRequest {
    constructor(token, visState, layoutConfig, changedContainerId) {
        this.token = token;
        this.visState = visState;
        this.layoutConfig = layoutConfig;
        this.changedContainerId = changedContainerId;
        if (token !== LAYOUT_TOKEN) {
            throw new Error('EncapsulatedLayoutRequest can only be created by VisualizationService');
        }
    }
    /** Package-private access for the service */
    getVisState() { return this.visState; }
    getLayoutConfig() { return this.layoutConfig; }
    getChangedContainerId() { return this.changedContainerId; }
    /** Factory method - only callable by VisualizationService */
    static create(visState, layoutConfig, changedContainerId) {
        return new EncapsulatedLayoutRequest(LAYOUT_TOKEN, visState, layoutConfig, changedContainerId);
    }
}
/**
 * Encapsulated ReactFlow data - can only be created by VisualizationService
 */
export class EncapsulatedReactFlowData {
    constructor(token, data, sourceVisState) {
        this.token = token;
        this.data = data;
        this.sourceVisState = sourceVisState;
        if (token !== VISSTATE_TOKEN) {
            throw new Error('EncapsulatedReactFlowData can only be created by VisualizationService');
        }
    }
    /** Safe access to ReactFlow data - guaranteed to come from VisState */
    getReactFlowData() { return this.data; }
    getSourceVisState() { return this.sourceVisState; }
    /** Factory method - only callable by VisualizationService */
    static create(data, sourceVisState) {
        return new EncapsulatedReactFlowData(VISSTATE_TOKEN, data, sourceVisState);
    }
}
/**
 * Visualization Service - The ONLY way to interact with layout and rendering
 *
 * This service enforces the architectural principle:
 * VisState → Layout Engine → VisState → ReactFlow Converter → ReactFlow
 */
export class VisualizationService {
    constructor() {
        this.layoutEngine = new ELKLayoutEngine();
    }
    /**
     * Perform layout operation using ONLY VisState data
     *
     * @param visState - The single source of truth
     * @param layoutConfig - Layout configuration
     * @param changedContainerId - For selective layout (optional)
     * @returns Promise that resolves when layout is complete and applied back to VisState
     */
    async performLayout(visState, layoutConfig = {}) {
        console.log('[VisualizationService] 🎯 LAYOUT: Starting layout with VisState as source');
        // STEP 1: Extract data from VisState (single source of truth)
        const visibleNodes = visState.visibleNodes;
        const visibleEdges = visState.visibleEdges;
        const visibleContainers = visState.visibleContainers;
        console.log('[VisualizationService] 📊 VISSTATE_DATA:', {
            nodes: visibleNodes.length,
            edges: visibleEdges.length,
            visibleContainers: visibleContainers.length
        });
        // STEP 2: Run layout and automatically apply to VisState
        // Use layoutWithChangedContainer with null to get full layout that applies to VisState
        const layoutResult = await this.layoutEngine.layoutWithChangedContainer(visibleNodes, visibleEdges, // Include ALL visible edges (regular + hyperedge representations)
        visibleContainers, // Send all visible containers for positioning
        layoutConfig, null, // null = full layout
        visState // Pass VisState for automatic result application
        );
        console.log('[VisualizationService] ✅ LAYOUT: Layout complete, results applied to VisState');
        // Results are automatically applied to VisState by layoutWithChangedContainer
    }
    /**
     * Generate ReactFlow data from ONLY VisState
     *
     * @param visState - The single source of truth
     * @returns Encapsulated ReactFlow data that can only come from VisState
     */
    generateReactFlowData(visState) {
        console.log('[VisualizationService] 🔄 REACTFLOW: Generating ReactFlow data from VisState');
        // CRITICAL: Create layout result from CURRENT VisState, not from any cache
        const layoutResult = this.createLayoutResultFromVisState(visState);
        // Convert to ReactFlow format using the actual VisState
        const converter = new ReactFlowConverter();
        const reactFlowData = converter.convert(visState);
        console.log('[VisualizationService] 📊 REACTFLOW_DATA:', {
            nodes: reactFlowData.nodes.length,
            edges: reactFlowData.edges.length
        });
        // Return encapsulated data that proves it came from VisState
        return EncapsulatedReactFlowData.create(reactFlowData, visState);
    }
    /**
     * Create a layout result structure from current VisState
     * This ensures we always use the CURRENT state, not stale data
     */
    createLayoutResultFromVisState(visState) {
        // Get CURRENT data from VisState - this includes hidden nodes properly
        const visibleNodes = visState.visibleNodes;
        const visibleEdges = visState.visibleEdges;
        const visibleContainers = visState.visibleContainers;
        console.log('[VisualizationService] 📊 CURRENT_VISSTATE:', {
            visibleNodes: visibleNodes.length,
            visibleEdges: visibleEdges.length,
            visibleContainers: visibleContainers.length
        });
        return {
            nodes: visibleNodes.map(node => ({
                ...node,
                // Use position from VisState layout if available, otherwise default
                x: node.layout?.position?.x || 0,
                y: node.layout?.position?.y || 0,
                width: node.layout?.dimensions?.width || node.dimensions?.width || 180,
                height: node.layout?.dimensions?.height || node.dimensions?.height || 60
            })),
            edges: visibleEdges,
            containers: visibleContainers.map(container => ({
                ...container,
                // Use position from VisState layout if available, otherwise default
                x: container.x || 0,
                y: container.y || 0,
                width: container.width || 400,
                height: container.height || 300
            }))
        };
    }
    /**
     * Full workflow: Layout + ReactFlow data generation
     * This is the main method components should use
     */
    async layoutAndRender(visState, layoutConfig) {
        console.log(`[VisualizationService] 🚀 layoutAndRender: Starting full workflow`);
        // COMPREHENSIVE DEBUG: Log VisState before any processing
        this.logVisStateDetailed(visState, 'BEFORE_LAYOUT');
        // Step 1: Perform layout (results applied to VisState)
        await this.performLayout(visState, layoutConfig);
        // COMPREHENSIVE DEBUG: Log VisState after layout
        this.logVisStateDetailed(visState, 'AFTER_LAYOUT');
        // Step 2: Generate ReactFlow data from updated VisState
        const result = this.generateReactFlowData(visState);
        // COMPREHENSIVE DEBUG: Log final ReactFlow data
        this.logReactFlowData(result.getReactFlowData(), 'FINAL_REACTFLOW');
        // Return the nodes and edges from the encapsulated data
        const reactFlowData = result.getReactFlowData();
        return { nodes: reactFlowData.nodes, edges: reactFlowData.edges };
    }
    /**
     * Comprehensive VisState debugging - FOCUSED ON HYPEREDGES
     */
    logVisStateDetailed(visState, stage) {
        console.log(`[VisualizationService] 📊 VISSTATE_${stage}:`);
        // Get data using VisState's public API
        const visibleNodes = visState.visibleNodes;
        const visibleEdges = visState.visibleEdges;
        const visibleContainers = visState.visibleContainers;
        const expandedContainers = visState.visibleContainers.filter(c => !c.collapsed);
        console.log(`  📊 SUMMARY: ${visibleContainers.length} containers, ${visibleNodes.length} nodes, ${visibleEdges.length} edges`);
        console.log(`  📦 CONTAINERS:`);
        visibleContainers.forEach(container => {
            console.log(`    ${container.id}: collapsed=${container.collapsed}, hidden=${container.hidden}, children=${container.children?.size || 0}`);
        });
        console.log(`  🔘 NODES:`);
        visibleNodes.forEach(node => {
            console.log(`    ${node.id}: hidden=${node.hidden}`);
        });
    }
    logReactFlowData(data, stage) {
        console.log(`[VisualizationService] 🎯 REACTFLOW_${stage}:`);
        console.log(`  📊 SUMMARY: ${data.nodes.length} nodes, ${data.edges.length} edges`);
        // Only log detailed node info for nodes involved in hyperedges
        const hyperEdges = data.edges.filter(e => e.type === 'hyper');
        const hyperEdgeNodeIds = new Set();
        hyperEdges.forEach(edge => {
            hyperEdgeNodeIds.add(edge.source);
            hyperEdgeNodeIds.add(edge.target);
        });
        console.log(`  🔘 NODES involved in hyperedges:`);
        data.nodes.forEach(node => {
            if (hyperEdgeNodeIds.has(node.id)) {
                console.log(`    ${node.id} (${node.type}): pos=(${node.position?.x || 0}, ${node.position?.y || 0}), size=${node.width || 'auto'}x${node.height || 'auto'}`);
            }
        });
        console.log(`  🔥 HYPEREDGES (${hyperEdges.length} total):`);
        hyperEdges.forEach(edge => {
            console.log(`    ${edge.id}: ${edge.source} → ${edge.target}`);
            // Find source and target node positions in ReactFlow data
            const sourceNode = data.nodes.find(n => n.id === edge.source);
            const targetNode = data.nodes.find(n => n.id === edge.target);
            if (sourceNode && targetNode) {
                const sourcePosReactFlow = sourceNode.position || { x: 0, y: 0 };
                const targetPosReactFlow = targetNode.position || { x: 0, y: 0 };
                console.log(`      📍 REACTFLOW POSITIONS: ${edge.source}(${sourcePosReactFlow.x}, ${sourcePosReactFlow.y}) → ${edge.target}(${targetPosReactFlow.x}, ${targetPosReactFlow.y})`);
                // Calculate distance
                const dx = targetPosReactFlow.x - sourcePosReactFlow.x;
                const dy = targetPosReactFlow.y - sourcePosReactFlow.y;
                const distance = Math.sqrt(dx * dx + dy * dy);
                console.log(`      📏 DISTANCE: ${distance.toFixed(2)}px`);
                if (distance < 10) {
                    console.log(`      ⚠️  WARNING: Hyperedge endpoints are very close/overlapping!`);
                }
            }
            else {
                console.log(`      ❌ ERROR: Could not find ReactFlow nodes for hyperedge endpoints`);
                console.log(`        Source ${edge.source}: ${sourceNode ? 'FOUND' : 'NOT FOUND'}`);
                console.log(`        Target ${edge.target}: ${targetNode ? 'FOUND' : 'NOT FOUND'}`);
            }
        });
    }
}
/**
 * Singleton instance to prevent multiple layout engines
 */
let serviceInstance = null;
/**
 * Get the singleton VisualizationService instance
 * This ensures all components use the same service
 */
export function getVisualizationService() {
    if (!serviceInstance) {
        serviceInstance = new VisualizationService();
    }
    return serviceInstance;
}
/**
 * Type guards for runtime safety
 */
export function isEncapsulatedReactFlowData(obj) {
    return obj instanceof EncapsulatedReactFlowData;
}
export function isEncapsulatedLayoutRequest(obj) {
    return obj instanceof EncapsulatedLayoutRequest;
}
//# sourceMappingURL=VisualizationService.js.map