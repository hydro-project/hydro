/**
 * @fileoverview Bridge-Based Event Handlers
 * 
 * Compatibility wrappers for alpha event handling.
 */

export function createNodeEventHandlers(config?: any) {
  // // console.log((('[EventHandlers] ⚠️ createNodeEventHandlers is deprecated, use FlowGraph eventHandlers prop')));
  return {
    onClick: (event: any, node: any) => {
      // // console.log((('Node clicked:', node.id)));
    }
  };
}

export function createEdgeEventHandlers(config?: any) {
  // // console.log((('[EventHandlers] ⚠️ createEdgeEventHandlers is deprecated, use FlowGraph eventHandlers prop')));
  return {
    onClick: (event: any, edge: any) => {
      // // console.log((('Edge clicked:', edge.id)));
    }
  };
}

export function createContainerEventHandlers(config?: any) {
  // // console.log((('[EventHandlers] ⚠️ createContainerEventHandlers is deprecated, use FlowGraph eventHandlers prop')));
  return {
    onClick: (event: any, container: any) => {
      // // console.log((('Container clicked:', container.id)));
    }
  };
}
