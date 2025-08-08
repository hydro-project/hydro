import React from 'react';

const KeyGroup = ({
  x,
  y,
  width,
  height,
  keyName,
  color,
  backgroundColor = 'white'
}) => {
  return (
    <g style={{
      transform: `translateX(${x}px) translateY(${y}px)`
    }}>
      <g style={{
        transform: `translateX(-${width / 2}px) translateY(-${height / 2}px)`
      }}>
        {/* Group box */}
        <rect
          x={0}
          y={0}
          width={width}
          height={height}
          fill={backgroundColor}
          stroke={color}
          strokeWidth="2"
          rx="8"
        />

        {/* Group header */}
        <rect
          x={0}
          y={0}
          width={width}
          height="18"
          fill={color}
          rx="8"
        />
        <rect
          x={0}
          y={10}
          width={width}
          height="8"
          fill={color}
        />

        {/* Key name text */}
        <foreignObject
          x={0}
          y={0}
          width={width}
          height="18"
        >
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              height: '100%',
              width: '100%',
              fontSize: '11px',
              fontWeight: 'bold',
              color: 'white',
              textAlign: 'center'
            }}
          >
            {keyName}
          </div>
        </foreignObject>
      </g>
    </g>
  );
};

// Object-oriented KeyGroup class
class KeyGroupClass {
  constructor(x, y, width, height, key, keyName, color, backgroundColor = 'white') {
    this.x = x; // center coordinate
    this.y = y; // center coordinate
    this.width = width;
    this.height = height;
    this.key = key;
    this.keyName = keyName;
    this.color = color;
    this.backgroundColor = backgroundColor;
    this.headerHeight = 18; // Fixed header height from KeyGroup component
  }

  // Get the bounding box of the content area (excluding header)
  getContentBounds() {
    const left = this.x - this.width / 2;
    const right = this.x + this.width / 2;
    const top = this.y - this.height / 2 + this.headerHeight;
    const bottom = this.y + this.height / 2;

    return {
      left,
      right,
      top,
      bottom,
      width: this.width,
      height: this.height - this.headerHeight,
      centerX: this.x,
      centerY: top + (bottom - top) / 2
    };
  }

  // Get the center coordinate of the content area
  getContentCenter() {
    const bounds = this.getContentBounds();
    return { x: bounds.centerX, y: bounds.centerY };
  }

  // Return the React element
  toReactElement() {
    return (
      <KeyGroup
        key={this.key}
        x={this.x}
        y={this.y}
        width={this.width}
        height={this.height}
        keyName={this.keyName}
        color={this.color}
        backgroundColor={this.backgroundColor}
      />
    );
  }
}

export default KeyGroup;
export { KeyGroupClass };
