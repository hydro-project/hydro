# VisualizationState Documentation

## Overview

The `VisualizationState` class is the core data structure for managing graph visualization state in the Hydro visualization system. It provides efficient access to graph elements (nodes, edges, containers, hyperEdges) with automatic visibility tracking and hierarchical container support.

## Table of Contents

- [Architecture](#architecture)
- [Core Concepts](#core-concepts)
- [API Reference](#api-reference)
- [Usage Examples](#usage-examples)
- [Performance Considerations](#performance-considerations)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Architecture

### State Management Pattern

VisualizationState follows a centralized state management pattern where:

- **Single Source of Truth**: All graph state is managed within one VisualizationState instance
- **Immutable Updates**: Methods return `this` for chaining while maintaining internal consistency
- **Efficient Access**: O(1) element lookups using Maps with cached visible element collections
- **Automatic Visibility**: Visible elements are automatically tracked based on `hidden` properties
- **Hierarchical Support**: Container/child relationships are maintained with automatic constraint enforcement

### Core Data Structures

```typescript
class VisualizationState {
  // Core element storage (all elements)
  private readonly graphNodes: Map<string, GraphNode>
  private readonly graphEdges: Map<string, GraphEdge>
  private readonly containers: Map<string, Container>
  private readonly hyperEdges: Map<string, HyperEdge>
  
  // Cached visible element collections (non-hidden)
  private readonly _visibleNodes: Map<string, GraphNode>
  private readonly _visibleEdges: Map<string, GraphEdge>  
  private readonly _visibleContainers: Map<string, Container>
  private readonly _expandedContainers: Map<string, Container>
  
  // Hierarchy tracking (private with # syntax)
  readonly #containerChildren: Map<string, Set<string>>
  readonly #nodeContainers: Map<string, string>
}
```

### Performance Characteristics

- **Element Lookup**: O(1) via Map-based storage
- **Visible Element Access**: O(1) via cached collections
- **Container Hierarchy Operations**: O(1) for parent/child lookups
- **Bulk Operations**: O(n) where n is the number of affected elements
- **Memory Usage**: Optimized with shared references, minimal duplication

## Core Concepts

### Element Types

#### GraphNode
Represents a basic graph node with styling and visibility.

```typescript
interface GraphNode {
  id: string;           // Unique identifier
  label: string;        // Display text
  style: string;        // Visual style identifier
  hidden: boolean;      // Visibility flag
  layout?: any;         // Layout-specific data
  dimensions?: any;     // Size information
}
```

#### GraphEdge
Represents a connection between two nodes.

```typescript
interface GraphEdge {
  id: string;           // Unique identifier
  source: string;       // Source node ID
  target: string;       // Target node ID
  style: string;        // Visual style identifier
  hidden: boolean;      // Visibility flag
}
```

#### Container
Represents a hierarchical grouping of nodes/other containers.

```typescript
interface Container {
  id: string;           // Unique identifier
  collapsed: boolean;   // Collapse state
  hidden: boolean;      // Visibility flag
  children: Set<string>; // Set of child element IDs
  layout?: any;         // Layout-specific data
  dimensions?: any;     // Size information
}
```

#### HyperEdge
Represents aggregated edges (used in collapsed containers).

```typescript
interface HyperEdge {
  id: string;              // Unique identifier  
  source: string;          // Source node/container ID
  target: string;          // Target node/container ID
  style: string;           // Visual style identifier
  aggregatedEdges: GraphEdge[]; // original edges
}
```

### Visibility Management

Elements have automatic visibility tracking:

- **Visible Elements**: Elements where `hidden: false`
- **Hidden Elements**: Elements where `hidden: true`
- **Automatic Updates**: Visibility collections update immediately when hidden property changes

### Container Hierarchy

Containers support hierarchical relationships:

- **Parent-Child Relationships**: Containers can contain nodes or other containers
- **Constraint Enforcement**: Automatic validation prevents circular dependencies
- **Collapse/Expand**: Containers can be collapsed (children hidden) or expanded (children visible)

## API Reference

### Factory Function

#### `createVisualizationState(): VisualizationState`
Creates a new VisualizationState instance.

```typescript
const state = createVisualizationState();
```

### Node Management

#### `setGraphNode(id: string, props: CreateNodeProps): VisualizationState`
Adds or updates a graph node.

**Parameters:**
- `id` - Unique node identifier
- `props.label` - Display label (required)
- `props.style` - Visual style (default: NODE_STYLES.DEFAULT)
- `props.hidden` - Visibility flag (default: false)
- `props.*` - Additional custom properties

**Returns:** This instance for method chaining

**Example:**
```typescript
state.setGraphNode('node1', {
  label: 'Source Node',
  style: NODE_STYLES.HIGHLIGHTED,
  customData: { type: 'source' }
});
```

#### `getGraphNode(id: string): GraphNode | undefined`
Retrieves a node by ID.

#### `updateNode(id: string, updates: NodeUpdates): VisualizationState`
Updates specific node properties.

**Parameters:**
- `updates.hidden` - Update visibility
- `updates.style` - Update visual style
- `updates.label` - Update display label

#### `removeGraphNode(id: string): void`
Removes a node and all connected edges.

### Edge Management

#### `setGraphEdge(id: string, props: CreateEdgeProps): VisualizationState`
Adds or updates a graph edge.

**Parameters:**
- `id` - Unique edge identifier
- `props.source` - Source node ID (required)
- `props.target` - Target node ID (required)
- `props.style` - Visual style (default: EDGE_STYLES.DEFAULT)
- `props.hidden` - Visibility flag (default: false)

#### `getGraphEdge(id: string): GraphEdge | undefined`
Retrieves an edge by ID.

#### `updateEdge(id: string, updates: EdgeUpdates): VisualizationState`
Updates specific edge properties.

#### `removeGraphEdge(id: string): void`
Removes an edge.

### Container Management

#### `setContainer(id: string, props: CreateContainerProps): VisualizationState`
Adds or updates a container.

**Parameters:**
- `id` - Unique container identifier
- `props.collapsed` - Collapse state (default: false)
- `props.children` - Set of child element IDs
- `props.hidden` - Visibility flag (default: false)

#### `getContainer(id: string): Container | undefined`
Retrieves a container by ID.

#### `updateContainer(id: string, updates: ContainerUpdates): VisualizationState`
Updates specific container properties.

#### `addContainerChild(containerId: string, childId: string): void`
Adds a child to a container.

#### `removeContainerChild(containerId: string, childId: string): void`
Removes a child from a container.

#### `removeContainer(id: string): void`
Removes a container and handles child cleanup.

#### `collapseContainer(containerId: string): void`
Collapses a container, hiding children and creating hyperEdges.

#### `expandContainer(containerId: string): void`
Expands a container, showing children and removing hyperEdges.

### HyperEdge Management

#### `setHyperEdge(id: string, props: HyperEdgeProps): VisualizationState`
Adds or updates a hyperEdge.

#### `getHyperEdge(id: string): HyperEdge | undefined`
Retrieves a hyperEdge by ID.

#### `removeHyperEdge(id: string): void`
Removes a hyperEdge.

### Visibility Access (Getters)

#### `get visibleNodes(): GraphNode[]`
Returns array of all visible (non-hidden) nodes.

#### `get visibleEdges(): GraphEdge[]`
Returns array of all visible (non-hidden) edges.

#### `get visibleContainers(): Container[]`
Returns array of all visible (non-hidden) containers.

#### `get expandedContainers(): Container[]`
Returns array of all non-collapsed containers.

#### `get allHyperEdges(): HyperEdge[]`
Returns array of all hyperEdges.

### Hierarchy Access

#### `getContainerChildren(containerId: string): ReadonlySet<string>`
Returns read-only set of child IDs for a container.

#### `getNodeContainer(nodeId: string): string | undefined`
Returns the parent container ID for a node.

### Layout Integration

#### `setNodeLayout(id: string, layout: Partial<any>): void`
Sets layout-specific data for a node.

#### `getNodeLayout(id: string): any`
Gets layout data for a node.

#### `setContainerLayout(id: string, layout: Partial<any>): void`
Sets layout-specific data for a container.

#### `getContainerLayout(id: string): any`
Gets layout data for a container.

#### `setContainerELKFixed(id: string, fixed: boolean): void`
Controls whether ELK layout engine should fix container position.

#### `getContainersRequiringLayout(changedContainerId?: string): Container[]`
Returns containers that need layout calculation with position fixing logic.

### State Management

#### `clear(): void`
Removes all elements and resets state.

## Usage Examples

### Basic Graph Construction

```typescript
import { createVisualizationState, NODE_STYLES, EDGE_STYLES } from './core/VisState';

// Create state and build graph with method chaining
const state = createVisualizationState()
  .setGraphNode('source', { 
    label: 'Data Source',
    style: NODE_STYLES.HIGHLIGHTED 
  })
  .setGraphNode('transform', { 
    label: 'Transform',
    style: NODE_STYLES.DEFAULT 
  })
  .setGraphNode('sink', { 
    label: 'Output',
    style: NODE_STYLES.DEFAULT 
  })
  .setGraphEdge('e1', { 
    source: 'source', 
    target: 'transform',
    style: EDGE_STYLES.DEFAULT 
  })
  .setGraphEdge('e2', { 
    source: 'transform', 
    target: 'sink',
    style: EDGE_STYLES.THICK 
  });

// Access visible elements for rendering
// // console.log((('Nodes to render:', state.visibleNodes)));
// // console.log((('Edges to render:', state.visibleEdges)));
```

### Hierarchical Containers

```typescript
// Create container with nested structure
state
  .setContainer('cluster1', {
    collapsed: false,
    children: new Set(['node1', 'node2'])
  })
  .setContainer('cluster2', {
    collapsed: false, 
    children: new Set(['node3', 'cluster1']) // Nested container
  });

// Collapse container to hide children
state.collapseContainer('cluster1');
// // console.log((('Expanded containers:', state.expandedContainers)));

// Expand to show children again
state.expandContainer('cluster1');
```

### Dynamic Updates

```typescript
// Update individual elements
state
  .updateNode('source', { style: NODE_STYLES.ERROR })
  .updateEdge('e1', { hidden: true })
  .updateContainer('cluster1', { collapsed: true });

// Add new elements dynamically
state
  .setGraphNode('newNode', { label: 'Dynamic Node' })
  .setGraphEdge('newEdge', { source: 'newNode', target: 'sink' });

// Remove elements
state.removeGraphNode('oldNode'); // Also removes connected edges
state.removeGraphEdge('oldEdge');
```

### Layout Integration

```typescript
// Set layout information for ELK integration
state.setNodeLayout('node1', { 
  x: 100, 
  y: 100, 
  width: 120, 
  height: 40 
});

state.setContainerLayout('cluster1', {
  x: 50,
  y: 50, 
  width: 300,
  height: 200,
  elkFixed: false // Allow ELK to reposition
});

// Get containers that need layout calculation
const containersForLayout = state.getContainersRequiringLayout('cluster1');
```

### Container Hierarchy Queries

```typescript
// Check hierarchy relationships
const children = state.getContainerChildren('cluster1');
// // console.log((('Cluster1 children:', Array.from(children))));

const parent = state.getNodeContainer('node1');
// // console.log((('Node1 parent:', parent)));

// Validate hierarchy constraints
try {
  state.addContainerChild('cluster1', 'cluster2'); // Would create cycle
} catch (error) {
  // // console.log((('Hierarchy constraint prevented cycle')));
}
```

## Performance Considerations

### Optimization Strategies

1. **Batch Operations**: Chain multiple updates to minimize internal recalculations
   ```typescript
   // Good: Single chain
   state.setGraphNode('n1', {}).setGraphNode('n2', {}).setGraphEdge('e1', {});
   
   // Less optimal: Multiple separate calls
   state.setGraphNode('n1', {});
   state.setGraphNode('n2', {});
   state.setGraphEdge('e1', {});
   ```

2. **Visibility Access**: Use getters for efficient access to visible elements
   ```typescript
   // Efficient: Direct getter access
   const nodes = state.visibleNodes;
   
   // Inefficient: Manual filtering
   const nodes = Array.from(state.graphNodes.values()).filter(n => !n.hidden);
   ```

3. **Container Operations**: Leverage automatic hierarchy management
   ```typescript
   // Efficient: Let VisualizationState manage relationships
   state.addContainerChild('container1', 'node1');
   
   // Inefficient: Manual container manipulation
   const container = state.getContainer('container1');
   container.children.add('node1');
   ```

### Memory Management

- **Shared References**: VisualizationState uses shared object references to minimize memory usage
- **Automatic Cleanup**: Removing elements automatically cleans up related data
- **Efficient Collections**: Maps and Sets are used for O(1) access patterns

### Scalability Limits

- **Recommended Limits**: 
  - Nodes: < 10,000 for optimal performance
  - Edges: < 50,000 for optimal performance
  - Container depth: < 10 levels for hierarchy operations

## Best Practices

### State Management

1. **Use Factory Function**: Always use `createVisualizationState()` instead of `new VisualizationState()`
2. **Method Chaining**: Leverage fluent interface for readable construction
3. **Single State Instance**: Use one VisualizationState per visualization
4. **Consistent IDs**: Use meaningful, stable identifiers for elements

### Error Handling

1. **Validate Input**: VisualizationState throws descriptive errors for invalid operations
2. **Check Existence**: Use getter methods to safely access elements
3. **Handle Hierarchy Constraints**: Expect errors when creating circular dependencies

```typescript
try {
  state.setGraphNode('', { label: 'Invalid' }); // Throws: empty ID
} catch (error) {
  console.error('Validation error:', error.message);
}

const node = state.getGraphNode('maybe-missing');
if (node) {
  // Safe to use node
} else {
  // Handle missing node case
}
```

### Performance

1. **Batch Updates**: Group related state changes
2. **Avoid Frequent Queries**: Cache visible element arrays when possible
3. **Use Hierarchy Queries**: Leverage built-in hierarchy methods
4. **Monitor State Size**: Watch for excessive element counts

## Troubleshooting

### Common Issues

#### Element Not Visible

**Problem**: Element exists but doesn't appear in `visibleNodes`/`visibleEdges`

**Solution**: Check the `hidden` property
```typescript
const node = state.getGraphNode('nodeId');
if (node?.hidden) {
  state.updateNode('nodeId', { hidden: false });
}
```

#### Container Hierarchy Errors

**Problem**: "Circular dependency" errors when adding container children

**Solution**: Check for existing parent-child relationships
```typescript
const existingParent = state.getNodeContainer('childId');
if (existingParent) {
  state.removeContainerChild(existingParent, 'childId');
}
state.addContainerChild('newParent', 'childId');
```

#### Missing Edges After Node Removal

**Problem**: Edges disappear when nodes are removed

**Solution**: This is expected behavior. Edges are automatically cleaned up when connected nodes are removed. To preserve edges, hide nodes instead:
```typescript
// Removes node AND connected edges
state.removeGraphNode('nodeId');

// Hides node but preserves edges
state.updateNode('nodeId', { hidden: true });
```

#### Layout Integration Issues

**Problem**: ELK layout not working correctly with containers

**Solution**: Ensure containers have proper layout data and position fixing
```typescript
// Set up container for ELK layout
state.setContainerLayout('containerId', {
  elkFixed: false, // Allow repositioning
  // ... other layout properties
});

// Get containers ready for layout
const containers = state.getContainersRequiringLayout('changedId');
```

### Debugging

1. **Inspect State**: Use browser dev tools to examine VisualizationState instance
2. **Check Visibility**: Verify `hidden` properties on elements
3. **Validate Hierarchy**: Use `getContainerChildren()` and `getNodeContainer()` to check relationships
4. **Monitor Performance**: Watch for excessive element counts or deep hierarchies

### Error Messages

Common error messages and their meanings:

- `"Entity does not exist"` - Trying to access non-existent element
- `"Node ID is required"` - Empty or invalid element ID
- `"Invalid style value"` - Style not in allowed constants
- `"Circular dependency detected"` - Container hierarchy would create cycle
- `"Source/target node does not exist"` - Edge references missing nodes

---

*This documentation covers VisualizationState version as of the current implementation. For the latest updates, refer to the inline TypeScript documentation in `core/VisState.ts`.*