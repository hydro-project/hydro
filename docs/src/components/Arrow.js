import React from 'react';

const Arrow = ({ 
  startX, 
  startY, 
  endX, 
  endY, 
  color = '#666',
  strokeWidth = 2,
  markerId = 'arrowhead'
}) => {
  return (
    <path
      d={`M ${startX} ${startY} L ${endX} ${endY}`}
      stroke={color}
      strokeWidth={strokeWidth}
      markerEnd={`url(#${markerId})`}
    />
  );
};

// Arrow marker definition component
export const ArrowMarker = ({ 
  id = 'arrowhead', 
  color = '#666',
  size = 6 
}) => {
  return (
    <marker
      id={id}
      markerWidth={size}
      markerHeight={size}
      refX={size - 1}
      refY={size / 2}
      orient="auto"
    >
      <polygon 
        points={`0 0, ${size} ${size / 2}, 0 ${size}`} 
        fill={color} 
      />
    </marker>
  );
};

export default Arrow;