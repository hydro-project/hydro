# Hydro Graph Visualizer

A React component for visualizing Hydro dataflow graphs using ReactFlow v12 and ELK layout algorithms.

## Quick Start

```javascript
import { Visualizer } from './visualizer';

function App() {
  return <Visualizer graphData={yourGraphData} />;
}
```

## Components

### `Visualizer` - Simple Graph Visualizer

The basic visualizer component with minimal controls.

```javascript
import { Visualizer } from './visualizer';

<Visualizer 
  graphData={graphData}
/>
```

**Props:**
- `graphData` (object, required): Graph data containing nodes and edges

### `GraphCanvas` - Advanced Graph Visualizer

Full-featured visualizer with layout controls and advanced options.

```javascript
import { GraphCanvas } from './visualizer';

<GraphCanvas 
  graphData={graphData}
  maxVisibleNodes={100}
/>
```

**Props:**
- `graphData` (object, required): Graph data containing nodes and edges
- `maxVisibleNodes` (number, optional): Maximum number of nodes to display (default: 50)

### `ReactFlowVisualization` - Legacy Compatibility

Drop-in replacement for older ReactFlow visualizations.

```javascript
import { ReactFlowVisualization } from './visualizer';

<ReactFlowVisualization graphData={graphData} />
```

## Supported Data Format

### Basic Graph Structure

```javascript
{
  "nodes": [
    {
      "id": "unique-node-id",
      "data": {
        "label": "Node Display Name",
        "nodeType": "Source",  // See node types below
        "locationId": 0,       // Optional: for internal tracking
        "location": "local"    // Optional: for internal tracking
      }
    }
  ],
  "edges": [
    {
      "id": "unique-edge-id",
      "source": "source-node-id",
      "target": "target-node-id",
      "data": {
        "label": "Edge Label"  // Optional
      }
    }
  ],
  "locations": [              // Optional: for internal tracking
    {
      "id": 0,
      "label": "Location Name"
    }
  ]
}
```

### Node Types

The visualizer supports the following node types with automatic color coding:

- `"Source"` - Data source nodes (teal)
- `"Transform"` - Data transformation nodes (purple) 
- `"Sink"` - Data sink/output nodes (blue)
- `"Network"` - Network operation nodes (pink)
- `"Operator"` - General operator nodes (gray)
- `"Join"` - Join operation nodes (light purple)
- `"Union"` - Union operation nodes (light green)
- `"Filter"` - Filter operation nodes (yellow)

### Example JSON File

```json
{
  "nodes": [
    {
      "id": "source1",
      "data": {
        "label": "Input Stream",
        "nodeType": "Source"
      }
    },
    {
      "id": "transform1", 
      "data": {
        "label": "Map Operation",
        "nodeType": "Transform"
      }
    },
    {
      "id": "sink1",
      "data": {
        "label": "Output",
        "nodeType": "Sink"
      }
    }
  ],
  "edges": [
    {
      "id": "e1",
      "source": "source1",
      "target": "transform1"
    },
    {
      "id": "e2", 
      "source": "transform1",
      "target": "sink1"
    }
  ]
}
```

## Layout Algorithms

The visualizer supports multiple ELK-based layout algorithms:

- **`mrtree`** (default) - Multi-rooted tree layout, good for hierarchical graphs
- **`layered`** - Layered layout, good for directed acyclic graphs  
- **`force`** - Force-directed layout, good for general graphs
- **`stress`** - Stress-minimization layout, good for sparse graphs
- **`radial`** - Radial layout, good for tree-like structures

Layout can be changed through the UI controls in `GraphCanvas` component.

## Color Palettes

Three color palettes are available:

- **`Set3`** (default) - Bright, distinct colors
- **`Pastel1`** - Soft, pastel colors  
- **`Dark2`** - Dark, high-contrast colors

## Features

### Interactive Controls
- **Zoom** - Mouse wheel or zoom controls
- **Pan** - Click and drag to move around
- **Node Selection** - Click nodes to select them
- **Node Dragging** - Drag nodes to reposition (temporary)
- **Edge Selection** - Click edges to select them

### UI Components
- **MiniMap** - Overview of the entire graph in bottom-right corner
- **Controls** - Zoom in/out, fit view, fullscreen controls
- **Background Grid** - Visual grid for better spatial reference
- **Layout Controls** - Dropdown to change layout algorithm (GraphCanvas only)
- **Color Palette Controls** - Dropdown to change color scheme (GraphCanvas only)
- **Legend** - Shows node type color coding (GraphCanvas only)

### Advanced Features
- **Automatic Layout** - ELK algorithms automatically position nodes
- **Edge Routing** - Smart edge routing around nodes
- **Responsive Design** - Adapts to container size
- **Performance Optimized** - Handles large graphs efficiently

## Loading Data

### From JSON File

```javascript
// Load JSON file
const response = await fetch('path/to/graph.json');
const graphData = await response.json();

// Use with visualizer
<Visualizer graphData={graphData} />
```

### Dynamic Data

```javascript
// Build graph data programmatically
const graphData = {
  nodes: [
    { id: '1', data: { label: 'Start', nodeType: 'Source' } },
    { id: '2', data: { label: 'Process', nodeType: 'Transform' } },
    { id: '3', data: { label: 'End', nodeType: 'Sink' } }
  ],
  edges: [
    { id: 'e1', source: '1', target: '2' },
    { id: 'e2', source: '2', target: '3' }
  ]
};

<Visualizer graphData={graphData} />
```

## Error Handling

The visualizer gracefully handles:
- **Empty graphs** - Shows empty canvas
- **Invalid node types** - Defaults to 'Transform' styling
- **Missing edges** - Shows nodes without connections
- **Layout failures** - Falls back to original positions

## Browser Compatibility

- **Modern browsers** - Chrome, Firefox, Safari, Edge (latest versions)
- **React 16.8+** - Requires React hooks support
- **ES2017+** - Uses modern JavaScript features

## Dependencies

- `@xyflow/react` - ReactFlow v12 for graph rendering
- `elkjs` - ELK layout algorithms
- `react` - React framework (peer dependency)

