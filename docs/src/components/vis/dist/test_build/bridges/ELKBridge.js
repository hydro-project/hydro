/**
 * @fileoverview ELK Bridge - Clean interface between VisState and ELK
 *
 * This bridge implements the core architectural principle:
 * - VisState contains ALL data (nodes, edges, hyperEdges, containers)
 * - ELK gets visible elements only, with no distinction between edge types
 * - ELK returns layout positions that get applied back to VisState
 */
import ELK from 'elkjs';
export class ELKBridge {
    constructor() {
        this.elk = new ELK();
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
        console.log('[ELKBridge] 📊 Sending to ELK:', {
            nodes: elkGraph.children?.length || 0,
            edges: elkGraph.edges?.length || 0
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
        // Extract ALL edges (regular + hyperedges) - this is the key fix!
        const allEdges = this.extractAllEdges(visState);
        console.log('[ELKBridge] 📋 Extracted from VisState:', {
            visibleNodes: visibleNodes.length,
            visibleContainers: visibleContainers.length,
            totalEdges: allEdges.length,
            regularEdges: allEdges.filter(e => !e.id.includes('hyper_')).length,
            hyperEdges: allEdges.filter(e => e.id.includes('hyper_')).length
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
                    // Use collapsed dimensions if available, otherwise use defaults
                    width: container.layout?.dimensions?.width || 200, // SIZES.COLLAPSED_CONTAINER_WIDTH
                    height: container.layout?.dimensions?.height || 60, // SIZES.COLLAPSED_CONTAINER_HEIGHT
                    x: container.layout?.position?.x || 0,
                    y: container.layout?.position?.y || 0,
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
     * Extract ALL edges - both regular edges and hyperedges with no distinction
     * This is the critical fix: hyperedges were getting lost in the old implementation
     */
    extractAllEdges(visState) {
        const allEdges = [];
        // Add visible regular edges
        const visibleEdges = visState.visibleEdges;
        allEdges.push(...visibleEdges);
        // Add ALL hyperedges (this was missing in the old implementation!)
        const hyperEdges = visState.allHyperEdges;
        allEdges.push(...hyperEdges);
        return allEdges;
    }
    /**
     * Build ELK graph from extracted data
     */
    buildELKGraph(nodes, containers, edges) {
        // Build hierarchy: top-level nodes and containers
        const elkNodes = [];
        // Add expanded containers as ELK nodes with children
        containers.forEach(container => {
            const childNodes = nodes.filter(node => {
                // Find nodes that belong to this container using VisState's hierarchy info
                return this.isNodeInContainer(node.id, container.id, container);
            });
            elkNodes.push({
                id: container.id,
                width: container.layout?.dimensions?.width,
                height: container.layout?.dimensions?.height,
                children: childNodes.map(node => ({
                    id: node.id,
                    width: node.width || 180,
                    height: node.height || 60
                })),
                layoutOptions: {
                    'elk.algorithm': 'layered',
                    'elk.direction': 'DOWN',
                    'elk.spacing.nodeNode': '75'
                }
            });
        });
        // Add top-level nodes (not in any container, including collapsed containers)
        nodes.forEach(node => {
            if (!this.isNodeInAnyContainer(node.id, containers)) {
                elkNodes.push({
                    id: node.id,
                    width: node.width || 180,
                    height: node.height || 60
                });
            }
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
            layoutOptions: {
                'elk.algorithm': 'layered',
                'elk.direction': 'DOWN',
                'elk.spacing.nodeNode': '100',
                'elk.spacing.edgeNode': '50'
            }
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
        console.log('[ELKBridge] ✅ Applied all ELK results to VisState');
    }
    /**
     * Update container dimensions and child positions from ELK result
     */
    updateContainerFromELK(elkNode, visState) {
        const container = visState.getContainer(elkNode.id);
        if (!container) {
            console.warn(`[ELKBridge] Container ${elkNode.id} not found in VisState`);
            return;
        }
        // Update container position and dimensions
        container.layout = container.layout || {};
        if (elkNode.x !== undefined) {
            container.layout.position = container.layout.position || {};
            container.layout.position.x = elkNode.x;
        }
        if (elkNode.y !== undefined) {
            container.layout.position = container.layout.position || {};
            container.layout.position.y = elkNode.y;
        }
        if (elkNode.width !== undefined || elkNode.height !== undefined) {
            container.layout.dimensions = container.layout.dimensions || {};
            if (elkNode.width !== undefined)
                container.layout.dimensions.width = elkNode.width;
            if (elkNode.height !== undefined)
                container.layout.dimensions.height = elkNode.height;
        }
        // Update child node positions
        elkNode.children?.forEach(elkChildNode => {
            this.updateNodeFromELK(elkChildNode, visState);
        });
    }
    /**
     * Update node position from ELK result
     */
    updateNodeFromELK(elkNode, visState) {
        // Try to find as regular node first
        let node = visState.getGraphNode(elkNode.id);
        if (node) {
            node.x = elkNode.x || 0;
            node.y = elkNode.y || 0;
            if (elkNode.width)
                node.width = elkNode.width;
            if (elkNode.height)
                node.height = elkNode.height;
            return;
        }
        // If not found as node, might be a collapsed container
        const container = visState.getContainer(elkNode.id);
        if (container && container.collapsed) {
            container.layout = container.layout || {};
            if (elkNode.x !== undefined || elkNode.y !== undefined) {
                container.layout.position = container.layout.position || {};
                if (elkNode.x !== undefined)
                    container.layout.position.x = elkNode.x;
                if (elkNode.y !== undefined)
                    container.layout.position.y = elkNode.y;
            }
            if (elkNode.width !== undefined || elkNode.height !== undefined) {
                container.layout.dimensions = container.layout.dimensions || {};
                if (elkNode.width !== undefined)
                    container.layout.dimensions.width = elkNode.width;
                if (elkNode.height !== undefined)
                    container.layout.dimensions.height = elkNode.height;
            }
            return;
        }
        console.warn(`[ELKBridge] Node/Container ${elkNode.id} not found in VisState`);
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