# Animation Components

This directory contains reusable components for creating stream visualization animations. These components were extracted from `KeyedStreamAnimation` and `KeyedStreamFoldAnimation` to reduce code duplication and ensure unified design.

## Components

### CollectionBox
A container for collection types (KeyedStream, Stream, KeyedSingleton, etc.)

**Props:**
- `x, y` - Position coordinates
- `width, height` - Dimensions
- `title` - Header text (e.g., "KeyedStream<K, V>")
- `children` - Content inside the box
- `headerColor` - Header background color (default: '#666')
- `borderColor` - Border color (default: '#666')
- `backgroundColor` - Background color (default: 'white')

### KeyGroup
A container for individual key groups within keyed collections

**Props:**
- `x, y` - Position coordinates
- `width, height` - Dimensions
- `keyName` - Key identifier text (e.g., "Key A")
- `color` - Theme color for the group
- `children` - Content inside the group
- `backgroundColor` - Background color (default: 'white')

### Arrow
Directional arrows between components

**Props:**
- `startX, startY` - Starting coordinates
- `endX, endY` - Ending coordinates
- `color` - Arrow color (default: '#666')
- `strokeWidth` - Line thickness (default: 2)
- `markerId` - Arrow marker ID (default: 'arrowhead')

### ArrowMarker
Arrow marker definition for SVG defs

**Props:**
- `id` - Marker ID (default: 'arrowhead')
- `color` - Arrow color (default: '#666')
- `size` - Arrow size (default: 6)

### OperatorBox
Boxes for operations like `.entries()`, `.fold()`, etc.

**Props:**
- `x, y` - Position coordinates
- `width, height` - Dimensions
- `text` - Operation text
- `backgroundColor` - Background color (default: '#f5f5f5')
- `borderColor` - Border color (default: '#666')
- `textColor` - Text color (default: '#333')
- `fontSize` - Text size (default: 12)
- `multiline` - Use foreignObject for complex text (default: false)

### AnimatedMessage
Moving message elements in animations

**Props:**
- `id` - Element ID for animation targeting
- `x, y` - Position coordinates
- `width, height` - Dimensions (default: 20x16)
- `text` - Message text
- `color` - Background color
- `textColor` - Text color (default: 'white')
- `fontSize` - Text size (default: 10)
- `rx` - Border radius (default: 8)
- `opacity` - Opacity (default: 1)
- `shadow` - Drop shadow effect (default: true)

### OutputMessage
Static output message elements

**Props:**
- `id` - Element ID for animation targeting
- `x, y` - Position coordinates
- `width, height` - Dimensions (default: 120x20)
- `text` - Message text
- `color` - Background color
- `textColor` - Text color (default: 'white')
- `fontSize` - Text size (default: 10)
- `rx` - Border radius (default: 10)
- `opacity` - Initial opacity (default: 0)

## Usage Example

```javascript
import React from 'react';
import { 
  CollectionBox, 
  KeyGroup, 
  Arrow, 
  ArrowMarker, 
  OperatorBox,
  AnimatedMessage 
} from './animation-components';

const MyAnimation = () => {
  return (
    <svg width={600} height={300}>
      <CollectionBox
        x={50}
        y={50}
        width={200}
        height={200}
        title="KeyedStream<K, V>"
      >
        <KeyGroup
          x={60}
          y={80}
          width={180}
          height={60}
          keyName="Key A"
          color="#4CAF50"
        />
      </CollectionBox>
      
      <Arrow startX={260} startY={150} endX={300} endY={150} />
      
      <OperatorBox
        x={310}
        y={130}
        width={80}
        height={40}
        text=".map()"
      />
      
      <AnimatedMessage
        id="msg-1"
        x={120}
        y={110}
        text="A1"
        color="#4CAF50"
      />
      
      <defs>
        <ArrowMarker />
      </defs>
    </svg>
  );
};
```

## Benefits

1. **Consistency** - Unified styling and behavior across all animations
2. **Reusability** - Easy to create new animations with existing components
3. **Maintainability** - Changes to styling can be made in one place
4. **Flexibility** - Components accept positioning and styling props
5. **Composability** - Components can be nested and combined as needed

## Creating New Animations

To create a new animation:

1. Import the needed components
2. Define your layout coordinates and colors
3. Use CollectionBox for main containers
4. Use KeyGroup for keyed collections
5. Use Arrow to connect components
6. Use OperatorBox for operations
7. Use AnimatedMessage/OutputMessage for moving elements
8. Include ArrowMarker in your SVG defs
9. Set up your animation timeline using the component IDs