import React from 'react';

const Arrow = ({ 
  id,
  startX, 
  startY, 
  endX, 
  endY, 
  strokeWidth = 2,
  markerId = 'arrowhead',
  dashed = false,
  strokeDasharray = '4,3',
  opacity = 1
}) => {
  // Calculate offset along the arrow direction to leave room for arrowhead
  const dx = endX - startX;
  const dy = endY - startY;
  const length = Math.sqrt(dx * dx + dy * dy);
  const offset = length > 0 ? 10 / length : 0;
  const adjustedEndX = endX - dx * offset;
  const adjustedEndY = endY - dy * offset;

  return (
    <path
      id={id}
      className={"arrow-line"}
      d={`M ${startX} ${startY} L ${adjustedEndX} ${adjustedEndY}`}
      stroke={"inherit"}
      strokeWidth={strokeWidth}
      strokeDasharray={dashed ? strokeDasharray : undefined}
      markerEnd={`url(#${markerId})`}
      opacity={opacity}
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
