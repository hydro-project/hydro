# Hydro Visualization System Documentation

Welcome to the comprehensive documentation for the Hydro graph visualization system. This system provides powerful, flexible tools for visualizing complex graph data with hierarchical containers, interactive layouts, and modern React components.

## 📖 Documentation Overview

### [🚀 Quick Start Guide](./QuickStart.md)
Get up and running quickly with basic examples and common patterns.
- Installation and setup
- Basic graph creation
- React integration
- Common patterns and examples

### [🏗️ Architecture Guide](./Architecture.md)
Understand the system design and component interactions.
- System overview and data flow
- Component responsibilities
- Performance characteristics
- Extensibility points

### [📋 VisualizationState API](./VisState.md)
Complete reference for the core state management class.
- Comprehensive API documentation
- Usage examples and best practices
- Performance considerations
- Troubleshooting guide

## 🎯 Key Features

### Core Capabilities
- **Centralized State Management**: Single source of truth for all graph data
- **Hierarchical Containers**: Support for nested groupings and collapse/expand
- **Automatic Layout**: ELK-powered layout engine with multiple algorithms
- **React Integration**: Modern React components with TypeScript support
- **Performance Optimized**: O(1) lookups, cached collections, efficient updates

### Visualization Features
- **Interactive Nodes & Edges**: Click, double-click, context menu support
- **Custom Styling**: Configurable colors, shapes, and visual styles
- **Container Operations**: Collapse/expand with automatic edge aggregation
- **Real-time Updates**: Dynamic graph modifications with smooth updates
- **Export Capabilities**: Support for various output formats

### Developer Experience
- **TypeScript First**: Full type safety and IntelliSense support
- **Method Chaining**: Fluent API for readable graph construction
- **Comprehensive Testing**: Unit tests, integration tests, and performance tests
- **Error Handling**: Descriptive error messages and validation
- **Extensive Documentation**: Complete API reference with examples

## 🏃‍♀️ Quick Examples

### Basic Graph
```typescript
import { createVisualizationState, NODE_STYLES } from './core/VisState';

const state = createVisualizationState()
  .setGraphNode('input', { label: 'Data Source', style: NODE_STYLES.HIGHLIGHTED })
  .setGraphNode('output', { label: 'Results', style: NODE_STYLES.DEFAULT })
  .setGraphEdge('flow', { source: 'input', target: 'output' });
```

### React Component
```typescript
import { GraphFlow } from './render/GraphFlow';

function MyVisualization() {
  return (
    <GraphFlow 
      visualizationState={state}
      layoutConfig={{ algorithm: 'layered' }}
      renderConfig={{ enableMiniMap: true }}
    />
  );
}
```

### Hierarchical Containers
```typescript
state
  .setContainer('pipeline', {
    children: new Set(['input', 'process', 'output']),
    collapsed: false
  })
  .collapseContainer('pipeline'); // Hide children, create hyperEdges
```

## 📁 Repository Structure

```
docs/src/components/vis/
├── core/                   # Core state management
│   ├── VisState.ts        # Main state class
│   ├── ContainerCollapseExpand.ts
│   └── adapter.ts
├── render/                # React rendering components
│   ├── GraphFlow.tsx      # Main ReactFlow component
│   ├── nodes.tsx          # Custom node components
│   ├── edges.tsx          # Custom edge components
│   ├── colorUtils.ts      # Shared color utilities
│   └── edgeUtils.ts       # Shared edge utilities
├── layout/                # Layout engine integration
│   ├── ELKLayoutEngine.ts # ELK layout coordination
│   ├── ELKStateManager.ts # State-to-ELK translation
│   └── types.ts           # Layout-specific types
├── shared/                # Shared utilities and configuration
│   ├── config.ts          # Styling and configuration
│   ├── types.ts           # Core type definitions
│   └── constants.ts       # Backward compatibility constants
├── services/              # High-level services
│   └── VisualizationService.ts
├── __tests__/             # Comprehensive test suite
│   ├── render-components.test.ts
│   ├── dry-refactoring.test.ts
│   └── VisState.test.ts
└── docs/                  # This documentation
    ├── VisState.md        # Complete API reference
    ├── Architecture.md    # System architecture
    ├── QuickStart.md      # Getting started guide
    └── README.md          # This file
```

