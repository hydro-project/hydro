# Visualizer Components

This directory contains the modular components for the Hydro graph visualizer.

## Structure

### Core Components
- **`ReactFlowVisualization.js`** - Main wrapper component that handles library loading
- **`GraphCanvas.js`** - Main graph rendering component with state management
- **`ReactFlowInner.js`** - Inner ReactFlow component with hooks

### UI Components
- **`FileDropZone.js`** - File upload interface
- **`Legend.js`** - Color and node type legend
- **`LayoutControls.js`** - Layout algorithm and color palette controls
- **`CustomNodes.js`** - Custom ReactFlow node components (ContainerNode, LabelNode)

### Utilities
- **`externalLibraries.js`** - CDN library loader for ReactFlow and ELK.js
- **`colorUtils.js`** - Color palettes and generation functions
- **`layoutConfigs.js`** - ELK.js layout algorithm configurations
- **`layoutAlgorithms.js`** - Graph layout algorithms including hierarchical layout

### Entry Points
- **`index.js`** - Central export file for all components
- **`../pages/visualizer.js`** - Main page component (refactored to use modules)

## Import Examples

```javascript
// Import specific components
import { ReactFlowVisualization } from '../components/visualizer/ReactFlowVisualization.js';
import { colorPalettes, generateNodeColors } from '../components/visualizer/colorUtils.js';

// Or import from index
import { ReactFlowVisualization, colorPalettes } from '../components/visualizer/index.js';
```

## Critical Implementation Notes

### Infinite Re-render Fix
The `onNodesChange` handler in `GraphCanvas.js` filters out 'dimensions' type changes to prevent ReactFlow's automatic dimension calculations from creating feedback loops.

### Container Click Fix
`ContainerNode` components use `onPointerDown` instead of `onMouseDown` because ReactFlow intercepts mousedown events for drag/selection but allows pointer events through.

Both fixes are critical for proper functionality - do not modify without understanding the root causes.
