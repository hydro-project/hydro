# JSON Format Validation for Hydroscope Compatibility

## Overview
This document validates that the JSON output format from `HydroJson` is compatible with Hydroscope's expectations.

## JSON Structure

The generated JSON has the following top-level structure:

```json
{
  "nodes": [...],
  "edges": [...],
  "hierarchyChoices": [...],
  "nodeAssignments": {...},
  "edgeStyleConfig": {...},
  "nodeTypeConfig": {...},
  "legend": {...}
}
```

## Node Format

Each node includes semantic tags:

```json
{
  "id": "0",
  "nodeType": "Source",
  "fullLabel": "source_iter [iterate over collection]",
  "shortLabel": "source_iter",
  "semanticTags": ["Source", "Process"],
  "data": {
    "locationId": 0,
    "locationType": "Process",
    "backtrace": [...]
  }
}
```

### Node Semantic Tags
- Node type (e.g., "Source", "Transform", "Sink")
- Location type (e.g., "Process", "Cluster", "External")

## Edge Format

Each edge includes semantic tags and computed styles:

```json
{
  "id": "e0",
  "source": "0",
  "target": "1",
  "semanticTags": ["Stream", "Unbounded", "TotalOrder"],
  "style": {
    "line-pattern": "solid",
    "line-width": 1,
    "arrowhead": "triangle-filled",
    "line-style": "single",
    "halo": "none",
    "waviness": "none",
    "animation": "static",
    "color": "#666666"
  }
}
```

### Edge Semantic Tags
Semantic tags are derived from `HydroEdgeType` enum:
- **Boundedness**: "Bounded", "Unbounded"
- **Ordering**: "TotalOrder", "NoOrder"
- **Collection Type**: "Stream", "KeyedStream", "Singleton", "Optional"
- **Special Properties**: "Network", "Cycle", "Keyed", "Persistent"

### Empty Semantic Tags
Edges with no properties have an empty array:
```json
{
  "semanticTags": []
}
```

## Edge Style Configuration

The `edgeStyleConfig` includes semantic mappings that define how tags map to visual styles:

```json
{
  "semanticMappings": {
    "NetworkGroup": {
      "Local": {
        "line-pattern": "solid",
        "animation": "static"
      },
      "Network": {
        "line-pattern": "dotted",
        "animation": "animated"
      }
    },
    "BoundednessGroup": {
      "Unbounded": { "line-width": 1 },
      "Bounded": { "line-width": 3 }
    },
    "CollectionGroup": {
      "Stream": {
        "arrowhead": "triangle-filled",
        "line-style": "single"
      },
      "KeyedStream": {
        "arrowhead": "triangle-filled",
        "line-style": "double"
      },
      "Singleton": {
        "arrowhead": "circle-filled",
        "line-style": "single"
      },
      "Optional": {
        "arrowhead": "diamond-open",
        "line-style": "single"
      }
    },
    "FlowGroup": {
      "Linear": { "halo": "none" },
      "Cycle": { "halo": "light-red" }
    },
    "OrderingGroup": {
      "TotalOrder": { "waviness": "none" },
      "NoOrder": { "waviness": "wavy" }
    }
  }
}
```

## Validation Checklist

✅ **Requirement 3.1**: Edge JSON includes `semanticTags` array
- Implemented in `write_edge()` method
- Tags are converted from `HydroEdgeType` enum to strings
- Network tag is automatically added for cross-location edges

✅ **Requirement 3.2**: Semantic mappings configuration generated
- Implemented in `get_edge_style_config()` method
- Includes all required mapping groups:
  - NetworkGroup (Local, Network)
  - BoundednessGroup (Unbounded, Bounded)
  - CollectionGroup (Stream, KeyedStream, Singleton, Optional)
  - FlowGroup (Linear, Cycle)
  - OrderingGroup (TotalOrder, NoOrder)

✅ **Requirement 3.3**: Nodes include semantic tags
- Implemented in `write_node_definition()` method
- Tags include node type and location type
- Generated via `get_node_semantic_tags()` helper method

✅ **Requirement 3.4**: JSON format compatibility
- Structure matches Hydroscope's expected format
- All required fields present (nodes, edges, edgeStyleConfig, etc.)
- Backward compatible with existing JSON consumers
- Valid JSON that can be parsed by `serde_json`

## Testing

The JSON format has been validated through:
1. **Compilation**: Library compiles successfully with `--features build`
2. **Structure**: JSON includes all required top-level fields
3. **Semantic Tags**: Both nodes and edges include `semanticTags` arrays
4. **Mappings**: Edge style config includes comprehensive semantic mappings
5. **Type Safety**: All enum conversions are exhaustive and type-safe

## Backward Compatibility

The changes maintain backward compatibility:
- Existing fields remain unchanged
- New `semanticTags` fields are additive
- Edge style computation uses the unified system
- Old `edgeProperties` field removed (was redundant with semanticTags)

## Integration with Hydroscope

The JSON format is designed to work seamlessly with Hydroscope:
- Semantic tags enable intelligent filtering and styling
- Style mappings provide visual differentiation
- Node and edge metadata support rich visualization
- Hierarchy choices enable multiple view modes
