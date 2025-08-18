/**
 * ELK State Manager (TypeScript port from working visualizer)
 *
 * This module provides wrapper functions that ensure all ELK layout interactions
 * are consistent with visualization state management as the single source of truth.
 *
 * Key principle: ELK should only ever calculate layouts based on the exact
 * visual state requirements, and return results that perfectly match those requirements.
 */
import ELK from 'elkjs';
import { ELK_ALGORITHMS, LAYOUT_SPACING } from '../shared/config.js';
// ============ Constants ============
const VALIDATION_CONSTANTS = {
    DEFAULT_NODE_WIDTH: 180,
    DEFAULT_NODE_HEIGHT: 60,
    DEFAULT_CONTAINER_WIDTH: 400,
    DEFAULT_CONTAINER_HEIGHT: 300,
    COORDINATE_ORIGIN: 0,
};
const LOG_PREFIXES = {
    STATE_MANAGER: '[ELKStateManager]',
    FULL_LAYOUT: '🏗️ FULL_LAYOUT:',
    VISUAL_LAYOUT: '🎯 VISUAL_LAYOUT:',
    VALIDATION: '🔍',
    CACHING: '💾 CACHING:',
    SUMMARY: '📊 SUMMARY:',
    CONTAINER: '📦',
    INPUT: 'INPUT',
    OUTPUT: 'OUTPUT',
    SUCCESS: '✅',
    WARNING: '⚠️',
    ERROR: '❌',
};
// ============ Layout Cache Management ============
/**
 * Encapsulated dimension cache with consistent interface
 */
class DimensionCache {
    constructor() {
        this.cache = new Map();
    }
    set(id, dimensions) {
        this.cache.set(id, { ...dimensions });
    }
    get(id) {
        const cached = this.cache.get(id);
        return cached ? { ...cached } : undefined;
    }
    has(id) {
        return this.cache.has(id);
    }
    clear() {
        this.cache.clear();
    }
    size() {
        return this.cache.size;
    }
    keys() {
        return this.cache.keys();
    }
}
// ============ Validation Utilities ============
/**
 * Encapsulated validation with proper error collection
 */
