# Vis - Next Generation Hydro Graph Visualizer

A modern, efficient visualization system for Hydro graphs with support for hierarchical containers, edge routing, and dynamic collapse/expand operations. **Now with full TypeScript support!**

## Core Components

### VisualizationState (`VisState.ts`)
The main state management class that handles:
- **Graph Nodes**: Basic graph nodes with styling and visibility
- **Graph Edges**: Connections between nodes with automatic visibility management
- **Containers**: Hierarchical groupings of nodes with collapse/expand functionality
- **HyperEdges**: Automatically generated edges for collapsed containers
- **Efficient Access**: Optimized Maps for quick access to visible elements

### Constants & Types (`constants.ts`)
Centralized styling and layout constants with full TypeScript support:
- Node styles (DEFAULT, HIGHLIGHTED, SELECTED, WARNING, ERROR)
- Edge styles (DEFAULT, HIGHLIGHTED, DASHED, THICK, WARNING)
- Container styles and layout dimensions
- **Type definitions** for all interfaces and configurations

## Key Features

### 🎯 **Smart Edge Management**
- Edges automatically hide when endpoints are hidden
- HyperEdges created automatically when containers collapse
- Incremental updates (no full rebuilds)

### 📦 **Hierarchical Containers**
- Nested container support
- Recursive collapse/expand with depth-first processing
- Automatic child state management

### ⚡ **Performance Optimized**
- Separate Maps for visible elements (no filtering needed)
- Efficient edge-to-node mapping for quick lookups
- Minimal DOM updates through smart state tracking

### 🔄 **Consistent State Management**
- Automatic constraint enforcement
- Clean transition logic
- Immutable operation patterns

### 🚀 **TypeScript Benefits**
- **Compile-time type safety** prevents runtime errors
- **IntelliSense support** with auto-completion
- **Clear API contracts** through interface definitions
- **Better refactoring** with IDE support

## Usage Example

```typescript
import { 
  createVisualizationState, 
  NODE_STYLES, 
  EDGE_STYLES,
  type GraphNode,
  type CreateNodeProps 
} from './vis';

const state = createVisualizationState();

// Add nodes with type safety
const nodeProps: CreateNodeProps = {
  label: 'My Node', 
  style: NODE_STYLES.DEFAULT 
};
const node: GraphNode = state.setGraphNode('node1', nodeProps);

state.setGraphNode('node2', { 
  label: 'Another Node', 
  style: NODE_STYLES.HIGHLIGHTED 
});

// Add edges with auto-completion
state.setGraphEdge('edge1', {
  source: 'node1',
  target: 'node2',
  style: EDGE_STYLES.THICK
});

// Create container with type-checked dimensions
state.setContainer('container1', {
  children: ['node1', 'node2'],
  expandedDimensions: { width: 200, height: 150 }
});

// Collapse container (creates hyperEdges automatically)
state.collapseContainer('container1');

// Get visible elements for rendering - all properly typed
const visibleNodes: GraphNode[] = state.getVisibleNodes();
const visibleEdges = state.getVisibleEdges();
const hyperEdges = state.getHyperEdges();
```

## TypeScript Integration

### Type Safety Benefits
```typescript
// Compile-time error prevention
state.setGraphNode('node1', {
  label: 'My Node',
  style: 'invalid-style' // ❌ TypeScript error: not assignable to NodeStyle
});

state.setGraphNode('node1', {
  label: 'My Node',
  style: NODE_STYLES.WARNING // ✅ Valid: type-checked constant
});

// Auto-completion for method parameters
state.setContainer('container1', {
  expandedDimensions: { 
    width: 200,
    height: 150
  },
  // ✨ IDE shows available properties: collapsed, hidden, children, etc.
});
```

### Interface Definitions
```typescript
interface CreateNodeProps {
  label: string;
  style?: NodeStyle;
  hidden?: boolean;
  [key: string]: any; // Allow custom properties
}

interface GraphNode extends BaseEntity {
  label: string;
  style: NodeStyle;
}
```

## Testing

The project includes comprehensive unit tests covering all functionality:

```bash
# Run all unit tests
npm test

# Run specific test suites
npm run test:vis-state          # Core state management tests
npm run test:constants          # Constants and configuration tests  
npm run test:json-parser        # JSON parsing functionality tests

# Run integration tests with real data
npm run test:integration        # End-to-end tests with Hydro graph data

# Run fuzz tests for robustness
npm run test:fuzz               # Randomized collapse/expand operations

# Run everything
npm run test:all                # All tests including integration and fuzz
```

