/**
 * Container Collapse/Expand Engine
 *
 * Clean implementation using hierarchical lineage tracking approach.
 * Each hyperedge stores the original leaf endpoints and we use dynamic
 * ancestor lookup to find the current visible state of any node.
 */
import { EDGE_STYLES } from '../shared/constants';
import { SIZES } from '../shared/config';
// Constants
const HYPER_EDGE_PREFIX = 'hyper_';
export class ContainerCollapseExpandEngine {
    constructor(visualizationState) {
        this.state = visualizationState;
    }
    /**
     * Find the current visible ancestor of a node by walking up the hierarchy
     */
    _findCurrentVisibleAncestor(nodeId) {
        let current = nodeId;
        while (current) {
            // If it's a visible node, return it
            const node = this.state.getGraphNode(current);
            if (node && !node.hidden) {
                return current;
            }
            // If it's a collapsed container, return it
            const container = this.state.getContainer(current);
            if (container && container.collapsed) {
                return current;
            }
            // Move up to parent container
            current = this.state.getParentContainer(current) || '';
        }
        // Fallback: return original nodeId
        return nodeId;
    }
    // ============ Public API ============
    /**
     * Collapse a container: hide children and lift their edges to container level
     */
    collapseContainer(containerId) {
        const container = this.state.getContainer(containerId);
        // Validate container exists
        if (!container) {
            throw new Error(`Cannot collapse container: container '${containerId}' does not exist`);
        }
        if (container.collapsed)
            return; // Already collapsed
        console.log(`[COLLAPSE] Starting collapse of container ${containerId}`);
        // 1. First, recursively collapse any child containers (bottom-up)
        this._collapseChildContainers(containerId);
        // 2. Find all edges that cross the container boundary BEFORE hiding children
        const crossingEdges = this._findCrossingEdges(containerId);
        console.log(`[COLLAPSE] Found ${crossingEdges.length} crossing edges`);
        // 3. Mark container as collapsed and hide children
        container.collapsed = true;
        // 3b. Set collapsed container dimensions
        this.state.setContainerLayout(containerId, {
            dimensions: {
                width: SIZES.COLLAPSED_CONTAINER_WIDTH,
                height: SIZES.COLLAPSED_CONTAINER_HEIGHT
            }
        });
        this._hideContainerChildren(containerId);
        // 4. Group crossing edges by external endpoint (using dynamic visible ancestor lookup)
        const edgeGroups = this._groupEdgesByCurrentExternalEndpoint(crossingEdges, containerId);
        console.log(`[COLLAPSE] Grouped into ${edgeGroups.size} external endpoints`);
        // 6. Create hyperedges for each group
        this._createHyperedgesFromGroups(containerId, edgeGroups);
        // 7. Hide the original crossing edges
        this._hideCrossingEdges(crossingEdges);
        console.log(`[COLLAPSE] Completed collapse of container ${containerId}`);
    }
    /**
     * Expand a container: show children and ground hyperedges back to edges
     */
    expandContainer(containerId) {
        const container = this.state.getContainer(containerId);
        // Validate container exists
        if (!container) {
            throw new Error(`Cannot expand container: container '${containerId}' does not exist`);
        }
        if (!container.collapsed)
            return; // Already expanded
        console.log(`[EXPAND] Starting expansion of container ${containerId}`);
        // 1. Mark container as expanded and show children
        container.collapsed = false;
        this._showContainerChildren(containerId);
        // 2. Find all hyperedges connected to this container
        const containerHyperedges = this._findContainerHyperedges(containerId);
        console.log(`[EXPAND] Found ${containerHyperedges.length} hyperedges to process`);
        // 4. Process each hyperedge using the updated leaf mapping
        containerHyperedges.forEach(hyperEdge => {
            this._groundHyperedgeWithLeafMapping(hyperEdge, containerId);
        });
        // 5. Then recursively expand any child containers (top-down)
        this._expandChildContainers(containerId);
        console.log(`[EXPAND] Completed expansion of container ${containerId}`);
    }
    // ============ Container State Management ============
    _collapseChildContainers(containerId) {
        const children = this.state.getContainerChildren(containerId);
        for (const childId of children) {
            const childContainer = this.state.getContainer(childId);
            if (childContainer) {
                this.collapseContainer(childId); // Recursive collapse
            }
        }
    }
    _expandChildContainers(containerId) {
        const children = this.state.getContainerChildren(containerId);
        for (const childId of children) {
            const childContainer = this.state.getContainer(childId);
            if (childContainer) {
                this.expandContainer(childId); // Recursive expand
            }
        }
    }
    _hideContainerChildren(containerId) {
        const children = this.state.getContainerChildren(containerId);
        for (const childId of children) {
            const node = this.state.getGraphNode(childId);
            if (node) {
                this.state.updateNode(childId, { hidden: true });
            }
        }
    }
    _showContainerChildren(containerId) {
        const children = this.state.getContainerChildren(containerId);
        for (const childId of children) {
            const node = this.state.getGraphNode(childId);
            if (node) {
                this.state.updateNode(childId, { hidden: false });
            }
        }
    }
    _hideCrossingEdges(crossingEdges) {
        crossingEdges.forEach(edge => {
            // Only hide if it's a regular edge (not a hyperedge)
            if (this.state.getGraphEdge(edge.id)) {
                this.state.updateEdge(edge.id, { hidden: true });
            }
            else if (this.state.getHyperEdge(edge.id)) {
                // For hyperedges, mark as hidden directly
                edge.hidden = true;
            }
        });
    }
    // ============ Edge Discovery ============
    /**
     * Find all edges that cross the container boundary
     */
    _findCrossingEdges(containerId) {
        const children = this.state.getContainerChildren(containerId);
        const crossingEdges = [];
        // Check all edges to see if they cross the container boundary
        for (const [edgeId, edge] of this.state.graphEdges) {
            if (edge.hidden)
                continue; // Skip already hidden edges
            const sourceInContainer = children.has(edge.source);
            const targetInContainer = children.has(edge.target);
            // Edge crosses boundary if exactly one endpoint is in container
            if (sourceInContainer !== targetInContainer) {
                crossingEdges.push(edge);
            }
        }
        // Also check hyperedges (in case we're collapsing a container that contains other collapsed containers)
        for (const [hyperEdgeId, hyperEdge] of this.state.hyperEdges) {
            if (hyperEdge.hidden)
                continue;
            const sourceInContainer = children.has(hyperEdge.source);
            const targetInContainer = children.has(hyperEdge.target);
            if (sourceInContainer !== targetInContainer) {
                crossingEdges.push(hyperEdge);
            }
        }
        return crossingEdges;
    }
    /**
     * Group crossing edges by their current external endpoint (using leaf mapping)
     */
    _groupEdgesByCurrentExternalEndpoint(crossingEdges, containerId) {
        const children = this.state.getContainerChildren(containerId);
        const groups = new Map();
        for (const edge of crossingEdges) {
            const sourceInContainer = children.has(edge.source);
            const internalEndpoint = sourceInContainer ? edge.source : edge.target;
            const originalExternalEndpoint = sourceInContainer ? edge.target : edge.source;
            // KEY CHANGE: Use dynamic lookup to find current visible ancestor
            const currentExternalEndpoint = this._findCurrentVisibleAncestor(originalExternalEndpoint);
            const isOutgoing = sourceInContainer; // container -> external
            if (!groups.has(currentExternalEndpoint)) {
                groups.set(currentExternalEndpoint, { incoming: [], outgoing: [] });
            }
            const group = groups.get(currentExternalEndpoint);
            if (isOutgoing) {
                group.outgoing.push(edge);
            }
            else {
                group.incoming.push(edge);
            }
        }
        return groups;
    }
    // ============ Hyperedge Creation ============
    _createHyperedgesFromGroups(containerId, edgeGroups) {
        for (const [externalEndpoint, group] of edgeGroups) {
            // Create hyperedge for incoming connections (external -> container)
            if (group.incoming.length > 0) {
                this._createHyperedge(externalEndpoint, containerId, group.incoming);
            }
            // Create hyperedge for outgoing connections (container -> external)
            if (group.outgoing.length > 0) {
                this._createHyperedge(containerId, externalEndpoint, group.outgoing);
            }
        }
    }
    _createHyperedge(sourceId, targetId, edges) {
        const hyperEdgeId = `${HYPER_EDGE_PREFIX}${sourceId}_to_${targetId}`;
        // SIMPLE APPROACH: Store the original endpoints of each aggregated edge
        const originalEndpoints = new Map();
        for (const edge of edges) {
            originalEndpoints.set(edge.id, {
                source: edge.source, // Original leaf source
                target: edge.target // Original leaf target
            });
        }
        console.log(`[CREATE] Creating hyperedge ${hyperEdgeId} aggregating ${originalEndpoints.size} edges`);
        this.state.setHyperEdge(hyperEdgeId, {
            source: sourceId,
            target: targetId,
            style: this._aggregateStyles(edges),
            originalEndpoints: originalEndpoints, // Map<edgeId, {source: leafId, target: leafId}>
            hidden: false
        });
    }
    _isContainerEndpoint(endpointId) {
        return this.state.getContainer(endpointId) !== undefined;
    }
    // ============ Hyperedge Grounding ============
    _findContainerHyperedges(containerId) {
        const containerHyperedges = [];
        for (const [hyperEdgeId, hyperEdge] of this.state.hyperEdges) {
            if (hyperEdge.hidden)
                continue;
            // Find hyperedges where this container is an endpoint
            if (hyperEdge.source === containerId || hyperEdge.target === containerId) {
                containerHyperedges.push(hyperEdge);
            }
        }
        return containerHyperedges;
    }
    /**
     * Ground a hyperedge: convert it back to edges using current visible ancestors
     */
    _groundHyperedgeWithLeafMapping(hyperEdge, expandingContainerId) {
        console.log(`[GROUND] Grounding hyperedge ${hyperEdge.id}: ${hyperEdge.source} → ${hyperEdge.target}`);
        console.log(`[GROUND] HyperEdge has originalEndpoints:`, !!hyperEdge.originalEndpoints);
        console.log(`[GROUND] OriginalEndpoints size:`, hyperEdge.originalEndpoints?.size || 0);
        if (!hyperEdge.originalEndpoints) {
            console.log(`[GROUND] Warning: hyperedge ${hyperEdge.id} has no originalEndpoints`);
            hyperEdge.hidden = true;
            return;
        }
        let anyEdgeProcessed = false;
        // For each original edge that was aggregated into this hyperedge
        for (const [edgeId, originalEndpoint] of hyperEdge.originalEndpoints) {
            console.log(`[GROUND] Processing edge ${edgeId} with original endpoints:`, originalEndpoint);
            // Try to find the original edge (could be regular edge or hyperedge)
            const originalEdge = this.state.getGraphEdge(edgeId);
            const originalHyperEdge = this.state.getHyperEdge(edgeId);
            if (!originalEdge && !originalHyperEdge) {
                console.log(`[GROUND] Warning: original edge/hyperedge ${edgeId} not found in state`);
                continue;
            }
            // If this was originally a hyperedge, we need to recursively process it
            if (originalHyperEdge) {
                console.log(`[GROUND] Found original hyperedge ${edgeId}, recursively processing its originalEndpoints`);
                // Recursively process the original hyperedge's endpoints
                if (originalHyperEdge.originalEndpoints) {
                    for (const [subEdgeId, subOriginalEndpoint] of originalHyperEdge.originalEndpoints) {
                        this._processOriginalEndpoint(subEdgeId, subOriginalEndpoint);
                    }
                }
                anyEdgeProcessed = true;
                continue;
            }
            // Process regular edge
            this._processOriginalEndpoint(edgeId, originalEndpoint);
            anyEdgeProcessed = true;
        }
        // Mark the parent hyperedge as processed
        hyperEdge.hidden = true;
        console.log(`[GROUND] Marked hyperedge ${hyperEdge.id} as hidden. Processed any edges: ${anyEdgeProcessed}`);
    }
    _processOriginalEndpoint(edgeId, originalEndpoint) {
        const originalEdge = this.state.getGraphEdge(edgeId);
        if (!originalEdge) {
            console.log(`[GROUND] Warning: cannot process edge ${edgeId} - not found in graphEdges`);
            return;
        }
        // Find where each original endpoint is currently visible
        const currentSource = this._findCurrentVisibleAncestor(originalEndpoint.source);
        const currentTarget = this._findCurrentVisibleAncestor(originalEndpoint.target);
        console.log(`[GROUND] Edge ${edgeId}: ${originalEndpoint.source}→${originalEndpoint.target} now ${currentSource}→${currentTarget}`);
        console.log(`[GROUND] Edge ${edgeId} current state: hidden=${originalEdge.hidden}`);
        // If both endpoints are visible leaf nodes, restore the original edge
        if (this._areBothVisibleLeafNodes(currentSource, currentTarget)) {
            console.log(`[GROUND] Restoring original edge ${edgeId}`);
            this.state.updateEdge(edgeId, { hidden: false });
        }
        else {
            // At least one endpoint is a container - create intermediate hyperedge if valid
            if (this._shouldCreateIntermediateHyperedge(currentSource, currentTarget)) {
                console.log(`[GROUND] Creating intermediate hyperedge for edge ${edgeId}: ${currentSource} → ${currentTarget}`);
                this._createIntermediateHyperedge(edgeId, currentSource, currentTarget, originalEndpoint, originalEdge.style);
            }
            else {
                console.log(`[GROUND] Leaving edge ${edgeId} hidden - will be restored when both endpoints are visible`);
                console.log(`[GROUND] - currentSource ${currentSource} is visible leaf: ${this._areBothVisibleLeafNodes(currentSource, currentSource)}`);
                console.log(`[GROUND] - currentTarget ${currentTarget} is visible leaf: ${this._areBothVisibleLeafNodes(currentTarget, currentTarget)}`);
                console.log(`[GROUND] - shouldCreateIntermediateHyperedge returned: ${this._shouldCreateIntermediateHyperedge(currentSource, currentTarget)}`);
            }
        }
    }
    _areBothVisibleLeafNodes(sourceId, targetId) {
        const sourceNode = this.state.getGraphNode(sourceId);
        const targetNode = this.state.getGraphNode(targetId);
        return sourceNode && !sourceNode.hidden && targetNode && !targetNode.hidden;
    }
    _shouldCreateIntermediateHyperedge(sourceId, targetId) {
        // Only create hyperedge if at least one endpoint is a collapsed container
        const sourceContainer = this.state.getContainer(sourceId);
        const targetContainer = this.state.getContainer(targetId);
        const sourceIsCollapsed = sourceContainer && sourceContainer.collapsed;
        const targetIsCollapsed = targetContainer && targetContainer.collapsed;
        return sourceIsCollapsed || targetIsCollapsed;
    }
    _createIntermediateHyperedge(edgeId, currentSource, currentTarget, originalEndpoint, style) {
        const newHyperEdgeId = `${HYPER_EDGE_PREFIX}${currentSource}_to_${currentTarget}`;
        console.log(`[GROUND] Creating intermediate hyperedge: ${newHyperEdgeId}`);
        // Check if hyperedge already exists and merge if so
        const existingHyperEdge = this.state.getHyperEdge(newHyperEdgeId);
        if (existingHyperEdge) {
            // Add to existing hyperedge AND ensure it's visible
            console.log(`[GROUND] Merging into existing hyperedge ${newHyperEdgeId}, current size: ${existingHyperEdge.originalEndpoints.size}, hidden: ${existingHyperEdge.hidden}`);
            existingHyperEdge.originalEndpoints.set(edgeId, originalEndpoint);
            existingHyperEdge.hidden = false; // CRITICAL FIX: Ensure it's visible
            console.log(`[GROUND] After merge, size: ${existingHyperEdge.originalEndpoints.size}, hidden: ${existingHyperEdge.hidden}`);
        }
        else {
            // Create new hyperedge
            console.log(`[GROUND] Creating new hyperedge ${newHyperEdgeId}`);
            const newOriginalEndpoints = new Map();
            newOriginalEndpoints.set(edgeId, originalEndpoint);
            this.state.setHyperEdge(newHyperEdgeId, {
                source: currentSource,
                target: currentTarget,
                style: style,
                originalEndpoints: newOriginalEndpoints,
                hidden: false
            });
        }
    }
    // ============ Utility Methods ============
    _aggregateStyles(edges) {
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
    // ============ Validation and Edge Cases ============
    validateTreeHierarchy(parentId, childId) {
        // Check for self-reference
        if (parentId === childId) {
            throw new Error(`Cannot add container '${childId}' as child of itself`);
        }
        // Check if child would create a cycle
        if (this._wouldCreateCycle(parentId, childId)) {
            throw new Error(`Adding '${childId}' to '${parentId}' would create a cycle in container hierarchy`);
        }
    }
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
            const parent = this.state.getParentContainer(currentId);
            if (parent) {
                return dfs(parent);
            }
            return false;
        };
        return dfs(parentId);
    }
    rebuildEdgeIndex() {
        // Placeholder for compatibility - V2 doesn't use edge indexing
    }
}
//# sourceMappingURL=ContainerCollapseExpand.js.map