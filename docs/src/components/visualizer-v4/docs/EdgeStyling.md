# Edge Styling System

The visualizer-v4 system supports comprehensive edge styling through JSON configuration and the StyleTuner UI panel. This document describes all available edge styles and how to use them.

## Available Edge Styles

### 1. Default
- **Style**: `default`
- **Appearance**: Blue edge with standard width (2px)
- **Usage**: Applied when no specific style is set

### 2. Thick
- **Style**: `thick`
- **Appearance**: Red edge with thick width (6px)
- **Usage**: For emphasizing important connections

### 3. Dashed
- **Style**: `dashed`
- **Appearance**: Purple dashed edge
- **Usage**: For showing conditional or optional connections

### 4. Warning
- **Style**: `warning`
- **Appearance**: Orange edge with medium width (3px)
- **Usage**: For highlighting problematic or attention-requiring connections

### 5. Highlighted
- **Style**: `highlighted`
- **Appearance**: Green edge with medium width (3px)
- **Usage**: For emphasizing successful or active connections

### 6. Animated
- **Style**: `animated`
- **Appearance**: Cyan edge with dashed pattern and animation
- **Usage**: For showing active data flow or dynamic connections

## StyleTuner Integration

The StyleTuner panel provides a user-friendly interface for configuring edge styles:

### Edge Routing Section
- **Edge Style**: Controls the path routing (Bezier, Straight, SmoothStep)

### Edge Appearance Section
- **Edge Semantic Style**: Choose from predefined styles (Default, Thick, Dashed, Warning, Highlighted, Animated)
- **Edge Color**: Manual color picker for custom colors
- **Edge Width**: Range slider for edge thickness
- **Edge Dashed**: Checkbox to make edges dashed

### How StyleTuner Works
1. **Semantic Style Priority**: When a semantic style is selected (e.g., "Thick"), it overrides the basic edge settings
2. **Fallback to Basic Settings**: When "Default" semantic style is selected, the basic edge color, width, and dashed settings are used
3. **JSON Override**: Per-edge JSON styling always takes highest precedence over StyleTuner settings

## JSON Configuration

### Using styleConfig

```json
{
  "styleConfig": {
    "edges": {
      "thick": {
        "style": "thick"
      },
      "dashed": {
        "style": "dashed" 
      },
      "warning": {
        "style": "warning"
      },
      "highlighted": {
        "style": "highlighted"
      },
      "animated": {
        "style": "animated"
      }
    }
  },
  "semanticTags": {
    "edge_id_1": ["thick"],
    "edge_id_2": ["dashed"],
    "edge_id_3": ["warning"],
    "edge_id_4": ["highlighted"],
    "edge_id_5": ["animated"]
  }
}
```

### Direct Edge Styling

You can also apply styles directly to edge objects using the `style` property:

```json
{
  "edges": [
    {
      "id": "e1",
      "source": "n1",
      "target": "n2",
      "style": {
        "strokeWidth": 6,
        "stroke": "#ff6b6b"
      }
    },
    {
      "id": "e2", 
      "source": "n1",
      "target": "n3",
      "style": {
        "strokeDasharray": "8,4",
        "stroke": "#9c27b0"
      }
    },
    {
      "id": "e3",
      "source": "n1", 
      "target": "n4",
      "animated": true,
      "style": {
        "stroke": "#00bcd4",
        "strokeDasharray": "5,5"
      }
    }
  ]
}
```

## Style Precedence

The system applies styles in the following order of precedence (highest to lowest):

1. **Per-edge semantic tags** (from styleConfig + semanticTags in JSON)
2. **Direct edge style properties** (edge.style object in JSON)
3. **StyleTuner semantic style** (selected via UI when set to non-default)
4. **StyleTuner basic settings** (color, width, dashed - used when semantic style is "Default")
5. **Default system values**

### StyleTuner Behavior
- When **semantic style ≠ "Default"**: Predefined semantic style overrides basic settings
- When **semantic style = "Default"**: Basic edge color, width, and dashed settings are applied
- **JSON styling always wins**: Per-edge JSON configuration overrides all StyleTuner settings

This ensures that:
- Users can set global defaults via StyleTuner
- Specific edges can be styled via JSON configuration
- JSON styling has ultimate precedence for precise control

## Integration with ReactFlow

The system uses ReactFlow's edge rendering components:

- **FloatingEdge**: Primary edge component that calculates dynamic attachment points on node perimeters

All edge types (including hyperedges) are now rendered using the unified FloatingEdge component.

All components support the same styling API through:
- `stroke`: Edge color
- `strokeWidth`: Edge thickness
- `strokeDasharray`: Dash pattern for dashed lines
- Animation support through ReactFlow's built-in edge animation

## Example Files

- **`edge-styles-demo.json`**: Complete demonstration of all 6 edge styles using JSON semantic tags
- **`simple-edge-styles.json`**: Minimal example showing 3 key styles (thick, dashed, animated)
- **`styletuner-test.json`**: Clean test file with edges that respond to StyleTuner settings
- **`styletuner-integration-demo.json`**: Demonstrates interaction between StyleTuner settings and JSON overrides

### Testing StyleTuner Integration

1. Load `styletuner-test.json` - a simple file with edges that have no explicit styling
2. Open the StyleTuner panel
3. Change the "Edge Semantic Style" to see how it affects all edges without JSON styling
4. Change "Edge Color", "Edge Width", and "Edge Dashed" to see basic styling effects
5. Load `styletuner-integration-demo.json` to see how JSON overrides work alongside StyleTuner
6. Notice that edges with JSON styling (direct or semantic) are not affected by StyleTuner changes

### Behavior Details

- **Edges without JSON styling**: Respond to all StyleTuner settings (both semantic and basic)
- **Edges with JSON semantic tags**: Use JSON-defined styles, ignore StyleTuner
- **Edges with direct JSON styling**: Use JSON-defined styles, ignore StyleTuner
- **StyleTuner semantic vs basic**: When semantic style ≠ "Default", it overrides basic edge settings

## Implementation Notes

- Edge styling is handled by the consolidated `edges.tsx` file containing the `FloatingEdge` component and shared styling logic
- Style detection from JSON objects is handled in `JSONParser.ts`
- Style constants are defined in `shared/config.ts`
- The system automatically detects edge styles from:
  - strokeWidth (≥3 = thick)
  - stroke color (red = warning)
  - strokeDasharray (any value = dashed)
  - animated property (true = animated)