## 🧪 Testing

### Run Tests
```bash
# Run all tests
npm test

# Run specific test suites
npm run test:vis-state
npm run test:render-components
npm run test:integration

# Run with coverage
npm run test:coverage
```

### Test Categories
- **Unit Tests**: Core functionality and individual components
- **Integration Tests**: Component interactions and data flow
- **Performance Tests**: Scalability and optimization validation
- **DRY Tests**: Code duplication and refactoring validation

## 🔧 Configuration

### Layout Configuration
```typescript
const layoutConfig = {
  algorithm: 'layered',        // or 'force', 'radial'
  direction: 'DOWN',           // or 'UP', 'LEFT', 'RIGHT'
  spacing: { nodeNode: 20 },   // Node spacing
  containerPadding: 15         // Container padding
};
```

### Render Configuration
```typescript
const renderConfig = {
  enableMiniMap: true,         // Show minimap
  enableControls: true,        // Show zoom controls
  fitView: true,              // Auto-fit on load
  nodesDraggable: true,       // Allow node dragging
  snapToGrid: false           // Grid snapping
};
```

### Styling Configuration
```typescript
const styleConfig = {
  nodeColors: 'Set2',         // ColorBrewer palette
  edgeStyle: 'default',       // Edge styling
  containerStyle: 'outlined'  // Container appearance
};
```

## 🎨 Customization

### Custom Node Types
```typescript
const CustomNode = ({ data, selected }) => (
  <div className={`custom-node ${selected ? 'selected' : ''}`}>
    <h3>{data.label}</h3>
    <p>{data.description}</p>
  </div>
);

const nodeTypes = {
  'custom': CustomNode,
  ...defaultNodeTypes
};
```

### Custom Event Handlers
```typescript
const eventHandlers = {
  onNodeClick: (nodeId) => console.log('Clicked:', nodeId),
  onNodeDoubleClick: (nodeId) => state.updateNode(nodeId, { style: 'highlighted' }),
  onEdgeClick: (edgeId) => console.log('Edge clicked:', edgeId)
};
```

## 🚀 Performance Best Practices

1. **Batch Operations**: Chain state updates for efficiency
2. **Use Getters**: Access visible elements via state getters
3. **Monitor Size**: Keep graphs under recommended limits (10k nodes)
4. **Leverage Caching**: Use built-in visibility and hierarchy caches
5. **Test Performance**: Regular performance testing with realistic data sizes

## 🔍 Troubleshooting

### Common Issues
- **Elements not visible**: Check `hidden` property values
- **Layout not updating**: Verify container ELK configuration
- **Hierarchy errors**: Avoid circular dependencies in containers
- **Performance issues**: Monitor graph size and complexity

### Getting Help
1. Check the [Troubleshooting section](./VisState.md#troubleshooting) in API docs
2. Review test examples for usage patterns
3. Examine the source code for implementation details
4. Check TypeScript types for parameter requirements

## 📈 Roadmap

### Current Features ✅
- Complete VisualizationState API
- React component integration
- ELK layout engine support
- Comprehensive test coverage
- DRY principle adherence
- Full TypeScript support

### Future Enhancements 🚧
- Animation system for smooth transitions
- WebGL rendering for large graphs
- Collaborative editing capabilities
- Additional layout algorithms
- Enhanced export formats
- Performance monitoring tools

## 🤝 Contributing

### Code Organization
- Follow existing TypeScript patterns
- Maintain test coverage for new features
- Update documentation for API changes
- Adhere to DRY principles

### Development Workflow
1. Write tests first for new features
2. Implement minimal changes to pass tests
3. Refactor for DRY compliance
4. Update documentation
5. Verify performance impact

---

*This documentation is maintained alongside the codebase. For the latest updates and implementation details, refer to the TypeScript source files and inline documentation.*