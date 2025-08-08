import React from 'react';

const OutputMessage = ({
  id,
  x,
  y,
  width = 120,
  height = 20,
  text,
  color,
  textColor = 'white',
  fontSize = 10,
  rx = 10,
  opacity = 0
}) => {
  return (
    <g id={id} opacity={opacity} transform={`translate(${x - width / 2}, ${y - height / 2})`}>
      <rect
        x={0}
        y={0}
        width={width}
        height={height}
        fill={color}
        rx={rx}
      />
      <foreignObject
        x={0}
        y={0}
        width={width}
        height={height}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100%',
            width: '100%',
            fontSize: `${fontSize}px`,
            fontWeight: 'bold',
            fontFamily: 'monospace',
            color: textColor,
            textAlign: 'center'
          }}
        >
          {text}
        </div>
      </foreignObject>
    </g>
  );
};

export default OutputMessage;
