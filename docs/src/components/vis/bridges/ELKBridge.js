/**
 * @fileoverview ELK Bridge - Clean interface between VisState and ELK
 *
 * This bridge implements the core architectural principle:
 * - VisState contains ALL data (nodes, edges, containers)
 * - ELK gets visible elements only through visibleEdges (hyperedges included transparently)
 * - ELK returns layout positions that get applied back to VisState
 */
import { getELKLayoutOptions } from '../shared/config';
import ELK from 'elkjs';
export class ELKBridge {
    constructor(layoutConfig = {}) {
        this.elk = new ELK();
        this.layoutConfig = { algorithm: 'layered', ...layoutConfig };
    }
    /**
     * Update layout configuration
     */
    updateLayoutConfig(config) {
        this.layoutConfig = { ...this.layoutConfig, ...config };
    }
    /**
     * Convert VisState to ELK format and run layout
     * Key insight: Include ALL visible edges (regular + hyper) with no distinction
     */
    async layoutVisState(visState) {
        console.log('[ELKBridge] 🚀 Starting ELK layout from VisState');
        // 1. Extract all visible data from VisState
        const elkGraph = this.visStateToELK(visState);
        // 2. Validate ELK input data
        this.validateELKInput(elkGraph);
        // 3. Run ELK layout
        console.log(`[ELKBridge] 📊 Sending to ELK children count:`, elkGraph.children?.length);
        console.log(`[ELKBridge] 📊 ELK Graph structure:`, {
            id: elkGraph.id,
            childrenCount: elkGraph.children?.length,
            childrenIds: elkGraph.children?.map(c => c.id),
            edgesCount: elkGraph.edges?.length
        });
        const elkResult = await this.elk.layout(elkGraph);
        console.log('[ELKBridge] ✅ ELK layout complete');
        // 4. Apply results back to VisState
        this.elkToVisState(elkResult, visState);
    }
    /**
     * Validate ELK input data to prevent null reference errors
     */
    validateELKInput(elkGraph) {
        // Ensure children array exists
        if (!elkGraph.children) {
            elkGraph.children = [];
        }
        // Ensure edges array exists
        if (!elkGraph.edges) {
            elkGraph.edges = [];
        }
        // Validate each node has required properties
        elkGraph.children.forEach(node => {
            if (!node.id) {
                throw new Error(`ELK node missing ID: ${JSON.stringify(node)}`);
            }
            if (typeof node.width !== 'number' || node.width <= 0) {
                node.width = 180; // Default width
            }
            if (typeof node.height !== 'number' || node.height <= 0) {
                node.height = 60; // Default height
            }
            // Validate children if this is a container
            if (node.children) {
                node.children.forEach(child => {
                    if (!child.id) {
                        throw new Error(`ELK child node missing ID: ${JSON.stringify(child)}`);
                    }
                    if (typeof child.width !== 'number' || child.width <= 0) {
                        child.width = 180;
                    }
                    if (typeof child.height !== 'number' || child.height <= 0) {
                        child.height = 60;
                    }
                });
            }
        });
        // Validate each edge has required properties
        elkGraph.edges.forEach(edge => {
            if (!edge.id) {
                throw new Error(`ELK edge missing ID: ${JSON.stringify(edge)}`);
            }
            if (!edge.sources || edge.sources.length === 0) {
                throw new Error(`ELK edge missing sources: ${edge.id}`);
            }
            if (!edge.targets || edge.targets.length === 0) {
                throw new Error(`ELK edge missing targets: ${edge.id}`);
            }
        });
    }
    /**
     * Convert VisState to ELK format
     */
    visStateToELK(visState) {
        // Extract visible nodes (both regular nodes and collapsed containers)
        const visibleNodes = this.extractVisibleNodes(visState);
        // Extract container hierarchy for visible containers
        const visibleContainers = this.extractVisibleContainers(visState);
        // Extract ALL edges via the unified visibleEdges interface
        // This now includes hyperedges transparently - ELK doesn't need to know the difference
        const allEdges = visState.visibleEdges;
        console.log('[ELKBridge] 📋 Extracted from VisState:', {
            visibleNodes: visibleNodes.length,
            visibleContainers: visibleContainers.length,
            totalEdges: allEdges.length,
            regularEdges: allEdges.length,
            hyperEdges: 0
        });
        return this.buildELKGraph(visibleNodes, visibleContainers, allEdges);
    }
    /**
     * Extract visible nodes (both GraphNodes and collapsed containers as nodes)
     */
    extractVisibleNodes(visState) {
        const nodes = [];
        // Add visible regular nodes using the correct VisState API
        const visibleNodes = visState.visibleNodes;
        nodes.push(...visibleNodes);
        // Add collapsed containers as nodes (they should be treated as regular nodes by ELK)
        const visibleContainers = visState.visibleContainers;
        visibleContainers.forEach(container => {
            if (container.collapsed) {
                // Convert collapsed container to a node-like structure for ELK
                const containerAsNode = {
                    id: container.id,
                    label: container.id,
                    // Use computed dimensions from VisState
                    width: container.width || 200, // SIZES.COLLAPSED_CONTAINER_WIDTH
                    height: container.height || 60, // SIZES.COLLAPSED_CONTAINER_HEIGHT
                    x: container.x || 0, // Use computed position
                    y: container.y || 0, // Use computed position
                    hidden: false,
                    style: 'default' // Use valid NodeStyle
                };
                nodes.push(containerAsNode);
            }
        });
        return nodes;
    }
    /**
     * Extract visible containers (only expanded ones that need hierarchical layout)
     */
    extractVisibleContainers(visState) {
        const containers = [];
        const expandedContainers = visState.expandedContainers;
        containers.push(...expandedContainers);
        return containers;
    }
    /**
     * Build ELK graph from extracted data
     */
    buildELKGraph(nodes, containers, edges) {
        console.log(`[ELKBridge] 🔨 Building ELK graph with ${nodes.length} nodes, ${containers.length} containers, ${edges.length} edges`);
        console.log(`[ELKBridge] 🔍 Available nodes:`, nodes.map(n => n.id));
        console.log(`[ELKBridge] 🔍 Available containers:`, containers.map(c => ({ id: c.id, children: Array.from(c.children), collapsed: c.collapsed })));
        // Build hierarchy: create nested ELK structure
        const elkNodes = [];
        // Find root containers (containers with no parent container)
        const rootContainers = containers.filter(container => {
            // Check if this container has a parent that's also a container
            const hasContainerParent = containers.some(otherContainer => otherContainer.children.has(container.id));
            return !hasContainerParent;
        });
        console.log(`[ELKBridge] 🔍 Found ${rootContainers.length} root containers:`, rootContainers.map(c => c.id));
        // Recursively build ELK hierarchy starting from root containers
        const buildContainerHierarchy = (container) => {
            // Find child nodes (regular nodes)
            const childNodes = nodes.filter(node => container.children.has(node.id));
            // Find child containers (nested containers)
            const childContainers = containers.filter(childContainer => container.children.has(childContainer.id));
            console.log(`[ELKBridge] 🔍 Container ${container.id} has ${childNodes.length} nodes and ${childContainers.length} containers:`, {
                nodes: childNodes.map(n => n.id),
                containers: childContainers.map(c => c.id),
                allChildren: Array.from(container.children)
            });
            // Create ELK children array with both nodes and nested containers
            const elkChildren = [
                // Add child nodes
                ...childNodes.map(node => ({
                    id: node.id,
                    width: node.width || 180,
                    height: node.height || 60
                })),
                // Add child containers (recursively)
                ...childContainers.map(childContainer => buildContainerHierarchy(childContainer))
            ];
            return {
                id: container.id,
                width: 400, // Default width - ELK will calculate proper size based on content
                height: 300, // Default height - ELK will calculate proper size based on content
                children: elkChildren,
                layoutOptions: getELKLayoutOptions(this.layoutConfig.algorithm)
            };
        };
        // Build hierarchy for each root container
        rootContainers.forEach(container => {
            const hierarchyNode = buildContainerHierarchy(container);
            console.log(`[ELKBridge] 🏗️ Built hierarchy for ${container.id}:`, JSON.stringify(hierarchyNode, null, 2));
            elkNodes.push(hierarchyNode);
        });
        console.log(`[ELKBridge] 🔍 Final elkNodes array length: ${elkNodes.length}`);
        // Add top-level nodes (not in any container, including collapsed containers)
        const topLevelNodes = nodes.filter(node => !this.isNodeInAnyContainer(node.id, containers));
        console.log(`[ELKBridge] 🔍 Found ${topLevelNodes.length} top-level nodes:`, topLevelNodes.map(n => n.id));
        topLevelNodes.forEach(node => {
            elkNodes.push({
                id: node.id,
                width: node.width || 180, // Use computed width
                height: node.height || 60 // Use computed height
            });
        });
        // Convert all edges to ELK format
        const elkEdges = edges.map(edge => ({
            id: edge.id,
            sources: [edge.source],
            targets: [edge.target]
        }));
        return {
            id: 'root',
            children: elkNodes,
            edges: elkEdges,
            layoutOptions: getELKLayoutOptions(this.layoutConfig.algorithm)
        };
    }
    /**
     * Apply ELK results back to VisState
     */
    elkToVisState(elkResult, visState) {
        console.log('[ELKBridge] 📝 Applying ELK results back to VisState');
        console.log('[ELKBridge] 🔍 ELK Result Structure:', JSON.stringify(elkResult, null, 2));
        if (!elkResult.children) {
            console.warn('[ELKBridge] ⚠️ No children in ELK result');
            return;
        }
        // Apply positions to containers and nodes
        elkResult.children.forEach(elkNode => {
            if (elkNode.children && elkNode.children.length > 0) {
                // This is a container
                this.updateContainerFromELK(elkNode, visState);
            }
            else {
                // This is a top-level node (or collapsed container)
                this.updateNodeFromELK(elkNode, visState);
            }
        });
        // Apply edge routing information
        console.log('--- ELKBRIDGE_EDGE_PROCESSING_START ---');
        const allEdges = elkResult.edges || [];
        if (allEdges.length > 0) {
            console.log(`[ELKBridge] 🔍 Processing ${allEdges.length} edges for sections`);
            allEdges.forEach(elkEdge => {
                this.updateEdgeFromELK(elkEdge, visState);
            });
        }
        else {
            console.log('[ELKBridge] ⚠️ No edges array in ELK result');
        }
        console.log('--- ELKBRIDGE_EDGE_PROCESSING_END ---');
        console.log('[ELKBridge] ✅ Applied all ELK results to VisState');
    }
    /**
     * Update edge routing information from ELK result
     */
    updateEdgeFromELK(elkEdge, visState) {
        // Use VisState's proper layout method instead of direct property access
        if (elkEdge.sections && elkEdge.sections.length > 0) {
            console.log(`[ELKBridge] 🔧 About to set layout for edge ${elkEdge.id} with ${elkEdge.sections.length} sections`);
            visState.setEdgeLayout(elkEdge.id, { sections: elkEdge.sections });
            console.log(`[ELKBridge] 📍 Updated edge ${elkEdge.id} with ${elkEdge.sections.length} sections`);
            // Debug: Try to read back the edge to see if it was set
            const edge = visState.getGraphEdge(elkEdge.id);
            console.log(`[ELKBridge] 🔍 Debug: Edge ${elkEdge.id} layout after update:`, edge?.layout);
        }
        else {
            console.log(`[ELKBridge] 📍 Edge ${elkEdge.id} has no sections (cross-container edge)`);
        }
    }
    /**
     * Update container dimensions and child positions from ELK result
     */
    updateContainerFromELK(elkNode, visState) {
        // Use VisState's proper layout methods instead of direct property access
        const layoutUpdates = {};
        if (elkNode.x !== undefined || elkNode.y !== undefined) {
            layoutUpdates.position = {};
            if (elkNode.x !== undefined)
                layoutUpdates.position.x = elkNode.x;
            if (elkNode.y !== undefined)
                layoutUpdates.position.y = elkNode.y;
        }
        if (elkNode.width !== undefined || elkNode.height !== undefined) {
            layoutUpdates.dimensions = {};
            if (elkNode.width !== undefined)
                layoutUpdates.dimensions.width = elkNode.width;
            if (elkNode.height !== undefined)
                layoutUpdates.dimensions.height = elkNode.height;
        }
        if (Object.keys(layoutUpdates).length > 0) {
            visState.setContainerLayout(elkNode.id, layoutUpdates);
            console.log(`[ELKBridge] 📏 Updated container ${elkNode.id} layout: ${JSON.stringify(layoutUpdates)}`);
        }
        // Update child positions (recursively handle containers vs nodes)
        elkNode.children?.forEach(elkChildNode => {
            if (elkChildNode.children && elkChildNode.children.length > 0) {
                // This child is also a container - recurse into it
                this.updateContainerFromELK(elkChildNode, visState);
            }
            else {
                // This child is a leaf node - update its position
                this.updateNodeFromELK(elkChildNode, visState);
            }
        });
    }
    /**
     * Update node position from ELK result
     */
    updateNodeFromELK(elkNode, visState) {
        console.log(`[ELKBridge] 🔧 Attempting to update node ${elkNode.id} with ELK coords (${elkNode.x}, ${elkNode.y})`);
        // Try to update as regular node first using VisState's layout methods
        try {
            const layoutUpdates = {};
            if (elkNode.x !== undefined || elkNode.y !== undefined) {
                layoutUpdates.position = {};
                if (elkNode.x !== undefined)
                    layoutUpdates.position.x = elkNode.x;
                if (elkNode.y !== undefined)
                    layoutUpdates.position.y = elkNode.y;
            }
            if (elkNode.width !== undefined || elkNode.height !== undefined) {
                layoutUpdates.dimensions = {};
                if (elkNode.width !== undefined)
                    layoutUpdates.dimensions.width = elkNode.width;
                if (elkNode.height !== undefined)
                    layoutUpdates.dimensions.height = elkNode.height;
            }
            if (Object.keys(layoutUpdates).length > 0) {
                console.log(`[ELKBridge] 📏 Calling setNodeLayout for ${elkNode.id} with:`, layoutUpdates);
                visState.setNodeLayout(elkNode.id, layoutUpdates);
                console.log(`[ELKBridge] ✅ Successfully updated node ${elkNode.id}`);
            }
            return;
        }
        catch (nodeError) {
            console.log(`[ELKBridge] ⚠️ Node ${elkNode.id} not found as regular node, trying as container:`, nodeError.message);
            // If not found as node, might be a collapsed container
            try {
                const layoutUpdates = {};
                if (elkNode.x !== undefined || elkNode.y !== undefined) {
                    layoutUpdates.position = {};
                    if (elkNode.x !== undefined)
                        layoutUpdates.position.x = elkNode.x;
                    if (elkNode.y !== undefined)
                        layoutUpdates.position.y = elkNode.y;
                }
                if (elkNode.width !== undefined || elkNode.height !== undefined) {
                    layoutUpdates.dimensions = {};
                    if (elkNode.width !== undefined)
                        layoutUpdates.dimensions.width = elkNode.width;
                    if (elkNode.height !== undefined)
                        layoutUpdates.dimensions.height = elkNode.height;
                }
                if (Object.keys(layoutUpdates).length > 0) {
                    visState.setContainerLayout(elkNode.id, layoutUpdates);
                }
                return;
            }
            catch (containerError) {
                console.warn(`[ELKBridge] Node/Container ${elkNode.id} not found in VisState`);
            }
        }
    }
    // Helper methods for containment logic
    isNodeInContainer(nodeId, containerId, container) {
        // Use the container's children set
        return container.children.has(nodeId);
    }
    isNodeInAnyContainer(nodeId, containers) {
        return containers.some(container => this.isNodeInContainer(nodeId, container.id, container));
    }
}
//# sourceMappingURL=ELKBridge.js.map