### Test Coverage
- ✅ **Unit Tests**: State creation and initialization
- ✅ **Node Management**: Create, update, hide, remove operations
- ✅ **Edge Management**: Automatic node mapping and visibility rules
- ✅ **Container Hierarchy**: Child tracking and nested containers
- ✅ **HyperEdge Management**: Creation, removal, and style aggregation
- ✅ **Container Transitions**: Collapse/expand with edge rerouting
- ✅ **JSON Parsing**: Real Hydro graph data parsing and validation
- ✅ **Integration Tests**: End-to-end functionality with real datasets
- ✅ **Fuzz Testing**: Randomized operations with invariant checking

### Fuzz Testing

The fuzz test performs randomized collapse/expand operations while maintaining system invariants:

- **Node Visibility**: Nodes are visible ⟺ not hidden and no parent container collapsed
- **Edge Visibility**: Edges are visible ⟺ both endpoints are visible  
- **HyperEdge Consistency**: HyperEdges only exist for collapsed containers
- **Collection Consistency**: Visible collections match actual visibility
- **Hierarchy Consistency**: Parent-child relationships are maintained
- **Mapping Consistency**: Node-to-edge mappings are accurate

### Real Data Testing

Integration tests use actual Hydro graph JSON files:
- `chat.json` - Chat application graph with location and backtrace hierarchies
- `paxos.json` - Paxos consensus algorithm graph
- Multiple grouping scenarios (location-based, backtrace-based)
- Performance validation for parsing and operations
- State consistency across complex operation sequences

## Architecture Principles

### 🏗️ **Separation of Concerns**
- State management separate from rendering
- Constants separate from logic
- Clean interfaces between components

### 📏 **Constraint-Driven Design**
- Automatic enforcement of visibility rules
- Consistent state transitions
- Predictable behavior patterns

### 🚀 **Performance First**
- O(1) lookups for visible elements
- Incremental updates only
- Minimal memory allocation during operations

## Development

### File Structure
```
vis/
├── VisState.js          # Core state management
├── constants.js         # Style and layout constants
├── index.js            # Public API exports
├── package.json        # Test scripts
├── README.md           # This file
└── __tests__/          # Test suite
    ├── VisState.test.js
    ├── constants.test.js
    └── runTests.js
```

### Adding New Features
1. Add functionality to `VisState.js`
2. Add any new constants to `constants.js`
3. Update exports in `index.js`
4. Add comprehensive tests
5. Update this README

The visualization system is designed to be extended with additional layout engines, renderers, and interaction handlers while maintaining the core state management principles.

## Style Tuner Panel

The visualization system includes a dockable Style Tuner panel that allows interactive adjustment of visual properties in real-time.

### Features

- **Edge Styling**: Change edge style (bezier/straight/smoothstep), color, width, and dashed patterns
- **Node Styling**: Adjust node border radius, padding, and font size
- **Container Styling**: Modify container border radius, border width, and shadow effects
- **Real-time Updates**: All changes are applied immediately to the visualization
- **Dockable Interface**: Panel can be docked, undocked, and collapsed as needed

### Usage

The Style Tuner panel is available on the `/vis` page and is docked to the top-right by default.

#### Edge Controls
- **Edge Style**: Choose between bezier curves, straight lines, or smooth step routing
- **Edge Color**: Pick color for both edge lines and arrowheads
- **Edge Width**: Adjust thickness of edge lines (1-8px)
- **Edge Dashed**: Toggle dashed line style

#### Node Controls  
- **Node Border Radius**: Control roundness of node corners (0-24px)
- **Node Padding**: Adjust internal spacing within nodes (4-32px)
- **Node Font Size**: Change text size in nodes (10-20px)

#### Container Controls
- **Container Border Radius**: Control roundness of container corners (0-24px)  
- **Container Border Width**: Adjust thickness of container borders (1-6px)
- **Container Shadow**: Choose shadow intensity (None/Light/Medium/Large)

### Implementation

```typescript
import { StyleTunerPanel } from '@site/src/components/visualizer-v4';

// Basic usage
<StyleTunerPanel 
  value={styleConfig} 
  onChange={setStyleConfig}
  defaultCollapsed={false}
/>

// Style config structure
const styleConfig = {
  edgeStyle: 'bezier' | 'straight' | 'smoothstep',
  edgeColor: '#1976d2',
  edgeWidth: 2,
  edgeDashed: false,
  nodeBorderRadius: 4,
  nodePadding: 12,
  nodeFontSize: 12,
  containerBorderRadius: 8,
  containerBorderWidth: 2,
  containerShadow: 'LIGHT' | 'MEDIUM' | 'LARGE' | 'NONE'
};
```

The Style Tuner integrates with the `StyleConfigProvider` to ensure all components receive consistent styling updates.
