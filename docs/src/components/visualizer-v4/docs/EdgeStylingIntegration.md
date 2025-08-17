# Edge Styling System - Complete Integration Guide

The visualizer-v4 now supports comprehensive edge styling based on semantic properties from the Hydro code. This system allows the Rust/Hydro side to define edge visual styles through JSON, which are then applied by the TypeScript visualizer.

## Overview

The edge styling system consists of several components working together:

1. **Rust/Hydro Side**: Generates JSON with `edgeStyleConfig` and `edgeProperties` on edges
2. **JSONParser**: Extracts edge style configuration from JSON
3. **EdgeBridge**: Converts semantic properties to ReactFlow edge styles
4. **ReactFlowBridge**: Integrates EdgeBridge for edge conversion
5. **FlowGraph**: Applies edge styling through configuration

## JSON Format

The Hydro code generates JSON with this structure:

```json
{
  "nodes": [...],
  "edges": [
    {
      "id": "edge1",
      "source": "node1", 
      "target": "node2",
      "edgeProperties": ["Network", "Bounded"],
      "label": "data flow"
    }
  ],
  "edgeStyleConfig": {
    "propertyMappings": {
      "Network": {
        "reactFlowType": "floating",
        "style": {
          "stroke": "#2563eb",
          "strokeWidth": 3,
          "strokeDasharray": "5,5"
        },
        "animated": true,
        "label": "NET"
      },
      "Bounded": {
        "reactFlowType": "floating", 
        "style": {
          "stroke": "#16a34a",
          "strokeWidth": 2
        },
        "animated": false,
        "label": "B"
      }
    },
    "defaultStyle": {
      "reactFlowType": "floating",
      "style": {
        "stroke": "#999999",
        "strokeWidth": 2
      },
      "animated": false
    },
    "combinationRules": {
      "priority": ["Network", "Bounded"],
      "description": "Network takes priority over Bounded"
    }
  }
}
```

## Usage

### Basic Usage

```typescript
import { 
  parseGraphJSON, 
  createRenderConfig, 
  FlowGraph 
} from 'visualizer-v4';

// Parse JSON with edge style configuration
const parseResult = parseGraphJSON(hydroJson);

// Create render config that includes edge styling
const renderConfig = createRenderConfig(parseResult, {
  fitView: true,
  enableControls: true
});

// Render with edge styling
<FlowGraph 
  visualizationState={parseResult.state}
  config={renderConfig}
/>
```

### Advanced Usage

```typescript
// Override or extend edge style configuration
const customConfig = createRenderConfig(parseResult, {
  fitView: true,
  edgeStyleConfig: {
    ...parseResult.metadata.edgeStyleConfig,
    propertyMappings: {
      ...parseResult.metadata.edgeStyleConfig?.propertyMappings,
      "CustomProperty": {
        reactFlowType: "floating",
        style: {
          stroke: "#8b5cf6",
          strokeWidth: 4
        },
        animated: true
      }
    }
  }
});
```

## Edge Properties

Edges can have multiple semantic properties from Hydro:

- **Network**: Network communication edges
- **Cycle**: Cyclic data flow edges  
- **Bounded**: Finite data streams
- **Unbounded**: Infinite data streams
- **NoOrder**: Unordered data
- **TotalOrder**: Ordered data
- **Keyed**: Key-value pairs

## Style Priority System

When edges have multiple properties, the style is determined by:

1. **Priority Order**: Defined in `combinationRules.priority`
2. **First Match**: If no priority property, use first property with mapping
3. **Default Style**: If no properties have mappings

Example priority: `["Cycle", "Network", "Bounded", "Unbounded"]`

## Floating Edges

The visualizer uses **floating edges** exclusively, which means:

- Edges don't connect to specific handles on nodes
- No `sourceHandle` or `targetHandle` properties
- Automatic connection point calculation by ReactFlow
- Better for complex layouts with many connections

## Style Properties

Each property mapping supports:

```typescript
{
  reactFlowType: "floating",  // Always "floating" for this visualizer
  style: {
    stroke: "#color",         // Edge color
    strokeWidth: number,      // Edge thickness
    strokeDasharray: "5,5",   // Dash pattern (optional)
    // ... other CSS properties
  },
  animated: boolean,          // Animation on/off
  label: "ABBREV"            // Short label for property
}
```

## Edge Labels

The system automatically creates edge labels by:

1. Combining original edge label with property abbreviations
2. Format: `"original label [ABC]"` where ABC are property abbreviations
3. Can be disabled by setting `showPropertyLabels: false`

## Debugging

Get edge style statistics:

```typescript
import { getEdgeStyleStats } from 'visualizer-v4';

const stats = getEdgeStyleStats(edges, edgeStyleConfig);
console.log('Property counts:', stats.propertyCounts);
console.log('Unmapped properties:', stats.unmappedProperties);
```

## Architecture

```
Hydro/Rust → JSON with edgeStyleConfig
     ↓
JSONParser → ParseResult with metadata.edgeStyleConfig  
     ↓
createRenderConfig → RenderConfig with edgeStyleConfig
     ↓  
FlowGraph → ReactFlowBridge.setEdgeStyleConfig()
     ↓
ReactFlowBridge → EdgeBridge.convertEdgesToReactFlow()
     ↓
EdgeBridge → Styled ReactFlow edges
```

## Key Benefits

1. **Semantic Accuracy**: Visual styles reflect actual Hydro properties
2. **Consistent Styling**: Same properties always look the same
3. **Extensible**: Easy to add new properties and styles
4. **Debugging**: Clear mapping from properties to visual styles
5. **Priority System**: Handles multiple properties gracefully

This system ensures that the visualizer accurately represents the semantic meaning of edges as determined by the Hydro code, while remaining flexible and extensible for future requirements.
