import React from 'react';

const Arrow = ({ 
  startX, 
  startY, 
  endX, 
  endY, 
  strokeWidth = 2,
  markerId = 'arrowhead'
}) => {
  return (
    <path
      className={"arrow-line"}
      d={`M ${startX} ${startY} L ${endX - 10} ${endY}`}
      stroke={"inherit"}
      strokeWidth={strokeWidth}
      markerEnd={`url(#${markerId})`}
    />
  );
};

// Arrow marker definition component
export const ArrowMarker = ({ 
  id = 'arrowhead', 
  size = 6 
}) => {
  return (
    <marker
      id={id}
      markerWidth={size}
      markerHeight={size}
      refX={size - 5}
      refY={size / 2}
      orient="auto"
    >
      <polygon 
        className="arrowhead"
        points={`0 0, ${size} ${size / 2}, 0 ${size}`} 
      />
    </marker>
  );
};

export default Arrow;
