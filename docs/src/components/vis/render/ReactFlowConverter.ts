/**
 * @fileoverview ReactFlow Data Converter
 * 
 * Converts positioned layout data to ReactFlow-compatible format with strong typing.
 */

import { LayoutResult } from '../layout/types.js';
import { Node, Edge } from '@xyflow/react';
import { 
  TypedReactFlowData, 
  TypedContainerNode, 
  TypedStandardNode, 
  TypedReactFlowEdge,
  ContainerNodeData,
  StandardNodeData,
  validateReactFlowData,
  isValidELKContainer
} from './types.js';
import { validateELKResult, validateReactFlowResult, logValidationReport } from './validation.js';

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

    // Convert nodes with proper parent relationships and strong typing
    layoutResult.nodes.forEach(node => {
      const parentId = parentMap.get(node.id);
      
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

    // Convert containers with strong typing and validation
    layoutResult.containers.forEach(container => {
      if (!isValidELKContainer(container)) {
        console.warn(`[ReactFlowConverter] ⚠️ Invalid container ${(container as any).id}: missing required dimensions`);
        return;
      }
      
      const parentId = parentMap.get(container.id);
      
      console.log(`[ReactFlowConverter] 📦 Converting container ${container.id}: ${container.width}x${container.height}`);
      
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

    // Convert edges with strong typing
    layoutResult.edges.forEach(edge => {
      const typedEdge: TypedReactFlowEdge = {
        id: edge.id,
        type: 'standard',
        source: edge.source,
        target: edge.target,
        data: {
          style: edge.style || 'default'
        }
      };
      edges.push(typedEdge);
    });

    // Convert hyperEdges with strong typing
    layoutResult.hyperEdges.forEach(hyperEdge => {
      const typedHyperEdge: TypedReactFlowEdge = {
        id: hyperEdge.id,
        type: 'hyper',
        source: hyperEdge.source,
        target: hyperEdge.target,
        data: {
          style: hyperEdge.style || 'default'
        }
      };
      edges.push(typedHyperEdge);
    });

    const result: TypedReactFlowData = { nodes, edges };
    
    // Validate the result before returning
    const reactFlowReport = validateReactFlowResult(result);
    logValidationReport(reactFlowReport, 'ReactFlow Output');
    
    if (!reactFlowReport.isValid) {
      console.error('[ReactFlowConverter] ❌ Generated invalid ReactFlow data - container dimensions may be missing');
    } else {
      console.log('[ReactFlowConverter] ✅ Generated valid typed ReactFlow data');
    }

    return result;
  }
}
