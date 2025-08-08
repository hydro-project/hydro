# Quick Start Guide

Get up and running with the Hydro Visualization System in minutes.

## Installation

```bash
# Install dependencies
npm install

# Build the visualization components
npm run build

# Run tests to verify setup
npm test
```

## Basic Usage

### 1. Create a Simple Graph

```typescript
import { createVisualizationState, NODE_STYLES, EDGE_STYLES } from './core/VisState';

// Create state instance
const state = createVisualizationState();

// Add nodes
state
  .setGraphNode('input', { 
    label: 'Data Input',
    style: NODE_STYLES.HIGHLIGHTED 
  })
  .setGraphNode('process', { 
    label: 'Process Data',
    style: NODE_STYLES.DEFAULT 
  })
  .setGraphNode('output', { 
    label: 'Results',
    style: NODE_STYLES.DEFAULT 
  });

// Add edges
state
  .setGraphEdge('flow1', { 
    source: 'input', 
    target: 'process' 
  })
  .setGraphEdge('flow2', { 
    source: 'process', 
    target: 'output' 
  });

// Access the graph data
// // console.log((('Nodes:', state.visibleNodes)));
// // console.log((('Edges:', state.visibleEdges)));
```

### 2. Add Containers (Grouping)

```typescript
// Create a container to group nodes
state.setContainer('pipeline', {
  collapsed: false,
  children: new Set(['input', 'process', 'output'])
});

// Collapse the container to hide children
state.collapseContainer('pipeline');

// Expand to show children again
state.expandContainer('pipeline');
```

### 3. Dynamic Updates

```typescript
// Update existing elements
state
  .updateNode('input', { style: NODE_STYLES.WARNING })
  .updateEdge('flow1', { style: EDGE_STYLES.THICK })
  .updateContainer('pipeline', { collapsed: true });

// Add new elements
state
  .setGraphNode('monitor', { label: 'Monitor' })
  .setGraphEdge('status', { source: 'output', target: 'monitor' });
```

## React Integration

### Basic FlowGraph Component

```typescript
import React from 'react';
import { FlowGraph } from './render/FlowGraph';
import { createVisualizationState } from './core/VisState';

function MyVisualization() {
  // Create your graph state
  const state = createVisualizationState()
    .setGraphNode('node1', { label: 'First Node' })
    .setGraphNode('node2', { label: 'Second Node' })
    .setGraphEdge('edge1', { source: 'node1', target: 'node2' });

  return (
    <div style={{ height: '400px', width: '600px' }}>
      <FlowGraph 
        visualizationState={state}
        layoutConfig={{ algorithm: 'layered' }}
        renderConfig={{ enableMiniMap: true }}
      />
    </div>
  );
}
```

### With Event Handlers

```typescript
function InteractiveVisualization() {
  const [state, setState] = useState(() => 
    createVisualizationState()
      .setGraphNode('node1', { label: 'Click me!' })
  );

  const handleNodeClick = (nodeId: string) => {
    // // console.log((('Node clicked:', nodeId)));
    // Update node style on click
    state.updateNode(nodeId, { style: NODE_STYLES.SELECTED });
    setState(state); // Trigger re-render
  };

  return (
    <FlowGraph 
      visualizationState={state}
      eventHandlers={{ onNodeClick: handleNodeClick }}
    />
  );
}
```

## Common Patterns

### Building from JSON Data

```typescript
// Parse JSON data into VisualizationState
import { JSONParser } from './core/JSONParser';

const jsonData = {
  nodes: [
    { id: 'n1', label: 'Node 1' },
    { id: 'n2', label: 'Node 2' }
  ],
  edges: [
    { id: 'e1', source: 'n1', target: 'n2' }
  ]
};

const state = JSONParser.parse(jsonData);
```

### Working with Hierarchies

```typescript
// Create nested structure
state
  .setGraphNode('child1', { label: 'Child 1' })
  .setGraphNode('child2', { label: 'Child 2' })
  .setContainer('parent', {
    children: new Set(['child1', 'child2'])
  })
  .setContainer('grandparent', {
    children: new Set(['parent'])
  });

// Query hierarchy
const children = state.getContainerChildren('parent');
const parent = state.getNodeContainer('child1');
```

### Layout Customization

