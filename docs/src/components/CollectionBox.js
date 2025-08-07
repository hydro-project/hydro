import React from 'react';

const CollectionBox = ({
  x,
  y,
  headerHeight,
  width,
  height,
  title,
  children,
  headerColor = '#666',
  borderColor = '#666',
  backgroundColor = 'white'
}) => {
  return (
    <svg x={x} y={y} width={width} height={height} style={{
      transform: `translateX(-${width / 2}px) translateY(-${height / 2}px)`
    }}>
      {/* Main container */}
      <rect
        x={1}
        y={1}
        width={width - 2}
        height={height - 2}
        fill={backgroundColor}
        stroke={borderColor}
        strokeWidth="2"
        rx="8"
      />

      {/* Header */}
      <rect
        x={1}
        y={1}
        width={width - 2}
        height={headerHeight}
        fill={headerColor}
        rx="8"
      />
      <rect
        x={0}
        y={1 + headerHeight - 8}
        width={width}
        height="8"
        fill={headerColor}
      />

      {/* Title text */}
      <foreignObject
        x={0}
        y={0}
        width={width}
        height={headerHeight}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100%',
            width: '100%',
            fontSize: '10px',
            fontWeight: 'bold',
            fontFamily: 'monospace',
            color: 'white',
            textAlign: 'center'
          }}
        >
          {title}
        </div>
      </foreignObject>

      {/* Children content */}
      {children}
    </svg>
  );
};

export default CollectionBox;