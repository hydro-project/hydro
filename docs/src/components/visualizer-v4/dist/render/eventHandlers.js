/**
 * @fileoverview Bridge-Based Event Handlers
 *
 * Compatibility wrappers for alpha event handling.
 */
export function createNodeEventHandlers(config) {
    console.log('[EventHandlers] ⚠️ createNodeEventHandlers is deprecated, use FlowGraph eventHandlers prop');
    return {
        onClick: (event, node) => {
            console.log('Node clicked:', node.id);
        }
    };
}
export function createEdgeEventHandlers(config) {
    console.log('[EventHandlers] ⚠️ createEdgeEventHandlers is deprecated, use FlowGraph eventHandlers prop');
    return {
        onClick: (event, edge) => {
            console.log('Edge clicked:', edge.id);
        }
    };
}
export function createContainerEventHandlers(config) {
    console.log('[EventHandlers] ⚠️ createContainerEventHandlers is deprecated, use FlowGraph eventHandlers prop');
    return {
        onClick: (event, container) => {
            console.log('Container clicked:', container.id);
        }
    };
}
//# sourceMappingURL=eventHandlers.js.map