```typescript
import { DEFAULT_LAYOUT_CONFIG } from './layout';

const customLayout = {
  ...DEFAULT_LAYOUT_CONFIG,
  algorithm: 'force',
  spacing: { nodeNode: 50 },
  direction: 'RIGHT'
};

// Use custom layout
<FlowGraph 
  visualizationState={state}
  layoutConfig={customLayout}
/>
```

## Styling and Themes

### Using Predefined Styles

```typescript
import { NODE_STYLES, EDGE_STYLES } from './shared/constants';

// Apply styles to elements
state
  .setGraphNode('error-node', { 
    label: 'Error',
    style: NODE_STYLES.ERROR 
  })
  .setGraphEdge('warning-edge', { 
    source: 'node1',
    target: 'error-node',
    style: EDGE_STYLES.DASHED 
  });
```

### Custom Styling

```typescript
// Custom node styling through data
state.setGraphNode('custom', {
  label: 'Custom Node',
  style: 'custom',
  nodeType: 'special', // Used by color utilities
  customData: { priority: 'high' }
});
```

## Error Handling

### Validation Errors

```typescript
try {
  state.setGraphNode('', { label: 'Invalid' }); // Empty ID
} catch (error) {
  console.error('Validation error:', error.message);
}

// Safe access
const node = state.getGraphNode('maybe-missing');
if (node) {
  // Node exists
} else {
  // Handle missing node
}
```

### Layout Errors

```typescript
// Layout engine handles errors gracefully
const layoutConfig = {
  algorithm: 'invalid-algorithm' // Will fallback to default
};
```

## Performance Tips

### Batch Operations

```typescript
// Good: Chain operations
state
  .setGraphNode('n1', { label: 'Node 1' })
  .setGraphNode('n2', { label: 'Node 2' })
  .setGraphEdge('e1', { source: 'n1', target: 'n2' });

// Less optimal: Separate operations
state.setGraphNode('n1', { label: 'Node 1' });
state.setGraphNode('n2', { label: 'Node 2' });
state.setGraphEdge('e1', { source: 'n1', target: 'n2' });
```

### Efficient Queries

```typescript
// Efficient: Use getters
const visibleNodes = state.visibleNodes;

// Inefficient: Manual filtering
const visibleNodes = Array.from(state.graphNodes.values())
  .filter(node => !node.hidden);
```

## Testing Your Graphs

### Unit Testing

```typescript
import { createVisualizationState } from './core/VisState';

describe('My Graph Logic', () => {
  test('should create basic graph', () => {
    const state = createVisualizationState()
      .setGraphNode('test', { label: 'Test Node' });
    
    expect(state.visibleNodes).toHaveLength(1);
    expect(state.visibleNodes[0].label).toBe('Test Node');
  });
});
```

### Integration Testing

```typescript
// Test full pipeline
const state = createVisualizationState()
  .setGraphNode('n1', { label: 'Node 1' })
  .setGraphNode('n2', { label: 'Node 2' });

// Verify layout integration
const layoutResult = await layoutEngine.layout(
  state.visibleNodes,
  state.visibleEdges,
  state.visibleContainers
);

expect(layoutResult.nodes).toHaveLength(2);
```

## Next Steps

1. **Read the [Architecture Guide](./Architecture.md)** - Understand system design
2. **Review [VisualizationState API](./VisState.md)** - Deep dive into core state management
3. **Explore Examples** - Check out example implementations in `/examples`
4. **Customize Components** - Create custom node/edge components
5. **Performance Optimization** - Learn advanced performance patterns

## Common Issues

### My elements aren't visible
Check the `hidden` property:
```typescript
const node = state.getGraphNode('nodeId');
if (node?.hidden) {
  state.updateNode('nodeId', { hidden: false });
}
```

### Container hierarchy errors
Avoid circular dependencies:
```typescript
// This would create a cycle and throw an error
state.addContainerChild('parent', 'child');
state.addContainerChild('child', 'parent'); // Error!
```

### Layout not updating
Ensure containers have proper layout configuration:
```typescript
state.setContainerLayout('containerId', {
  elkFixed: false // Allow ELK to reposition
});
```

## Resources

- [Full API Documentation](./VisState.md)
- [Architecture Overview](./Architecture.md)
- [Test Examples](./__tests__/)
- [Component Source](./render/)

---

*Need help? Check the [Troubleshooting section](./VisState.md#troubleshooting) in the VisualizationState documentation.*