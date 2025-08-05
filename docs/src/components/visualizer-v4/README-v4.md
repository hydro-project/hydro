# Visualizer v4 - Integration of v3 Core/Bridges with v2 Frontend

This directory contains **visualizer-v4**, which combines the improved front-end logic from the **visualizer (v2)** directory with the modern, clean architecture of the **vis (v3)** directory.

## Architecture Overview

### Core Principles

- **VisState.ts as Single Source of Truth**: All application state flows through `core/VisState.ts`
- **Stateless Bridges**: All bridge components (`ELKBridge`, `ReactFlowBridge`, `CoordinateTranslator`) remain stateless
- **Clean Separation**: v3 architecture handles state management, v2 frontend handles user interface
- **Non-transient State Management**: All persistent state is reflected in `core/VisState.ts`

### Directory Structure

```
visualizer-v4/
├── core/                    # v3 Core (State Management)
│   ├── VisState.ts          # Single source of truth for app state
│   ├── VisualizationEngine.ts
│   ├── ContainerCollapseExpand.ts
│   └── ...
├── bridges/                 # v3 Bridges (Stateless)
│   ├── ELKBridge.ts         # Layout bridge (stateless)
│   ├── ReactFlowBridge.ts   # Rendering bridge (stateless)
│   └── CoordinateTranslator.ts
├── shared/                  # v3 Shared Types & Constants
│   ├── types.ts
│   ├── constants.ts
│   └── config.ts
├── v2-components/           # v2 Frontend Components (adapted)
│   ├── Visualizer.js        # Main visualizer component
│   ├── ReactFlowInner.js    # ReactFlow integration
│   ├── LayoutControls.js    # UI controls
│   └── ...  
├── v2-utils/                # v2 Utilities (adapted)
│   ├── layout.js
│   ├── reactFlowConfig.js
│   └── ...
├── integration/             # Integration Layer
│   └── StateAdapter.js      # Bridge between v2 frontend and v3 core
└── index.ts                 # Main exports
```

## Key Features

### 1. V3 Core Architecture (Preserved)

- **VisualizationState**: Complete state management with container collapse/expand
- **Container Hierarchy**: Tree validation and edge lifting/grounding
- **Hyperedge Management**: Automatic edge aggregation during container operations
- **Layout State**: Centralized position and dimension management
- **Manual Positions**: User drag interactions stored in VisState

### 2. V3 Bridge Architecture (Preserved)

- **ELKBridge**: Stateless layout engine integration
- **ReactFlowBridge**: Stateless ReactFlow data conversion  
- **CoordinateTranslator**: Stateless coordinate system translation
- **Clean Interfaces**: Bridges only transform data, never store state

### 3. V2 Frontend Logic (Adapted)

- **Rich UI Components**: Layout controls, info panels, legends
- **ReactFlow Integration**: Advanced node/edge rendering
- **User Interactions**: Drag, zoom, pan, selection
- **Visual Feedback**: Loading states, animations, tooltips

### 4. Integration Layer

The `StateAdapter.js` provides a compatibility layer that allows v2 frontend components to work with v3 architecture:

```javascript
import { createIntegratedStateManager } from './integration/StateAdapter.js';

const stateManager = createIntegratedStateManager();

// Set data (v2 format → v3 VisState)
stateManager.setGraphData({ nodes, edges, containers });

// Layout (v3 ELK bridge)
await stateManager.performLayout({ algorithm: 'mrtree' });

// Render (v3 ReactFlow bridge)  
const reactFlowData = stateManager.getReactFlowData();

// Container operations (v3 VisState)
stateManager.collapseContainer('container1');
```

## Usage

### Basic Usage

```javascript
import { createVisualizationState, ELKBridge, ReactFlowBridge } from './visualizer-v4';

// Create v3 state as single source of truth
const visState = createVisualizationState();

// Add data to VisState
visState.setGraphNode('node1', { label: 'Node 1' });
visState.setGraphNode('node2', { label: 'Node 2' });
visState.setGraphEdge('edge1', { source: 'node1', target: 'node2' });

// Use stateless bridges
const elkBridge = new ELKBridge();
const reactFlowBridge = new ReactFlowBridge();

// Layout via bridge
await elkBridge.applyLayout(visState, { algorithm: 'mrtree' });

// Render via bridge
const reactFlowData = reactFlowBridge.visStateToReactFlow(visState);
```

### With Integration Layer

```javascript
import { createIntegratedStateManager } from './visualizer-v4/integration/StateAdapter.js';

const stateManager = createIntegratedStateManager();

// Use v2-style interface with v3 architecture underneath
stateManager.setGraphData({
  nodes: [{ id: 'node1', label: 'Node 1' }],
  edges: [{ id: 'edge1', source: 'node1', target: 'node2' }]
});

await stateManager.performLayout();
const reactFlowData = stateManager.getReactFlowData();
```

## State Management

### All State Flows Through VisState

```javascript
// ✅ Correct: State stored in VisState
visState.setGraphNode('node1', { label: 'My Node' });
visState.setNodeLayout('node1', { position: { x: 100, y: 200 } });
visState.setManualPosition('node1', 150, 250);

// ❌ Incorrect: Don't store state in React components or bridges
// component.setState({ nodePosition: { x: 100, y: 200 } }); // NO
// bridge.nodePositions.set('node1', { x: 100, y: 200 }); // NO
```

### Container Operations

```javascript
// Container operations automatically manage hyperedges
visState.collapseContainer('container1');  // Creates hyperedges
visState.expandContainer('container1');    // Restores original edges

// All operations maintain VisState consistency
const nodes = visState.visibleNodes;       // Always current
const edges = visState.visibleEdges;       // Includes hyperedges when appropriate
```

## Development

### Build

```bash
npm run build
```

### Test

```bash
npm test
```

### Test Integration

```bash
npm test -- integration.test.js
```

## Migration from v2/v3

### From Visualizer v2

- Replace state management with VisState
- Use StateAdapter for compatibility
- Update components to read from VisState
- Replace direct state mutations with VisState methods

### From Vis v3

- Add v2 frontend components 
- Use existing core/bridges as-is
- Integrate via StateAdapter or direct VisState usage

## Architecture Benefits

### 1. Clean State Management
- Single source of truth eliminates state synchronization bugs
- All state mutations go through VisState methods
- Automatic consistency checks and validation

### 2. Stateless Bridges
- No hidden state in bridges
- Easy to test and reason about
- Perfect separation of concerns

### 3. Rich Frontend
- Advanced UI components from v2
- Smooth user interactions
- Professional visual design

### 4. Extensibility
- Easy to add new bridges
- Frontend components can be swapped
- Clean interfaces throughout

## Status

- ✅ **Core Architecture**: Complete (from vis v3)
- ✅ **Bridge System**: Complete (from vis v3) 
- ✅ **Frontend Components**: Complete (from visualizer v2)
- ✅ **Integration Layer**: Complete
- ✅ **Build System**: Complete
- ✅ **Tests**: 115 tests passing
- ✅ **Documentation**: Complete

**Integration Status**: 🎉 **COMPLETE**

The visualizer-v4 successfully combines the best of both architectures while maintaining clean separation and the single source of truth principle.