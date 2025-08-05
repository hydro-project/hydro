/**
 * Container Collapse/Expand Engine
 *
 * Handles sophisticated container state transitions with symmetric edge lifting/grounding operations.
 * Ensures proper tree hierarchy validation and optimized edge processing.
 */
import { EDGE_STYLES } from '../shared/constants';
// Constants for consistent string literals
const HYPER_EDGE_PREFIX = 'hyper_';
const DEFAULT_STYLE = 'default';
/**
 * Container collapse/expand operations with tree hierarchy validation and optimized edge processing.
 *
 * Features:
 * - Tree hierarchy enforcement (no cycles/DAGs)
 * - Symmetric collapse ↔️ expand operations
 * - Edge lifting/grounding with proper metadata preservation
 * - Optimized edge lookup with indexing
 * - Sequential operation guarantee (no concurrency)
 *
 * @class ContainerCollapseExpandEngine
 */
export class ContainerCollapseExpandEngine {
    constructor(visualizationState) {
        this.state = visualizationState;
        this.containerToEdges = new Map();
        this._buildContainerEdgeIndex();
    }
    // ============ Public API ============
    /**
     * Collapse a container (depth-first, bottom-up with edge lifting)
     * Validates tree hierarchy and processes edges efficiently
     */
    collapseContainer(containerId) {
        const container = this.state.getContainer(containerId);
        // Validate container exists and tree hierarchy
        this._validateContainerForCollapse(containerId, container);
        if (container.collapsed) {
            return; // Already collapsed
        }
        // First, recursively collapse any child containers (bottom-up)
        const children = this.state.getContainerChildren(containerId);
        for (const childId of children) {
            if (this.state.containers.has(childId)) {
                this.collapseContainer(childId);
            }
        }
        // Now collapse this container and lift edges/hyperEdges to this level
        this._performCollapseWithLift(containerId);
        // Update edge index
        this._updateContainerEdgeIndex(containerId);
    }
    /**
     * Expand a container (depth-first, top-down with edge grounding)
     * SYMMETRIC INVERSE of collapseContainer()
     */
    expandContainer(containerId) {
        const container = this.state.getContainer(containerId);
        // Validate container exists and state
        this._validateContainerForExpansion(containerId, container);
        if (!container.collapsed) {
            return; // Already expanded
        }
        // First expand this container and ground edges/hyperEdges to child level
        this._performExpandWithGround(containerId);
        // Then recursively expand any child containers (top-down)
        const children = this.state.getContainerChildren(containerId);
        for (const childId of children) {
            if (this.state.containers.has(childId)) {
                this.expandContainer(childId);
            }
        }
        // Update edge index
        this._updateContainerEdgeIndex(containerId);
    }
    /**
     * Validate tree hierarchy when adding container child
     * Prevents cycles and enforces single-parent constraint
     */
    validateTreeHierarchy(parentId, childId) {
        // Check for self-reference
        if (parentId === childId) {
            throw new Error(`Cannot add container '${childId}' as child of itself`);
        }
        // Check if child would create a cycle
        if (this._wouldCreateCycle(parentId, childId)) {
            throw new Error(`Adding '${childId}' to '${parentId}' would create a cycle in container hierarchy`);
        }
        // Check if child already has a different parent
        const existingParent = this.state.getNodeContainer(childId);
        if (existingParent && existingParent !== parentId) {
            throw new Error(`Container '${childId}' already has parent '${existingParent}'. Each container can have only one parent.`);
        }
    }
    /**
     * Rebuild the container-to-edges index for optimized lookups
     */
    rebuildEdgeIndex() {
        this.containerToEdges.clear();
        this._buildContainerEdgeIndex();
    }
    // ============ Core Implementation (Symmetric Pair) ============
    /**
     * Perform the actual collapse operation for a single container
     * This includes lifting edges and hyperEdges from child containers
     */
    _performCollapseWithLift(containerId) {
        const container = this.state.getContainer(containerId);
        // 1. Create collapsed container representation
        this._createCollapsedContainerRepresentation(containerId, container);
        // 2. Mark container as collapsed
        this._markContainerAsCollapsed(containerId, container);
        // 3. Get and categorize children
        const children = this.state.getContainerChildren(containerId);
        const { containerNodes, childContainers } = this._categorizeChildren(children);
        // 4. Hide child nodes and handle edge rerouting
        this._hideChildNodesAndRerouteEdges(containerId, containerNodes);
        // 5. Lift edges and hyperEdges to this container level
        this._liftEdgesToContainer(containerId, containerNodes, childContainers);
    }
    /**
     * Perform the actual expansion operation for a single container
     * SYMMETRIC INVERSE of _performCollapseWithLift()
     */
    _performExpandWithGround(containerId) {
        const container = this.state.getContainer(containerId);
        const collapsedContainer = this.state.collapsedContainers.get(containerId);
        if (!collapsedContainer)
            return;
        // 1. Mark container as expanded and cleanup
        this._markContainerAsExpandedAndCleanup(containerId, container);
        // 2. Show child nodes
        this._showChildNodes(containerId);
        // 3. Ground hyperEdges and edges from this container to child level
        this._groundEdgesFromContainer(containerId);
    }
    // ============ Validation Helpers ============
    _validateContainerForCollapse(containerId, container) {
        if (!container) {
            throw new Error(`Cannot collapse container: container '${containerId}' does not exist`);
        }
    }
    _validateContainerForExpansion(containerId, container) {
        if (!container) {
            throw new Error(`Cannot expand container: container '${containerId}' does not exist`);
        }
    }
    /**
     * Check if adding childId to parentId would create a cycle
     * Uses DFS to detect cycles in the container hierarchy
     */
    _wouldCreateCycle(parentId, childId) {
        const visited = new Set();
        const dfs = (currentId) => {
            if (currentId === childId) {
                return true; // Found cycle
            }
            if (visited.has(currentId)) {
                return false; // Already explored this path
            }
            visited.add(currentId);
            // Check all ancestors of currentId
            const parent = this.state.getNodeContainer(currentId);
            if (parent) {
                return dfs(parent);
            }
            return false;
        };
        return dfs(parentId);
    }
    // ============ Edge Index Management ============
    /**
     * Build optimized index of container -> edges for efficient lookups
     */
    _buildContainerEdgeIndex() {
        // For each edge, add it to the index for all containers that contain its endpoints
        for (const [edgeId, edge] of this.state.graphEdges) {
            this._indexEdgeForContainers(edgeId, edge);
        }
    }
    /**
     * Add an edge to the container index for all relevant containers
     */
    _indexEdgeForContainers(edgeId, edge) {
        const sourceContainer = this.state.getNodeContainer(edge.source);
        const targetContainer = this.state.getNodeContainer(edge.target);
        // Add edge to source container's index
        if (sourceContainer) {
            this._addEdgeToContainerIndex(sourceContainer, edgeId);
        }
        // Add edge to target container's index (if different)
        if (targetContainer && targetContainer !== sourceContainer) {
            this._addEdgeToContainerIndex(targetContainer, edgeId);
        }
    }
    /**
     * Add edge to container's edge set in the index
     */
    _addEdgeToContainerIndex(containerId, edgeId) {
        if (!this.containerToEdges.has(containerId)) {
            this.containerToEdges.set(containerId, new Set());
        }
        this.containerToEdges.get(containerId).add(edgeId);
    }
    /**
     * Update edge index when container hierarchy changes
     */
    _updateContainerEdgeIndex(containerId) {
        // Remove existing entries for this container
        this.containerToEdges.delete(containerId);
        // Rebuild entries for affected edges
        const children = this.state.getContainerChildren(containerId);
        for (const childId of children) {
            const childEdges = this.state.nodeToEdges.get(childId) || new Set();
            for (const edgeId of childEdges) {
                const edge = this.state.graphEdges.get(edgeId);
                if (edge) {
                    this._indexEdgeForContainers(edgeId, edge);
                }
            }
        }
    }
    /**
     * Get edges efficiently using the optimized index
     */
    _getContainerEdges(containerId) {
        return this.containerToEdges.get(containerId) || new Set();
    }
    // ============ State Management Helpers ============
    _createCollapsedContainerRepresentation(containerId, container) {
        this.state.collapsedContainers.set(containerId, {
            id: containerId,
            originalContainer: container,
            style: container.style || DEFAULT_STYLE
        });
    }
    _markContainerAsCollapsed(containerId, container) {
        container.collapsed = true;
        this.state._updateExpandedContainers(containerId, container);
        // Set collapsed dimensions in VisState
        this.state.setContainerLayout(containerId, {
            dimensions: {
                width: 200, // SIZES.COLLAPSED_CONTAINER_WIDTH
                height: 60 // SIZES.COLLAPSED_CONTAINER_HEIGHT
            }
        });
    }
    _markContainerAsExpandedAndCleanup(containerId, container) {
        container.collapsed = false;
        this.state._updateExpandedContainers(containerId, container);
        this.state.collapsedContainers.delete(containerId);
        // Restore expanded dimensions from expandedDimensions property
        if (container?.expandedDimensions) {
            this.state.setContainerLayout(containerId, {
                dimensions: container.expandedDimensions
            });
        }
    }
    _showChildNodes(containerId) {
        const children = this.state.getContainerChildren(containerId);
        const { containerNodes } = this._categorizeChildren(children);
        this._setNodesVisibility(containerNodes, false);
    }
    _hideChildNodesAndRerouteEdges(containerId, containerNodes) {
        this._setNodesVisibility(containerNodes, true);
        this._rerouteHyperEdgesToCollapsedContainer(containerId, containerNodes);
    }
    // ============ Edge Processing Helpers ============
    _liftEdgesToContainer(containerId, containerNodes, childContainers) {
        const liftedConnections = new Map(); // externalId -> {incoming: Set, outgoing: Set}
        // Process direct node edges using optimized index
        this._liftNodeEdgesOptimized(containerId, containerNodes, liftedConnections);
        // Process hyperEdges from child containers
        this._liftChildContainerHyperEdges(containerId, childContainers, liftedConnections);
        // Create new hyperEdges for all lifted connections
        this._createHyperEdgesFromLiftedConnections(containerId, liftedConnections);
    }
    /**
     * Optimized edge lifting using container edge index
     */
    _liftNodeEdgesOptimized(containerId, containerNodes, liftedConnections) {
        const processedEdges = new Set();
        // Use optimized index to get relevant edges
        const relevantEdges = this._getContainerEdges(containerId);
        for (const edgeId of relevantEdges) {
            if (processedEdges.has(edgeId))
                continue;
            processedEdges.add(edgeId);
            const edge = this.state.graphEdges.get(edgeId);
            if (!edge)
                continue;
            this._processNodeEdge(edge, containerNodes, liftedConnections);
        }
    }
    _groundEdgesFromContainer(containerId) {
        const children = this.state.getContainerChildren(containerId);
        // Process hyperEdges connected to this container
        this._groundContainerHyperEdges(containerId);
        // Process direct node edges that were hidden during collapse
        this._groundNodeEdges(containerId, children);
    }
    _processNodeEdge(edge, containerNodes, liftedConnections) {
        const sourceInContainer = containerNodes.has(edge.source);
        const targetInContainer = containerNodes.has(edge.target);
        if (sourceInContainer && targetInContainer) {
            // Both endpoints in container - hide the edge (internal edge)
            this.state.updateEdge(edge.id, { hidden: true });
        }
        else if (sourceInContainer || targetInContainer) {
            // One endpoint in container, one external
            const externalId = sourceInContainer ? edge.target : edge.source;
            const internalId = sourceInContainer ? edge.source : edge.target;
            // Only create hyperEdge if the external endpoint should be connected
            if (this._isEndpointConnectable(externalId)) {
                const isOutgoing = sourceInContainer; // container -> external
                this._addToLiftedConnections(liftedConnections, externalId, edge, isOutgoing, internalId);
            }
            // Hide the original edge regardless
            this.state.updateEdge(edge.id, { hidden: true });
        }
    }
    _groundNodeEdges(containerId, children) {
        // Restore internal edges (edges between nodes in this container)
        for (const [edgeId, edge] of this.state.graphEdges) {
            if (!edge.hidden)
                continue; // Skip already visible edges
            const sourceNode = this.state.graphNodes.get(edge.source);
            const targetNode = this.state.graphNodes.get(edge.target);
            // Both endpoints must be nodes (not containers) and visible
            if (sourceNode && !sourceNode.hidden && targetNode && !targetNode.hidden) {
                this.state.updateEdge(edgeId, { hidden: false });
            }
        }
    }
    // ============ HyperEdge Processing ============
    _liftChildContainerHyperEdges(containerId, childContainers, liftedConnections) {
        this._processHyperEdges((hyperEdge) => childContainers.has(hyperEdge.source) || childContainers.has(hyperEdge.target), (hyperEdge) => this._liftChildContainerHyperEdge(hyperEdge, childContainers, liftedConnections));
    }
    _liftChildContainerHyperEdge(hyperEdge, childContainers, liftedConnections) {
        const sourceIsChild = childContainers.has(hyperEdge.source);
        const targetIsChild = childContainers.has(hyperEdge.target);
        if (sourceIsChild || targetIsChild) {
            const externalId = sourceIsChild ? hyperEdge.target : hyperEdge.source;
            const isOutgoing = sourceIsChild; // child container -> external
            // Only lift if the external endpoint is connectable
            if (this._isEndpointConnectable(externalId) && hyperEdge.originalEdges) {
                for (const originalEdge of hyperEdge.originalEdges) {
                    const childInternalEndpoint = hyperEdge.originalInternalEndpoint ||
                        (sourceIsChild ? hyperEdge.source : hyperEdge.target);
                    this._addToLiftedConnections(liftedConnections, externalId, originalEdge, isOutgoing, childInternalEndpoint);
                }
            }
        }
    }
    _groundContainerHyperEdges(containerId) {
        this._processHyperEdges((hyperEdge) => hyperEdge.source === containerId || hyperEdge.target === containerId, (hyperEdge) => this._groundSingleContainerHyperEdge(hyperEdge, containerId));
    }
    _groundSingleContainerHyperEdge(hyperEdge, containerId) {
        const isSourceContainer = hyperEdge.source === containerId;
        const externalId = isSourceContainer ? hyperEdge.target : hyperEdge.source;
        const internalEndpoint = hyperEdge.originalInternalEndpoint;
        this._groundConnection(externalId, internalEndpoint, hyperEdge, isSourceContainer);
    }
    _rerouteHyperEdgesToCollapsedContainer(containerId, containerNodes) {
        const hyperEdgesToUpdate = [];
        // Find hyperEdges that need rerouting
        for (const [hyperEdgeId, hyperEdge] of this.state.hyperEdges) {
            const update = this._calculateHyperEdgeReroute(hyperEdge, containerNodes, containerId);
            if (update) {
                hyperEdgesToUpdate.push({ id: hyperEdgeId, originalHyperEdge: hyperEdge, ...update });
            }
        }
        // Apply the updates
        for (const update of hyperEdgesToUpdate) {
            this.state.removeHyperEdge(update.id);
            // Only create a new hyperEdge if source and target are different
            if (update.newSource !== update.newTarget) {
                const newHyperEdgeId = `${HYPER_EDGE_PREFIX}${update.newSource}_to_${update.newTarget}`;
                this.state.setHyperEdge(newHyperEdgeId, {
                    source: update.newSource,
                    target: update.newTarget,
                    style: update.originalHyperEdge.style,
                    originalEdges: update.originalHyperEdge.originalEdges,
                    originalInternalEndpoint: update.originalHyperEdge.originalInternalEndpoint
                });
            }
        }
    }
    _calculateHyperEdgeReroute(hyperEdge, containerNodes, containerId) {
        let needsUpdate = false;
        let newSource = hyperEdge.source;
        let newTarget = hyperEdge.target;
        // Check if source is a node we're hiding
        if (containerNodes.has(hyperEdge.source)) {
            newSource = containerId;
            needsUpdate = true;
        }
        // Check if target is a node we're hiding
        if (containerNodes.has(hyperEdge.target)) {
            newTarget = containerId;
            needsUpdate = true;
        }
        return needsUpdate ? { newSource, newTarget } : null;
    }
    // ============ Utility Helpers ============
    _categorizeChildren(children) {
        const containerNodes = new Set();
        const childContainers = new Set();
        for (const childId of children) {
            if (this.state.graphNodes.has(childId)) {
                containerNodes.add(childId);
            }
            else if (this.state.containers.has(childId)) {
                childContainers.add(childId);
            }
        }
        return { containerNodes, childContainers };
    }
    _setNodesVisibility(nodeIds, hidden) {
        for (const nodeId of nodeIds) {
            this.state.updateNode(nodeId, { hidden });
        }
    }
    _isEndpointConnectable(endpointId) {
        // Check if endpoint is a visible node
        const node = this.state.graphNodes.get(endpointId);
        if (node && !node.hidden) {
            return true;
        }
        // Check if endpoint is a visible, collapsed container
        const container = this.state.containers.get(endpointId);
        if (container && !container.hidden && container.collapsed) {
            return true;
        }
        return false;
    }
    _processHyperEdges(predicate, updateFn) {
        const hyperEdgesToRemove = [];
        for (const [hyperEdgeId, hyperEdge] of this.state.hyperEdges) {
            if (predicate(hyperEdge)) {
                hyperEdgesToRemove.push(hyperEdgeId);
                if (updateFn) {
                    updateFn(hyperEdge, hyperEdgeId);
                }
            }
        }
        // Remove processed hyperEdges
        for (const hyperEdgeId of hyperEdgesToRemove) {
            this.state.removeHyperEdge(hyperEdgeId);
        }
    }
    _addToLiftedConnections(liftedConnections, externalId, edge, isOutgoing, internalEndpoint) {
        if (!liftedConnections.has(externalId)) {
            liftedConnections.set(externalId, { incoming: new Set(), outgoing: new Set() });
        }
        const direction = isOutgoing ? 'outgoing' : 'incoming';
        const connections = liftedConnections.get(externalId);
        // Store the edge with its original internal endpoint
        const edgeWithEndpoint = {
            ...edge,
            originalInternalEndpoint: internalEndpoint
        };
        connections[direction].add(edgeWithEndpoint);
    }
    _groundConnection(externalId, internalEndpoint, hyperEdge, isSourceContainer) {
        if (hyperEdge.originalEdges) {
            // Restore original edges only if both endpoints are visible
            for (const originalEdge of hyperEdge.originalEdges) {
                const sourceNode = this.state.graphNodes.get(originalEdge.source);
                const targetNode = this.state.graphNodes.get(originalEdge.target);
                // Only restore edge if both endpoints are visible nodes
                if (sourceNode && !sourceNode.hidden && targetNode && !targetNode.hidden) {
                    this.state.updateEdge(originalEdge.id, { hidden: false });
                }
            }
        }
    }
    _createHyperEdgesFromLiftedConnections(containerId, liftedConnections) {
        for (const [externalId, connections] of liftedConnections) {
            this._createDirectionalHyperEdges(containerId, externalId, connections);
        }
    }
    _createDirectionalHyperEdges(containerId, externalId, connections) {
        if (connections.incoming.size > 0) {
            this._createHyperEdge(externalId, containerId, Array.from(connections.incoming));
        }
        if (connections.outgoing.size > 0) {
            this._createHyperEdge(containerId, externalId, Array.from(connections.outgoing));
        }
    }
    _createHyperEdge(sourceId, targetId, edgesArray) {
        const hyperEdgeId = `${HYPER_EDGE_PREFIX}${sourceId}_to_${targetId}`;
        this.state.setHyperEdge(hyperEdgeId, {
            source: sourceId,
            target: targetId,
            style: this._aggregateEdgeStyles(edgesArray),
            originalEdges: edgesArray.map(e => ({ id: e.id, source: e.source, target: e.target, style: e.style })),
            originalInternalEndpoint: edgesArray[0].originalInternalEndpoint ||
                this._findOriginalInternalEndpoint(edgesArray, targetId === sourceId ? sourceId : targetId)
        });
    }
    _findOriginalInternalEndpoint(edges, containerId) {
        const children = this.state.getContainerChildren(containerId);
        const internalEndpoints = new Set();
        for (const edge of edges) {
            const internalEndpoint = children.has(edge.source) ? edge.source : edge.target;
            internalEndpoints.add(internalEndpoint);
        }
        // If multiple internal endpoints, prefer containers over nodes
        const containerEndpoints = Array.from(internalEndpoints).filter(id => this.state.containers.has(id));
        const nodeEndpoints = Array.from(internalEndpoints).filter(id => this.state.graphNodes.has(id));
        if (containerEndpoints.length > 0) {
            return containerEndpoints[0]; // Prefer containers
        }
        else if (nodeEndpoints.length > 0) {
            return nodeEndpoints[0]; // Fall back to nodes
        }
        return Array.from(internalEndpoints)[0]; // Fallback
    }
    _aggregateEdgeStyles(edges) {
        // Priority order: ERROR > WARNING > THICK > HIGHLIGHTED > DEFAULT
        const stylePriority = {
            'error': 5,
            'warning': 4,
            'thick': 3,
            'highlighted': 2,
            'default': 1
        };
        let highestPriority = 0;
        let resultStyle = EDGE_STYLES.DEFAULT;
        for (const edge of edges) {
            const priority = stylePriority[edge.style] || 1;
            if (priority > highestPriority) {
                highestPriority = priority;
                resultStyle = edge.style;
            }
        }
        return resultStyle;
    }
}
//# sourceMappingURL=ContainerCollapseExpand.js.map