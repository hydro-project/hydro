/**
 * @fileoverview ReactFlow Data Converter
 * 
 * Converts positioned layout data to ReactFlow-compatible format with strong typing.
 */

import { LayoutResult } from '../layout/types';
import { Node, Edge, MarkerType } from '@xyflow/react';
import { 
  TypedReactFlowData, 
  TypedContainerNode, 
  TypedStandardNode, 
  TypedReactFlowEdge,
  ContainerNodeData,
  StandardNodeData,
  validateReactFlowData,
  isValidELKContainer
} from './types';
import { validateELKResult, validateReactFlowResult, logValidationReport } from './validation';

export class ReactFlowConverter {
  static convert(layoutResult: LayoutResult): TypedReactFlowData {
    // Validate input from ELK
    const elkReport = validateELKResult(layoutResult);
    logValidationReport(elkReport, 'ELK Input');
    
    if (!elkReport.isValid) {
      console.error('[ReactFlowConverter] ❌ Invalid ELK input detected, proceeding with caution...');
    }

    const nodes: (TypedStandardNode | TypedContainerNode)[] = [];
    const edges: TypedReactFlowEdge[] = [];

    // Create a map to track parent-child relationships
    const parentMap = new Map<string, string>();
    
    // First, build parent relationships from containers
    layoutResult.containers.forEach(container => {
      if (container.children) {
        // Convert Set to array if needed
        const childrenArray = Array.from(container.children);
        childrenArray.forEach(childId => {
          parentMap.set(childId, container.id);
        });
      }
    });

    // CRITICAL: Add containers FIRST so they appear before their children in the nodes array
    // Convert containers with strong typing and validation
    layoutResult.containers.forEach(container => {
      if (!isValidELKContainer(container)) {
        console.warn(`[ReactFlowConverter] ⚠️ Invalid container ${(container as any).id}: missing required dimensions`);
        return;
      }
      
      const parentId = parentMap.get(container.id);
      
      const containerNodeData: ContainerNodeData = {
        label: container.id,
        collapsed: container.collapsed || false,
        style: 'default',
        // CRITICAL: Pass ELK-calculated dimensions in data
        width: container.width,
        height: container.height,
      };
      
      const containerNode: TypedContainerNode = {
        id: container.id,
        type: 'container',
        position: { x: container.x || 0, y: container.y || 0 },
        data: containerNodeData,
        // Set explicit dimensions from ELK in style as well for ReactFlow
        style: {
          width: container.width,
          height: container.height,
        },
        // Containers can also have parents (nested containers)
        parentId: parentId,
        extent: parentId ? 'parent' : undefined,
      };
      
      nodes.push(containerNode);
    });

    // Convert nodes with proper parent relationships and strong typing
    layoutResult.nodes.forEach(node => {
      const parentId = parentMap.get(node.id);
      
            // Only log detailed node conversion in debug mode
      if (process.env.NODE_ENV === 'development') {
        // Simplified node conversion logging
      }
      
      const standardNodeData: StandardNodeData = {
        label: node.label || node.id,
        style: node.style || 'default',
        nodeType: (node as any).nodeType,
        // Pass through any additional custom properties
        ...Object.fromEntries(
          Object.entries(node as any).filter(([key]) => 
            !['id', 'label', 'style', 'x', 'y', 'width', 'height', 'hidden'].includes(key)
          )
        )
      };
      
      const standardNode: TypedStandardNode = {
        id: node.id,
        type: 'standard',
        position: { x: node.x || 0, y: node.y || 0 },
        data: standardNodeData,
        // CRITICAL: Set parent relationship for ReactFlow hierarchical layout
        parentId: parentId,
        extent: parentId ? 'parent' : undefined,
      };
      
      nodes.push(standardNode);
    });

    // Convert edges with strong typing
    layoutResult.edges.forEach(edge => {
      const typedEdge: TypedReactFlowEdge = {
        id: edge.id,
        type: 'standard',
        source: edge.source,
        target: edge.target,
        // Let ReactFlow use default handle IDs (no explicit sourceHandle/targetHandle)
        markerEnd: {
          type: MarkerType.ArrowClosed,
          width: 20,
          height: 20
        },
        data: {
          style: edge.style || 'default'
        }
      };
      
      // Debug logging for edge arrowheads
      console.log(`🏹 [EDGE DEBUG] Created edge ${edge.id}:`, {
        id: typedEdge.id,
        type: typedEdge.type,
        source: typedEdge.source,
        target: typedEdge.target,
        hasMarkerEnd: !!typedEdge.markerEnd,
        markerEnd: typedEdge.markerEnd
      });
      
      edges.push(typedEdge);
    });

    // Convert hyperEdges with strong typing
    layoutResult.hyperEdges.forEach(hyperEdge => {
      const typedHyperEdge: TypedReactFlowEdge = {
        id: hyperEdge.id,
        type: 'hyper',
        source: hyperEdge.source,
        target: hyperEdge.target,
        // Let ReactFlow use default handle IDs for flexibility
        markerEnd: {
          type: MarkerType.ArrowClosed,
          width: 20,
          height: 20
        },
        data: {
          style: hyperEdge.style || 'default'
        }
      };
      
      // Debug logging for hyper edge arrowheads
      console.log(`🏹 [HYPER EDGE DEBUG] Created hyper edge ${hyperEdge.id}:`, {
        id: typedHyperEdge.id,
        type: typedHyperEdge.type,
        source: typedHyperEdge.source,
        target: typedHyperEdge.target,
        hasMarkerEnd: !!typedHyperEdge.markerEnd,
        markerEnd: typedHyperEdge.markerEnd
      });
      
      edges.push(typedHyperEdge);
    });

    const result: TypedReactFlowData = { nodes, edges };
    
    // Debug logging for final ReactFlow data
    console.log(`🎯 [REACTFLOW DEBUG] Final ReactFlow data:`, {
      nodeCount: nodes.length,
      edgeCount: edges.length,
      edgesWithMarkers: edges.filter(e => e.markerEnd).length,
      sampleEdge: edges[0] // Log first edge as sample
    });
    
    // Validate the result before returning
    const reactFlowReport = validateReactFlowResult(result);
    logValidationReport(reactFlowReport, 'ReactFlow Output');
    
    if (!reactFlowReport.isValid) {
      console.error('[ReactFlowConverter] ❌ Generated invalid ReactFlow data - container dimensions may be missing');
    } else if (process.env.NODE_ENV === 'development') {
      console.log('[ReactFlowConverter] ✅ Generated valid typed ReactFlow data');
    }

    return result;
  }
}
