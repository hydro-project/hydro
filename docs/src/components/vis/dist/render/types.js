/**
 * @fileoverview ReactFlow Integration Types
 *
 * Strong TypeScript types to enforce correct data flow from ELK layout to ReactFlow rendering.
 * These types ensure that ELK-calculated dimensions are properly passed through the pipeline.
 */
// ============ Type Guards ============
/**
 * Type guard to check if node data is container data
 */
export function isContainerNodeData(data) {
    return 'width' in data && 'height' in data && 'collapsed' in data;
}
/**
 * Type guard to check if node is a container node
 */
export function isContainerNode(node) {
    return node.type === 'container';
}
/**
 * Type guard to check if ELK container has required dimensions
 */
export function isValidELKContainer(container) {
    return (typeof container.id === 'string' &&
        typeof container.x === 'number' &&
        typeof container.y === 'number' &&
        typeof container.width === 'number' &&
        typeof container.height === 'number' &&
        typeof container.collapsed === 'boolean');
}
// ============ Validation Functions ============
/**
 * Validates that ELK layout result has all required properties
 */
export function validateELKLayoutResult(result) {
    if (!result || typeof result !== 'object')
        return false;
    // Check nodes
    if (!Array.isArray(result.nodes))
        return false;
    if (!result.nodes.every((node) => typeof node.id === 'string' &&
        typeof node.x === 'number' &&
        typeof node.y === 'number' &&
        typeof node.width === 'number' &&
        typeof node.height === 'number'))
        return false;
    // Check containers
    if (!Array.isArray(result.containers))
        return false;
    if (!result.containers.every(isValidELKContainer))
        return false;
    return true;
}
/**
 * Validates that ReactFlow data has proper container dimensions
 */
export function validateReactFlowData(data) {
    if (!data || typeof data !== 'object')
        return false;
    if (!Array.isArray(data.nodes) || !Array.isArray(data.edges))
        return false;
    // Check that all container nodes have proper dimensions
    return data.nodes.every((node) => {
        if (node.type === 'container') {
            return (node.data &&
                typeof node.data.width === 'number' &&
                typeof node.data.height === 'number' &&
                node.style &&
                typeof node.style.width === 'number' &&
                typeof node.style.height === 'number' &&
                node.data.width === node.style.width &&
                node.data.height === node.style.height);
        }
        return true;
    });
}
//# sourceMappingURL=types.js.map