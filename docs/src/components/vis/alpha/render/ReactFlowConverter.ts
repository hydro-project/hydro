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
      const isCollapsed = container.collapsed || false;
      
      // � PASS-THROUGH DIMENSIONS: Use whatever dimensions VisState/ELK provided
      // ReactFlowConverter should NOT make dimension decisions - that's VisState's job
      const width = container.width;
      const height = container.height;
      
      console.log(`[ReactFlowConverter] 📦 CONTAINER ${container.id}: collapsed=${isCollapsed}, dims=${width}x${height} (using dimensions from VisState/ELK)`);
      
      const containerNodeData: ContainerNodeData = {
        label: container.id,
        collapsed: isCollapsed,
        style: 'default',
        width: width,
        height: height,
      };
      
      const containerNode: TypedContainerNode = {
        id: container.id,
        type: 'container',
        position: { x: container.x || 0, y: container.y || 0 },
        data: containerNodeData,
        // Pass through the dimensions that VisState determined are correct
        style: {
          width: width,
          height: height,
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
      
      // 🔥 PARENT RELATIONSHIP LOGGING
      console.log(`[ReactFlowConverter] 🔗 NODE PARENT: ${node.id} parent=${parentId || 'none'}`);
      
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
          width: 15,
          height: 15,
          color: '#999'
        },
        data: {
          style: edge.style || 'default'
        }
      };
      
      edges.push(typedEdge);
    });

    // Convert hyperEdges with strong typing
    layoutResult.hyperEdges.forEach(hyperEdge => {
      console.log(`[ReactFlowConverter] 🔥 CONVERTING HYPEREDGE ${hyperEdge.id}:`);
      console.log(`  Source: ${hyperEdge.source}, Target: ${hyperEdge.target}`);
      
      // Find the source and target nodes in our converted nodes to verify positions
      const sourceNode = nodes.find(n => n.id === hyperEdge.source);
      const targetNode = nodes.find(n => n.id === hyperEdge.target);
      
      if (sourceNode && targetNode) {
        console.log(`  Source position: (${sourceNode.position.x}, ${sourceNode.position.y})`);
        console.log(`  Target position: (${targetNode.position.x}, ${targetNode.position.y})`);
        
        // 🔥 HANDLE AND CONNECTION ANALYSIS
        console.log(`  🔗 SOURCE NODE: ${sourceNode.id} type=${sourceNode.type}`);
        if (sourceNode.type === 'container') {
          const containerData = sourceNode.data as ContainerNodeData;
          console.log(`    📦 CONTAINER collapsed=${containerData.collapsed}, size=${containerData.width}x${containerData.height}`);
          console.log(`    🎯 HANDLES: Container should have connection handles for hyperedges`);
        }
        
        console.log(`  🔗 TARGET NODE: ${targetNode.id} type=${targetNode.type}`);
        if (targetNode.parentId) {
          console.log(`    👶 CHILD NODE: parent=${targetNode.parentId}, position relative to parent`);
        }
        
        const dx = targetNode.position.x - sourceNode.position.x;
        const dy = targetNode.position.y - sourceNode.position.y;
        const distance = Math.sqrt(dx * dx + dy * dy);
        console.log(`  📏 DIRECT DISTANCE: ${distance.toFixed(2)}px (may not reflect actual rendered distance due to parent-child relationships)`);
        
        if (distance < 10) {
          console.log(`  ⚠️  WARNING: Hyperedge endpoints are very close/overlapping!`);
        }
      } else {
        console.log(`  ❌ ERROR: Could not find nodes for hyperedge endpoints`);
        console.log(`    Source ${hyperEdge.source}: ${sourceNode ? 'FOUND' : 'NOT FOUND'}`);
        console.log(`    Target ${hyperEdge.target}: ${targetNode ? 'FOUND' : 'NOT FOUND'}`);
      }
      
      const typedHyperEdge: TypedReactFlowEdge = {
        id: hyperEdge.id,
        type: 'hyper',
        source: hyperEdge.source,
        target: hyperEdge.target,
        // 🔥 HANDLE CONNECTION LOGGING
        // Let ReactFlow use default handle IDs for flexibility
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
      
      console.log(`  🎯 REACTFLOW EDGE: ${typedHyperEdge.id} type=${typedHyperEdge.type}`);
      console.log(`    sourceHandle: ${typedHyperEdge.sourceHandle || 'default'}, targetHandle: ${typedHyperEdge.targetHandle || 'default'}`);
      
      edges.push(typedHyperEdge);
    });

    const result: TypedReactFlowData = { nodes, edges };
    
    // Validate the result before returning
    const reactFlowReport = validateReactFlowResult(result);
    logValidationReport(reactFlowReport, 'ReactFlow Output');
    
    if (!reactFlowReport.isValid) {
      console.error('[ReactFlowConverter] ❌ Generated invalid ReactFlow data - container dimensions may be missing');
    }

    return result;
  }
}
