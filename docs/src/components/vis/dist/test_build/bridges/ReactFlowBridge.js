/**
 * @fileoverview ReactFlow Bridge - Clean interface between VisState and ReactFlow
 *
 * This bridge implements the core principle:
 * - VisState contains ALL positioned data after ELK layout
 * - ReactFlow gets locations, dimensions, edges, and display metadata
 * - No business logic in conversion - pure data transformation
 */
import { MarkerType } from '@xyflow/react';
import { CoordinateTranslator } from './CoordinateTranslator';
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
        // Convert regular edges to ReactFlow edges
        this.convertEdges(visState, edges);
        // Convert hyperedges to ReactFlow edges
        this.convertHyperEdges(visState, edges);
        console.log('[ReactFlowBridge] ✅ Conversion complete:', {
            nodes: nodes.length,
            edges: edges.length,
            containers: nodes.filter(n => n.type === 'container').length,
            hyperEdges: edges.filter(e => e.type === 'hyper').length
        });
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
            // Get ELK coordinates from VisState (canonical coordinates)
            const elkCoords = {
                x: container.layout?.position?.x || 0,
                y: container.layout?.position?.y || 0
            };
            // Convert ELK coordinates to ReactFlow coordinates
            const parentContainer = parentId ?
                CoordinateTranslator.getContainerInfo(parentId, visState) :
                undefined;
            const position = CoordinateTranslator.elkToReactFlow(elkCoords, parentContainer);
            const width = container.layout?.dimensions?.width || (container.collapsed ? 200 : 400);
            const height = container.layout?.dimensions?.height || (container.collapsed ? 60 : 300);
            console.log(`[ReactFlowBridge] 📦 Container ${container.id}: collapsed=${container.collapsed}, ELK=(${elkCoords.x}, ${elkCoords.y}), ReactFlow=(${position.x}, ${position.y}), size=${width}x${height}`);
            const containerNode = {
                id: container.id,
                type: 'container',
                position,
                data: {
                    label: container.id,
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
                parentId,
                extent: parentId ? 'parent' : undefined
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
            // Get ELK coordinates from node (canonical coordinates)
            const elkCoords = {
                x: node.x || 0,
                y: node.y || 0
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
                parentId,
                extent: parentId ? 'parent' : undefined
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
            // NEVER include sourceHandle/targetHandle if they are undefined, null, or string "null"
            // Let ReactFlow use its defaults
            if (edge.sourceHandle !== undefined && edge.sourceHandle !== null && edge.sourceHandle !== 'null') {
                reactFlowEdge.sourceHandle = edge.sourceHandle;
            }
            if (edge.targetHandle !== undefined && edge.targetHandle !== null && edge.targetHandle !== 'null') {
                reactFlowEdge.targetHandle = edge.targetHandle;
            }
            console.log(`[ReactFlowBridge] ✅ Created ReactFlow edge ${edge.id}:`, reactFlowEdge);
            edges.push(reactFlowEdge);
        });
    }
    /**
     * Convert hyperedges to ReactFlow edges
     */
    convertHyperEdges(visState, edges) {
        visState.allHyperEdges.forEach(hyperEdge => {
            console.log(`[ReactFlowBridge] 🔥 Converting hyperedge ${hyperEdge.id}: ${hyperEdge.source} → ${hyperEdge.target}`);
            const reactFlowHyperEdge = {
                id: hyperEdge.id,
                type: 'hyper',
                source: hyperEdge.source,
                target: hyperEdge.target,
                markerEnd: {
                    type: MarkerType.ArrowClosed,
                    width: 15,
                    height: 15,
                    color: '#999'
                },
                data: {
                    style: hyperEdge.style || 'default'
                }
            };
            // NEVER include sourceHandle/targetHandle if they are undefined, null, or string "null"
            // Let ReactFlow use its defaults
            if (hyperEdge.sourceHandle !== undefined && hyperEdge.sourceHandle !== null && hyperEdge.sourceHandle !== 'null') {
                reactFlowHyperEdge.sourceHandle = hyperEdge.sourceHandle;
            }
            if (hyperEdge.targetHandle !== undefined && hyperEdge.targetHandle !== null && hyperEdge.targetHandle !== 'null') {
                reactFlowHyperEdge.targetHandle = hyperEdge.targetHandle;
            }
            edges.push(reactFlowHyperEdge);
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