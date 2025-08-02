# Graph Visualization with ReactFlow

This module provides a ReactFlow-based visualization system for graphs with automatic ELK layout. It includes custom nodes, edges, and container support with hierarchical layouts. The visualizer is completely independent and receives graph data via JSON.

## Installation

```bash
npm install reactflow elkjs
```

## Quick Start

```typescript
import React from 'react';
import { 
  GraphFlow,
  createVisualizationState,
  NODE_STYLES,
  EDGE_STYLES
} from './vis';

// Import CSS styles
import './vis/render/styles.css';

function MyGraph() {
  // Create visualization state
  const visualizationState = createVisualizationState();
  
  // Add nodes
  visualizationState.setGraphNode('node1', { 
    label: 'Input', 
    style: NODE_STYLES.DEFAULT 
  });
  visualizationState.setGraphNode('node2', { 
    label: 'Process', 
    style: NODE_STYLES.HIGHLIGHTED 
  });
  
  // Add edges
  visualizationState.setGraphEdge('edge1', { 
    source: 'node1', 
    target: 'node2',
    style: EDGE_STYLES.DEFAULT
  });

  return (
    <div style={{ width: '100%', height: '600px' }}>
      <GraphFlow 
        visualizationState={visualizationState}
        onLayoutComplete={() => console.log('Layout done!')}
      />
    </div>
  );
}
```

## Features

### Automatic Layout with ELK

The system uses ELK.js for automatic graph layout with multiple algorithm options:

- **Layered**: Hierarchical layout (default)
- **Force**: Force-directed layout
- **Stress**: Stress-minimization layout  
- **Radial**: Radial tree layout

```typescript
const layoutConfig = {
  algorithm: 'layered',
  direction: 'DOWN',
  spacing: {
    nodeNode: 50,
    edgeEdge: 10,
    edgeNode: 15,
    componentComponent: 100
  }
};

<GraphFlow 
  visualizationState={state}
  layoutConfig={layoutConfig}
/>
```

### Custom Nodes and Containers

#### Standard Nodes
- Styled based on node type (default, highlighted, error, warning)
- Clickable and draggable
- Support for custom properties

#### Container Nodes
- Collapsible/expandable containers
- Hierarchical grouping of child nodes
- Automatic sizing based on contents

```typescript
// Create a container
coreState.setContainer('container1', {
  expandedDimensions: { width: 300, height: 200 },
  children: ['node1', 'node2'],
  collapsed: false
});
```

### HyperEdges

Visualize aggregated edges as special hyper-edges:

```typescript
// HyperEdges are automatically generated from multiple edges
// between the same source/target when nodes are collapsed
```

### Event Handling

```typescript
const eventHandlers = {
  onNodeClick: (event, node) => {
    console.log('Clicked:', node.id);
  },
  onNodeDoubleClick: (event, node) => {
    // Toggle container collapse
    if (node.type === 'container') {
      const container = state.getContainer(node.id);
      state.updateContainer(node.id, { 
        collapsed: !container.collapsed 
      });
    }
  },
  onEdgeClick: (event, edge) => {
    console.log('Edge clicked:', edge.id);
  }
};

<GraphFlow 
  visualizationState={state}
  eventHandlers={eventHandlers}
/>
```

### Styling and Themes

The system includes comprehensive CSS styling:

```css
/* Custom node styles */
.graph-standard-node.highlighted {
  border-color: #ffc107;
  background: rgba(255, 193, 7, 0.1);
}

/* Custom edge styles */
.react-flow__edge-path.thick {
  stroke-width: 3px;
}

/* Container styles */
.graph-container-node:hover {
  border-color: #007bff;
}
```

### Configuration Options

#### Layout Configuration

```typescript
interface LayoutConfig {
  algorithm: 'layered' | 'stress' | 'mrtree' | 'radial' | 'force' | 'disco';
  direction: 'DOWN' | 'UP' | 'LEFT' | 'RIGHT';
  spacing: {
    nodeNode: number;
    edgeEdge: number; 
    edgeNode: number;
    componentComponent: number;
  };
  containerPadding: {
    top: number;
    bottom: number;
    left: number;
    right: number;
  };
  nodeSize: {
    width: number;
    height: number;
  };
  hierarchical: boolean;
  separateConnectedComponents: boolean;
}
```

#### Render Configuration

```typescript
interface RenderConfig {
  enableZoom: boolean;
  enablePan: boolean;
  enableSelection: boolean;
  enableMiniMap: boolean;
  enableControls: boolean;
  fitView: boolean;
  snapToGrid: boolean;
  gridSize: number;
  nodesDraggable: boolean;
  nodesConnectable: boolean;
  elementsSelectable: boolean;
  showBackground: boolean;
  backgroundVariant: 'lines' | 'dots' | 'cross';
  backgroundColor: string;
}
```

## Architecture

### Core Components

1. **ELKLayoutEngine**: Handles automatic layout using ELK.js
2. **ReactFlowConverter**: Converts layout results to ReactFlow format
3. **GraphFlow**: Main React component
4. **Custom Nodes/Edges**: Specialized ReactFlow components
5. **VisualizationState**: Generic graph state management (JSON-driven)

### Data Flow

```
JSON Graph Data -> VisualizationState -> ELKLayoutEngine -> LayoutResult -> ReactFlowConverter -> ReactFlow
```

### File Structure

```
render/
├── GraphFlow.tsx          # Main component
├── ReactFlowConverter.ts  # Data conversion
├── nodes.tsx             # Custom node components  
├── edges.tsx             # Custom edge components
├── types.ts              # Type definitions
├── config.ts             # Default configurations
├── styles.css            # CSS styling
└── index.ts              # Exports

layout/
├── ELKLayoutEngine.ts    # ELK integration
├── types.ts              # Layout types
├── config.ts             # Layout defaults
└── index.ts              # Exports
```

## Advanced Usage

### Custom Node Types

You can extend the system with custom node types:

```typescript
import { NodeProps } from 'reactflow';

const CustomNode: React.FC<NodeProps> = ({ data }) => {
  return (
    <div className="custom-node">
      {data.label}
    </div>
  );
};

// Register the custom type
const nodeTypes = {
  'custom': CustomNode
};
```

### Performance Optimization

For large graphs (1000+ nodes):

1. Use `useMemo` for expensive calculations
2. Implement virtualization for very large datasets
3. Debounce layout updates
4. Use `React.memo` for custom components

### Debugging

Enable debug mode:

```typescript
<GraphFlow 
  visualizationState={state}
  onError={(error) => console.error('Viz error:', error)}
  onLayoutComplete={() => console.log('Layout complete')}
/>
```

## Usage

The main entry point for the visualization system is the `vis.js` page, which provides:

- File upload and JSON parsing
- Interactive graph visualization with ReactFlow v12
- InfoPanel with legend and hierarchy controls
- Zoom and pan controls
- Container collapse/expand functionality

## Dependencies

- `reactflow`: ^12.3.0 - React-based node graph library
- `elkjs`: ^0.9.3 - Automatic graph layout
- `react`: ^18.0.0 - React framework

## Browser Support

- Chrome 90+
- Firefox 88+
- Safari 14+
- Edge 90+
