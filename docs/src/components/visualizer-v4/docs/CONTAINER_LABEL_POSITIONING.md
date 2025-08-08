# Container Label Positioning & Dimension Adjustment Implementation

## Overview

Fixed the issue where container labels were being occluded by moving them to the bottom-right corner of containers and automatically adjusting container dimensions to accommodate the label space.

## Key Components Modified

### 1. Constants (core/constants.ts)
Added new layout constants for consistent label positioning:

```typescript
// Container label positioning and sizing
CONTAINER_LABEL_HEIGHT: 24,           // Height reserved for container labels
CONTAINER_LABEL_PADDING: 8,           // Padding around container labels  
CONTAINER_LABEL_FONT_SIZE: 12,        // Font size for container labels
```

### 2. VisualizationState (core/VisState.ts)
**New Method:** `getContainerAdjustedDimensions(id: string)`
- Calculates container dimensions that include space for bottom-right labels
- For expanded containers: adds `CONTAINER_LABEL_HEIGHT + CONTAINER_LABEL_PADDING` to height
- For collapsed containers: ensures minimum height for label visibility
- Enforces minimum width/height constraints
- Well-documented with examples

**Modified:** `visibleContainers` getter
- Now uses adjusted dimensions by default
- ELK layout dimensions take precedence when available
- Ensures all container consumers get label-aware dimensions

### 3. Container Node Component (render/nodes.tsx)
**New Label Positioning:**
- Labels positioned absolutely at `bottom: 8px; right: 8px`
- White background with colored border for visibility
- Text truncation with ellipsis for long labels
- Maximum width constrained to container width minus padding
- Collapsed indicator moved to top-left when needed

**Styling Improvements:**
- Removed centered text alignment
- Added proper positioning for label elements
- Consistent visual hierarchy between collapsed/expanded states

### 4. ReactFlow Bridge (bridges/ReactFlowBridge.ts)
**Updated Dimension Handling:**
- Uses `visState.getContainerAdjustedDimensions()` instead of raw dimensions
- Ensures ReactFlow containers have correct sizes for label display
- Improved logging shows adjusted dimensions

### 5. ELK Bridge (bridges/ELKBridge.ts)
**Collapsed Container Handling:**
- Uses adjusted dimensions for collapsed containers treated as nodes
- Ensures ELK layout calculations account for label space

## Usage

### For Container Creation
```typescript
// Dimensions are automatically adjusted when retrieved
const state = createVisualizationState()
  .setContainer('container1', {
    expandedDimensions: { width: 300, height: 200 },
    label: 'My Container'
  });

// Get label-adjusted dimensions
const dims = state.getContainerAdjustedDimensions('container1');
// // console.log(((dims))); // { width: 300, height: 232 } (200 + 24 + 8)
```

### For Layout Engines
```typescript
// All layout engines automatically get adjusted dimensions
const containers = visState.visibleContainers; // Already includes label space
```

## Constants Documentation

| Constant | Value | Purpose |
|----------|--------|---------|
| `CONTAINER_LABEL_HEIGHT` | 24px | Height reserved for container labels |
| `CONTAINER_LABEL_PADDING` | 8px | Padding around container labels |
| `CONTAINER_LABEL_FONT_SIZE` | 12px | Font size for container labels |
| `MIN_CONTAINER_WIDTH` | 200px | Minimum container width |
| `MIN_CONTAINER_HEIGHT` | 150px | Minimum container height (before label adjustment) |

## Benefits

### ✅ No Label Occlusion
- Labels positioned in dedicated bottom-right space
- Never overlap with container content or child elements
- Consistent positioning across all container types

### ✅ Layout Integration  
- ELK layout engine accounts for label space in calculations
- Automatic dimension adjustments in VisState
- No manual dimension management required

### ✅ Responsive Design
- Labels truncate gracefully in narrow containers
- Maximum width prevents overflow beyond container bounds
- Minimum dimensions ensure label visibility

### ✅ Well-Exposed Architecture
- All adjustments handled in central VisState class
- Constants are clearly defined and easily adjustable
- Comprehensive documentation and examples provided

### ✅ State Management
- Single source of truth for dimension calculations
- Automatic propagation to all layout engines and bridges
- Clean separation between base dimensions and display dimensions

## Testing

### Visual Testing
- Created `test-container-labels.html` for visual verification
- Shows expanded and collapsed container examples
- Demonstrates dimension calculations and label positioning

### Unit Testing
- Test file created: `containerLabelPositioning.test.ts`
- Covers dimension adjustment logic
- Validates constant relationships
- Tests edge cases and error conditions

## Migration Notes

### Existing Code Compatibility
- All existing container creation code continues to work unchanged
- Dimension adjustments are automatic and transparent
- Layout engines automatically use adjusted dimensions

### Configuration
- Label dimensions can be adjusted via `LAYOUT_CONSTANTS`
- Changes propagate automatically throughout the system
- No code changes required to modify label sizing

## Implementation Quality

### Architecture
- ✅ Single responsibility: VisState handles all dimension logic
- ✅ Clean interfaces: Well-documented public methods
- ✅ Consistent: All consumers use the same adjusted dimensions
- ✅ Extensible: Easy to modify constants or add new label features

### Performance
- ✅ Efficient: Calculations only done when dimensions are requested  
- ✅ Cached: VisState provides efficient access to computed dimensions
- ✅ Minimal overhead: Simple arithmetic operations

### Maintainability
- ✅ Clear constants with descriptive names
- ✅ Comprehensive documentation and examples
- ✅ Centralized logic in one place
- ✅ Easy to test and modify
