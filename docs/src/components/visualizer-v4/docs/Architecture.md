# Visualization System Architecture

## Overview

The Hydro visualization system is built on a modular architecture that separates concerns between state management, layout calculation, and rendering. This document provides an architectural overview of how the components work together.

## System Components

```
┌─────────────────────────────────────────────────────────────┐
│                     React UI Layer                          │
├─────────────────────────────────────────────────────────────┤
│                    Render Components                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ FlowGraph   │  │    Nodes    │  │       Edges         │  │
│  │   .tsx      │  │   .tsx      │  │      .tsx           │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                   Layout Engine                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ ELK Layout  │  │ State Mgr   │  │  Converters         │  │
│  │ Engine      │  │            │  │                     │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                   Core State                                │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              VisualizationState                         │ │
│  │          (Single Source of Truth)                      │ │
│  └─────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                   Data Layer                                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   JSON      │  │  Services   │  │    Adapters         │  │
│  │  Parser     │  │             │  │                     │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Data Flow

### 1. Input Processing
```
Raw Graph Data → JSON Parser → VisualizationState
                              ↓
                     Validation & Storage
```

### 2. Layout Calculation  
```
VisualizationState → ELK State Manager → ELK Layout Engine
       ↓                    ↓                    ↓
   Graph Elements    Layout Graph         Positioned Elements
```

### 3. Rendering Pipeline
```
Layout Results → ReactFlowBridge → React Components → DOM
    ↓                ↓                   ↓
  Positioned     ReactFlow Nodes     Visual Output
   Elements      & Edges
```

## Core Design Principles

### 1. Single Source of Truth
**VisualizationState** is the central authority for all graph data:
- All components read from VisualizationState
- All updates go through VisualizationState methods
- No direct manipulation of internal data structures
- Consistent state across all system components

### 2. Separation of Concerns

#### State Management (Core)
- **VisualizationState**: Central state management
- **ContainerCollapseExpand**: Hierarchical operations
- **Types**: Shared type definitions

#### Layout (Layout)
- **ELKLayoutEngine**: Layout calculation coordination
- **ELKStateManager**: State-to-ELK translation
- **Layout Types**: Layout-specific interfaces

#### Rendering (Render)
- **FlowGraph**: Main ReactFlow component
- **Nodes/Edges**: Custom React components
- **ReactFlowBridge**: Layout-to-ReactFlow translation

#### Data (Services/Shared)
- **JSONParser**: Input data processing
- **VisualizationService**: High-level coordination
- **Config**: Styling and configuration

### 3. Immutable Operations
- Methods return new state or `this` for chaining
- Internal collections use defensive copying
- No external mutation of internal data structures

### 4. Performance Optimization
- O(1) element lookups via Map-based storage
- Cached visible element collections
- Efficient hierarchy operations
- Minimal re-calculations

## Component Interactions

### VisualizationState ↔ Layout Engine

```typescript
// State provides data to layout
const nodes = visualizationState.visibleNodes;
const edges = visualizationState.visibleEdges;
const containers = visualizationState.visibleContainers;

// Layout applies results back to state
visualizationState.setNodeLayout(nodeId, { x, y, width, height });
visualizationState.setContainerLayout(containerId, layoutData);
```

### Layout Engine ↔ ReactFlow

```typescript
// Layout results converted to ReactFlow format
const layoutResult = await layoutEngine.layout(nodes, edges, containers);
const bridge = new ReactFlowBridge();
const reactFlowData = bridge.convert(layoutResult);

// ReactFlow renders the positioned elements
<ReactFlow nodes={reactFlowData.nodes} edges={reactFlowData.edges} />
```

### State Management Patterns

#### Centralized Updates
```typescript
// All updates go through VisualizationState
visualizationState
  .setGraphNode('node1', { label: 'Node 1' })
  .setGraphEdge('edge1', { source: 'node1', target: 'node2' })
  .setContainer('container1', { children: new Set(['node1']) });