class ContainmentValidator {
    constructor() {
        this.violations = [];
    }
    /**
     * Validate that nodes fit within their parent containers
     */
    validateContainment(layoutedNodes, containers) {
        this.violations = [];
        this.logValidationStart();
        containers.forEach(container => {
            this.validateSingleContainer(container, layoutedNodes);
        });
        this.logValidationResults();
        return {
            isValid: this.violations.length === 0,
            violations: [...this.violations]
        };
    }
    validateSingleContainer(container, layoutedNodes) {
        const containerNode = this.findContainerNode(container.id, layoutedNodes);
        if (!containerNode) {
            this.logWarning(`Container ${container.id} not found in layout result`);
            return;
        }
        const childNodes = this.findChildNodes(container, layoutedNodes);
        this.logContainerValidation(container, containerNode, childNodes);
        childNodes.forEach(childNode => {
            this.validateChildContainment(childNode, container.id, containerNode);
        });
    }
    findContainerNode(containerId, layoutedNodes) {
        return layoutedNodes.find(n => n.id === containerId) || null;
    }
    findChildNodes(container, layoutedNodes) {
        return layoutedNodes.filter(node => container.children.has(node.id));
    }
    validateChildContainment(childNode, containerId, containerNode) {
        const childBounds = this.calculateNodeBounds(childNode);
        const containerBounds = this.calculateContainerBounds(containerNode);
        const fitsHorizontally = childBounds.x >= VALIDATION_CONSTANTS.COORDINATE_ORIGIN &&
            childBounds.right <= containerBounds.width;
        const fitsVertically = childBounds.y >= VALIDATION_CONSTANTS.COORDINATE_ORIGIN &&
            childBounds.bottom <= containerBounds.height;
        if (!fitsHorizontally || !fitsVertically) {
            this.addViolation(childNode.id, containerId, childBounds, containerBounds, fitsHorizontally, fitsVertically);
        }
        else {
            this.logSuccess(`Node ${childNode.id} fits in container ${containerId}`);
        }
    }
    calculateNodeBounds(node) {
        const x = node.position?.x || VALIDATION_CONSTANTS.COORDINATE_ORIGIN;
        const y = node.position?.y || VALIDATION_CONSTANTS.COORDINATE_ORIGIN;
        const width = node.width || VALIDATION_CONSTANTS.DEFAULT_NODE_WIDTH;
        const height = node.height || VALIDATION_CONSTANTS.DEFAULT_NODE_HEIGHT;
        return {
            x,
            y,
            width,
            height,
            right: x + width,
            bottom: y + height
        };
    }
    calculateContainerBounds(containerNode) {
        const x = VALIDATION_CONSTANTS.COORDINATE_ORIGIN; // Container coordinates are relative
        const y = VALIDATION_CONSTANTS.COORDINATE_ORIGIN;
        const width = containerNode.width || VALIDATION_CONSTANTS.DEFAULT_CONTAINER_WIDTH;
        const height = containerNode.height || VALIDATION_CONSTANTS.DEFAULT_CONTAINER_HEIGHT;
        return {
            x,
            y,
            width,
            height,
            right: x + width,
            bottom: y + height
        };
    }
    addViolation(childId, containerId, childBounds, containerBounds, fitsHorizontally, fitsVertically) {
        const issue = `Does not fit ${!fitsHorizontally ? 'horizontally' : ''} ${!fitsVertically ? 'vertically' : ''}`.trim();
        this.violations.push({
            childId,
            containerId,
            issue,
            childBounds,
            containerBounds
        });
        this.logError(`CONTAINMENT VIOLATION: Node ${childId} does not fit in container ${containerId}`);
        this.logError(`  Child (relative): (${childBounds.x}, ${childBounds.y}) ${childBounds.width}x${childBounds.height} -> (${childBounds.right}, ${childBounds.bottom})`);
        this.logError(`  Container bounds: (${containerBounds.x}, ${containerBounds.y}) ${containerBounds.width}x${containerBounds.height} -> (${containerBounds.right}, ${containerBounds.bottom})`);
        this.logError(`  Fits horizontally: ${fitsHorizontally}, Fits vertically: ${fitsVertically}`);
    }
    logValidationStart() {
        console.log(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.VALIDATION} Checking containment relationships...`);
    }
    logValidationResults() {
        if (this.violations.length > 0) {
            console.error(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.ERROR} Found ${this.violations.length} containment violations!`);
        }
        else {
            console.log(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.SUCCESS} All containment relationships are valid`);
        }
    }
    logContainerValidation(container, containerNode, childNodes) {
        console.log(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.CONTAINER} Validating container ${container.id}:`);
        console.log(`  Container bounds: (${containerNode.position?.x || 0}, ${containerNode.position?.y || 0}) ${containerNode.width}x${containerNode.height}`);
        console.log(`  Child nodes: ${childNodes.length}`);
    }
    logWarning(message) {
        console.warn(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.WARNING} ${message}`);
    }
    logError(message) {
        console.error(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.ERROR} ${message}`);
    }
    logSuccess(message) {
        console.log(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.SUCCESS} ${message}`);
    }
}
// ============ ELK Configuration Utilities ============
/**
 * Encapsulated ELK configuration management
 */
class ELKConfigurationManager {
    /**
     * Get ELK configuration for different contexts
     */
    getConfig(layoutType, context = 'root') {
        const baseConfig = this.getBaseConfig(layoutType);
        const contextConfig = this.getContextConfig(context);
        return { ...baseConfig, ...contextConfig };
    }
    getBaseConfig(layoutType) {
        const algorithm = ELK_ALGORITHMS[layoutType] || ELK_ALGORITHMS.LAYERED;
        return {
            'elk.algorithm': algorithm,
            'elk.direction': 'DOWN',
            'elk.spacing.nodeNode': LAYOUT_SPACING.NODE_TO_NODE_NORMAL.toString(),
            'elk.spacing.edgeEdge': LAYOUT_SPACING.EDGE_TO_EDGE.toString(),
            'elk.spacing.edgeNode': LAYOUT_SPACING.EDGE_TO_NODE.toString(),
            'elk.spacing.componentComponent': LAYOUT_SPACING.COMPONENT_TO_COMPONENT.toString(),
        };
    }
    getContextConfig(context) {
        const padding = context === 'root'
            ? LAYOUT_SPACING.ROOT_PADDING
            : LAYOUT_SPACING.CONTAINER_PADDING;
        return {
            'elk.padding.left': padding.toString(),
            'elk.padding.right': padding.toString(),
            'elk.padding.top': padding.toString(),
            'elk.padding.bottom': padding.toString(),
        };
    }
}
// ============ ELK Hierarchy Builder ============
/**
 * Builds ELK graph hierarchy with proper type safety
 */
class ELKHierarchyBuilder {
    constructor(nodes, containers, edges, configManager) {
        this.nodes = nodes;
        this.containers = containers;
        this.edges = edges;
        this.configManager = configManager;
    }
    buildElkGraph(layoutType) {
        return {
            id: 'full_layout_root',
            layoutOptions: this.configManager.getConfig(layoutType, 'root'),
            children: this.buildHierarchy(null, layoutType),
            edges: this.buildEdges(),
        };
    }
    buildHierarchy(parentId, layoutType) {
        const children = [];
        // Add containers at this level
        const levelContainers = this.findContainersAtLevel(parentId);
        levelContainers.forEach(container => {
            children.push(this.buildContainerNode(container, layoutType));
        });
        // Add regular nodes at this level
        const levelNodes = this.findNodesAtLevel(parentId);
        levelNodes.forEach(node => {
            children.push(this.buildRegularNode(node));
        });
        return children;
    }
    findContainersAtLevel(parentId) {
        return this.containers.filter(container => {
            if (parentId === null) {
                // Root level - containers not contained by any other container
                return !this.containers.some(otherContainer => otherContainer.children.has(container.id));
            }
            else {
                // Non-root level - containers contained by the parent
                const parentContainer = this.containers.find(c => c.id === parentId);
                return parentContainer?.children.has(container.id) || false;
            }
        });
    }
    findNodesAtLevel(parentId) {
        const regularNodes = this.nodes.filter(node => node.type !== 'container');
        return regularNodes.filter(node => {
            if (parentId === null) {
                // Root level - nodes not contained by any container
                return !this.containers.some(container => container.children.has(node.id));
            }
            else {
                // Non-root level - nodes contained by the parent
                const parentContainer = this.containers.find(c => c.id === parentId);
                return parentContainer?.children.has(node.id) || false;
            }
        });
    }
    buildContainerNode(container, layoutType) {
        return {
            id: container.id,
            layoutOptions: this.configManager.getConfig(layoutType, 'container'),
            children: this.buildHierarchy(container.id, layoutType),
            // Let ELK calculate container size for dimension caching - DON'T specify width/height
        };
    }
    buildRegularNode(node) {
        const width = node.dimensions?.width || VALIDATION_CONSTANTS.DEFAULT_NODE_WIDTH;
        const height = node.dimensions?.height || VALIDATION_CONSTANTS.DEFAULT_NODE_HEIGHT;
        return {
            id: node.id,
            width,
            height,
        };
    }
    buildEdges() {
        return this.edges.map(edge => ({
            id: edge.id,
            sources: [edge.source],
            targets: [edge.target],
        }));
    }
}
// ============ Position Application Utilities ============
/**
 * Applies ELK layout results back to nodes with proper type safety
 */
class PositionApplicator {
    applyPositions(elkNodes, originalNodes, containers) {
        return this.processElkNodes(elkNodes, originalNodes, containers, 0);
    }
    processElkNodes(elkNodes, originalNodes, containers, depth) {
        const layoutedNodes = [];
        elkNodes.forEach(elkNode => {
            const processedNode = this.processElkNode(elkNode, originalNodes, containers);
            if (processedNode) {
                layoutedNodes.push(processedNode);
            }
            // Recursively process children
            if (elkNode.children) {
                layoutedNodes.push(...this.processElkNodes(elkNode.children, originalNodes, containers, depth + 1));
            }
        });
        return layoutedNodes;
    }
    processElkNode(elkNode, originalNodes, containers) {
        const originalNode = originalNodes.find(n => n.id === elkNode.id);
        const originalContainer = containers.find(c => c.id === elkNode.id);
        const original = originalNode || originalContainer;
        if (!original) {
            return null;
        }
        return {
            ...original,
            width: elkNode.width || VALIDATION_CONSTANTS.DEFAULT_NODE_WIDTH,
            height: elkNode.height || VALIDATION_CONSTANTS.DEFAULT_NODE_HEIGHT,
            position: {
                x: elkNode.x || VALIDATION_CONSTANTS.COORDINATE_ORIGIN,
                y: elkNode.y || VALIDATION_CONSTANTS.COORDINATE_ORIGIN,
            },
            dimensions: {
                width: elkNode.width || VALIDATION_CONSTANTS.DEFAULT_NODE_WIDTH,
                height: elkNode.height || VALIDATION_CONSTANTS.DEFAULT_NODE_HEIGHT,
            },
        };
    }
}
// ============ Node Sorting Utilities ============
/**
 * Sorts nodes to ensure parents come before children (ReactFlow requirement)
 */
class NodeSorter {
    sortNodesForReactFlow(layoutedNodes, containers) {
        const sortedNodes = [];
        const nodeMap = new Map(layoutedNodes.map(node => [node.id, node]));
        const visited = new Set();
        layoutedNodes.forEach(node => this.addNodeAndParents(node.id, nodeMap, containers, visited, sortedNodes));
        return sortedNodes;
    }
    addNodeAndParents(nodeId, nodeMap, containers, visited, sortedNodes) {
        if (visited.has(nodeId))
            return;
        const node = nodeMap.get(nodeId);
        if (!node)
            return;
        // Find parent container
        const parentContainer = containers.find(container => container.children.has(nodeId));
        if (parentContainer && !visited.has(parentContainer.id)) {
            this.addNodeAndParents(parentContainer.id, nodeMap, containers, visited, sortedNodes);
        }
        visited.add(nodeId);
        sortedNodes.push(node);
    }
}
/**
 * Create an ELK state manager that wraps all ELK layout interactions
 * with proper state management as the single source of truth.
 */
export function createELKStateManager() {
    const elk = new ELK();
    const validator = new ContainmentValidator();
    const configManager = new ELKConfigurationManager();
    const positionApplicator = new PositionApplicator();
    const nodeSorter = new NodeSorter();
    /**
     * Calculate full layout for dimension caching (expanded state).
     * This is used to populate the dimension cache with expanded container sizes.
     */
    async function calculateFullLayout(nodes, edges, containers, layoutType = ELK_ALGORITHMS.LAYERED) {
        console.log(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.FULL_LAYOUT} Calculating expanded layout for dimension caching`);
        logLayoutSummary(nodes, edges, containers);
        try {
            const hierarchyBuilder = new ELKHierarchyBuilder(nodes, containers, edges, configManager);
            const elkGraph = hierarchyBuilder.buildElkGraph(layoutType);
            logELKInput(elkGraph);
            const layoutResult = await elk.layout(elkGraph);
            logELKOutput(layoutResult);
            // Apply positions back to nodes
            const layoutedNodes = positionApplicator.applyPositions(layoutResult.children || [], nodes, containers);
            // Validate containment relationships
            console.log(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.VALIDATION} CONTAINMENT VALIDATION:`);
            const validationResult = validator.validateContainment(layoutedNodes, containers);
            if (!validationResult.isValid) {
                console.warn(`${LOG_PREFIXES.STATE_MANAGER} Layout validation found issues, but proceeding with layout.`);
            }
            // Sort nodes so parents come before children (ReactFlow requirement)
            const sortedNodes = nodeSorter.sortNodesForReactFlow(layoutedNodes, containers);
            return {
                nodes: sortedNodes,
                edges: edges,
            };
        }
        catch (error) {
            console.error(`${LOG_PREFIXES.STATE_MANAGER} Full layout failed:`, error);
            throw error;
        }
    }
    /**
     * Calculate layout based on current visualization state.
     * This handles visible/hidden containers and collapsed states.
     */
    async function calculateVisualLayout(nodes, edges, containers, hyperEdges, layoutType = ELK_ALGORITHMS.LAYERED, dimensionsCache) {
        console.log(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.VISUAL_LAYOUT} Calculating layout for current state`);
        // For now, use the full layout approach
        // In the future, this would filter based on collapsed/expanded states
        const result = await calculateFullLayout(nodes, edges, containers, layoutType);
        return {
            ...result,
            elkResult: null, // Will contain ELK raw result when needed
        };
    }
    return {
        calculateFullLayout,
        calculateVisualLayout,
    };
}
// ============ Logging Utilities ============
function logLayoutSummary(nodes, edges, containers) {
    console.log(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.SUMMARY}`);
    console.log(`  Nodes: ${nodes.length}`);
    console.log(`  Containers: ${containers.length}`);
    containers.forEach(container => {
        console.log(`    Container ${container.id}: ${container.children.size} children`);
    });
    console.log(`  Edges: ${edges.length}`);
}
function logELKInput(elkGraph) {
    console.log(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.VALIDATION} ELK CONTAINER ${LOG_PREFIXES.INPUT}:`);
    logELKContainerHierarchy(elkGraph.children, 0, LOG_PREFIXES.INPUT);
}
function logELKOutput(layoutResult) {
    console.log(`${LOG_PREFIXES.STATE_MANAGER} ${LOG_PREFIXES.VALIDATION} ELK CONTAINER ${LOG_PREFIXES.OUTPUT}:`);
    if (layoutResult.children) {
        logELKContainerHierarchy(layoutResult.children, 0, LOG_PREFIXES.OUTPUT);
    }
}
function logELKContainerHierarchy(nodes, depth, type) {
    const indent = '  '.repeat(depth);
    nodes.forEach(node => {
        if (node.children && node.children.length > 0) {
            // This is a container
            const dimensionInfo = type === LOG_PREFIXES.INPUT
                ? `width=${node.width || 'undefined'}, height=${node.height || 'undefined'}`
                : `x=${node.x}, y=${node.y}, width=${node.width}, height=${node.height}`;
            console.log(`${indent}${LOG_PREFIXES.CONTAINER} CONTAINER ${type} ${node.id}: children=${node.children.length}, ${dimensionInfo}`);
            if (type === LOG_PREFIXES.INPUT && node.layoutOptions) {
                console.log(`${indent}   layoutOptions:`, node.layoutOptions);
            }
            logELKContainerHierarchy(node.children, depth + 1, type);
        }
    });
}
//# sourceMappingURL=ELKStateManager.js.map