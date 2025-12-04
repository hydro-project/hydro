const SlicedRegion = ({
  id,
  x,
  y,
  width,
  height,
  label = "sliced!",
  strokeColor = '#888',
  strokeWidth = 2,
  strokeDasharray = '6,4',
  labelColor = '#666',
  labelFontSize = 10,
  rx = 8,
  opacity = 1,
  labelPosition = 'left' // 'left' or 'right'
}) => {
  const labelX = labelPosition === 'right' 
    ? x + width / 2 - 8 
    : x - width / 2 + 8;
  const textAnchor = labelPosition === 'right' ? 'end' : 'start';

  return (
    <g opacity={opacity}>
      {/* Dotted rectangle */}
      <rect
        id={id}
        x={x - width / 2}
        y={y - height / 2}
        width={width}
        height={height}
        fill="none"
        stroke={strokeColor}
        strokeWidth={strokeWidth}
        strokeDasharray={strokeDasharray}
        rx={rx}
      />
      
      {/* Label at top corner */}
      <text
        x={labelX}
        y={y - height / 2 - 5}
        fontSize={labelFontSize}
        fontFamily="monospace"
        fontWeight="bold"
        fill={labelColor}
        textAnchor={textAnchor}
      >
        {label}
      </text>
    </g>
  );
};

export default SlicedRegion;