```

#### Reactive Access
```typescript
// Components read reactive data
const visibleNodes = visualizationState.visibleNodes;
const expandedContainers = visualizationState.expandedContainers;
```

## Error Handling Strategy

### Validation Layers

1. **Input Validation**: JSONParser validates raw data format
2. **State Validation**: VisualizationState validates operations
3. **Layout Validation**: ELK integration validates layout constraints
4. **Render Validation**: ReactFlow components validate props

### Error Boundaries

```typescript
// VisualizationState throws descriptive errors
try {
  state.setGraphNode('', { label: 'Invalid' });
} catch (error) {
  // Handle validation error
}

// Layout engine handles ELK errors
try {
  const result = await layoutEngine.layout(...);
} catch (error) {
  // Fallback to previous layout or default positioning
}
```

## Performance Characteristics

### Time Complexity
- **Element Access**: O(1) - Map-based lookups
- **Visibility Queries**: O(1) - Cached collections
- **Hierarchy Operations**: O(1) - Direct Map access
- **Layout Calculation**: O(n log n) - ELK algorithm
- **Rendering**: O(n) - ReactFlow rendering

### Space Complexity
- **Element Storage**: O(n) - One Map per element type
- **Visibility Cache**: O(n) - Separate collections for visible elements
- **Hierarchy Index**: O(n) - Parent/child relationship maps
- **Layout Data**: O(n) - Position and dimension data

### Optimization Strategies

1. **Lazy Evaluation**: Visibility collections updated only when needed
2. **Incremental Updates**: Only changed elements trigger recalculation
3. **Memory Sharing**: Shared object references reduce duplication
4. **Batch Operations**: Multiple updates processed together

## Extensibility Points

### 1. Custom Node Types
```typescript
// Add new node component
const customNodeTypes = {
  'custom': CustomNodeComponent,
  ...nodeTypes
};
```

### 2. Layout Algorithms
```typescript
// Implement LayoutEngine interface
class CustomLayoutEngine implements LayoutEngine {
  async layout(nodes, edges, containers, config) {
    // Custom layout logic
  }
}
```

### 3. Data Sources
```typescript
// Implement data adapter
class CustomDataAdapter {
  parseData(rawData) {
    // Convert to VisualizationState format
  }
}
```

### 4. Event Handlers
```typescript
// Custom event handling
const customEventHandlers = {
  onNodeClick: (nodeId) => { /* custom logic */ },
  onEdgeClick: (edgeId) => { /* custom logic */ }
};
```

## Testing Strategy

### Unit Tests
- **VisualizationState**: Core state operations
- **Layout Engine**: Layout calculations
- **Converters**: Data transformations
- **Components**: React component behavior

### Integration Tests
- **State ↔ Layout**: Full layout pipeline
- **Layout ↔ Render**: Complete render pipeline
- **End-to-End**: Full visualization workflow

### Performance Tests
- **Large Graphs**: Scalability testing
- **Frequent Updates**: Update performance
- **Memory Usage**: Memory leak detection

## Configuration Management

### Styling Configuration
```typescript
// Centralized styling in shared/config.ts
export const NODE_COLORS = {
  source: '#8dd3c7',
  transform: '#ffffb3',
  sink: '#80b1d3'
};
```

### Layout Configuration
```typescript
// Layout algorithm settings
export const DEFAULT_LAYOUT_CONFIG = {
  algorithm: 'layered',
  spacing: { nodeNode: 20, edgeNode: 10 },
  direction: 'DOWN'
};
```

### Render Configuration
```typescript
// ReactFlow settings
export const DEFAULT_RENDER_CONFIG = {
  enableMiniMap: true,
  enableControls: true,
  fitView: true
};
```

## Future Considerations

### Scalability Improvements
1. **Virtual Rendering**: For very large graphs
2. **Level-of-Detail**: Simplified rendering at zoom levels
3. **Streaming Updates**: For real-time data

### Feature Extensions
1. **Animation System**: Smooth transitions
2. **Custom Layouts**: Domain-specific algorithms
3. **Collaborative Editing**: Multi-user support
4. **Export Formats**: PDF, SVG, PNG export

### Performance Optimizations
1. **Web Workers**: Background layout calculation
2. **Canvas Rendering**: Alternative to SVG for large graphs
3. **Incremental Layout**: Partial layout updates
4. **Memory Pooling**: Object reuse strategies

---

*This architectural overview reflects the current design as of the latest implementation. For specific implementation details, refer to the individual component documentation.*