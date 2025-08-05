/**
 * @fileoverview ReactFlow Bridge - Converts VisualizationState to ReactFlow format
 *
 * This bridge converts VisualizationState to ReactFlow's expected data structures.
 * ReactFlow only sees unified edges (hyperedges are included transparently).
 * Uses configurable handle system for maximum layout flexibility.
 */
import { CoordinateTranslator } from './CoordinateTranslator';
import { MarkerType } from '@xyflow/react';
import { getHandleConfig } from '../render/handleConfig';
export class ReactFlowBridge {
    /**
     * Convert positioned VisState data to ReactFlow format
     * Pure data transformation - no layout logic
     */
    visStateToReactFlow(visState) {
        console.log('[ReactFlowBridge] 🔄 Converting VisState to ReactFlow format');
        const nodes = [];
        const edges = [];
        // Create parent-child mapping for hierarchical layout
        const parentMap = this.buildParentMap(visState);
        // Convert containers to ReactFlow nodes
        this.convertContainers(visState, nodes, parentMap);
        // Convert regular nodes to ReactFlow nodes  
        this.convertNodes(visState, nodes, parentMap);
        // Convert all edges to ReactFlow edges (now includes hyperedges transparently)
        this.convertEdges(visState, edges);
        return { nodes, edges };
    }
    /**
     * Build parent-child relationship map
     */
    buildParentMap(visState) {
        const parentMap = new Map();
        // Map nodes to their parent containers
        visState.visibleContainers.forEach(container => {
            if (!container.collapsed) {
                // Only expanded containers can have children in ReactFlow
                container.children.forEach(childId => {
                    parentMap.set(childId, container.id);
                });
            }
        });
        return parentMap;
    }
    /**
     * Convert containers to ReactFlow container nodes
     */
    convertContainers(visState, nodes, parentMap) {
        visState.visibleContainers.forEach(container => {
            const parentId = parentMap.get(container.id);
            // Get coordinates from VisState computed view (already canonical coordinates)
            const elkCoords = {
                x: container.x || 0,
                y: container.y || 0
            };
            // Convert ELK coordinates to ReactFlow coordinates
            const parentContainer = parentId ?
                CoordinateTranslator.getContainerInfo(parentId, visState) :
                undefined;
            const position = CoordinateTranslator.elkToReactFlow(elkCoords, parentContainer);
            // Use computed dimensions from VisState (includes ELK-calculated sizes via expandedDimensions)
            const width = container.width || (container.collapsed ? 200 : 400);
            const height = container.height || (container.collapsed ? 60 : 300);
            console.log(`[ReactFlowBridge] 📦 Container ${container.id}: collapsed=${container.collapsed}, ELK=(${elkCoords.x}, ${elkCoords.y}), ReactFlow=(${position.x}, ${position.y}), size=${width}x${height}`);
            const containerNode = {
                id: container.id,
                type: 'container',
                position,
                data: {
                    label: container.data?.label || container.label || container.id, // Use proper label like InfoPanel
                    style: container.style || 'default',
                    collapsed: container.collapsed || false,
                    width,
                    height,
                    // Pass through any custom properties
                    ...this.extractCustomProperties(container)
                },
                style: {
                    width,
                    height
                },
                parentId
                // FIX: Remove extent: 'parent' - causes ReactFlow drag coordinate issues
                // extent: parentId ? 'parent' : undefined
            };
            nodes.push(containerNode);
        });
    }
    /**
     * Convert regular nodes to ReactFlow standard nodes
     */
    convertNodes(visState, nodes, parentMap) {
        visState.visibleNodes.forEach(node => {
            const parentId = parentMap.get(node.id);
            // Get ELK coordinates from node layout (where ELKBridge stores them)
            const nodeLayout = visState.getNodeLayout(node.id);
            console.log(`[ReactFlowBridge] 🔍 Node ${node.id} layout:`, nodeLayout, 'raw node coords:', { x: node.x, y: node.y });
            const elkCoords = {
                x: nodeLayout?.position?.x || node.x || 0,
                y: nodeLayout?.position?.y || node.y || 0
            };
            // Convert ELK coordinates to ReactFlow coordinates
            const parentContainer = parentId ?
                CoordinateTranslator.getContainerInfo(parentId, visState) :
                undefined;
            if (parentContainer) {
                console.log(`[ReactFlowBridge] 🔍 Parent container ${parentId} info:`, {
                    id: parentContainer.id,
                    x: parentContainer.x,
                    y: parentContainer.y,
                    width: parentContainer.width,
                    height: parentContainer.height
                });
            }
            const position = CoordinateTranslator.elkToReactFlow(elkCoords, parentContainer);
            console.log(`[ReactFlowBridge] 🔘 Node ${node.id}: parent=${parentId || 'none'}, ELK=(${elkCoords.x}, ${elkCoords.y}), ReactFlow=(${position.x}, ${position.y})`);
            const standardNode = {
                id: node.id,
                type: 'standard',
                position,
                data: {
                    label: node.label || node.id,
                    style: node.style || 'default',
                    // Pass through any custom properties
                    ...this.extractCustomProperties(node)
                },
                parentId
                // FIX: Remove extent: 'parent' - causes ReactFlow drag coordinate issues
                // extent: parentId ? 'parent' : undefined
            };
            nodes.push(standardNode);
        });
    }
    /**
     * Convert regular edges to ReactFlow edges
     */
    convertEdges(visState, edges) {
        visState.visibleEdges.forEach(edge => {
            // Debug: log the actual edge data to see what we're getting
            console.log(`[ReactFlowBridge] 🔍 Debug edge ${edge.id}:`, {
                source: edge.source,
                target: edge.target,
                sourceHandle: edge.sourceHandle,
                targetHandle: edge.targetHandle,
                sourceHandleType: typeof edge.sourceHandle,
                targetHandleType: typeof edge.targetHandle
            });
            const handleConfig = getHandleConfig();
            const reactFlowEdge = {
                id: edge.id,
                type: 'standard',
                source: edge.source,
                target: edge.target,
                markerEnd: {
                    type: MarkerType.ArrowClosed,
                    width: 15,
                    height: 15,
                    color: '#999'
                },
                data: {
                    style: edge.style || 'default'
                }
            };
            // Only add handle properties for discrete handle strategy
            if (!handleConfig.enableContinuousHandles) {
                // For discrete handles, use specified handles or defaults
                reactFlowEdge.sourceHandle = edge.sourceHandle || 'default-out';
                reactFlowEdge.targetHandle = edge.targetHandle || 'default-in';
            }
            // For continuous handles, omit sourceHandle/targetHandle to let ReactFlow auto-connect
            console.log(`[ReactFlowBridge] ✅ Created ReactFlow edge ${edge.id}:`, reactFlowEdge);
            edges.push(reactFlowEdge);
        });
    }
    /**
     * Extract custom properties from graph elements
     */
    extractCustomProperties(element) {
        const customProps = {};
        // Filter out known properties to get custom ones
        const knownProps = new Set([
            'id', 'label', 'style', 'hidden', 'layout',
            'source', 'target', 'children', 'collapsed',
            'x', 'y', 'width', 'height'
        ]);
        Object.entries(element).forEach(([key, value]) => {
            if (!knownProps.has(key)) {
                customProps[key] = value;
            }
        });
        return customProps;
    }
}
//# sourceMappingURL=ReactFlowBridge.js.map