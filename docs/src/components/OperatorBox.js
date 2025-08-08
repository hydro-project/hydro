import React from 'react';

const OperatorBox = ({
  x,
  y,
  width,
  height,
  text,
  backgroundColor = '#f5f5f5',
  borderColor = '#555',
  textColor = '#333',
  fontSize = 12
}) => {
  return (
    <g style={{
      transform: `translateX(${x}px) translateY(${y}px)`
    }}>
      <g style={{
        transform: `translateX(-${width / 2}px) translateY(-${height / 2}px)`
      }}>
        <rect
          x={0}
          y={0}
          width={width}
          height={height}
          fill={backgroundColor}
          stroke={borderColor}
          strokeWidth="2"
          rx="8"
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
              padding: '10px',
              boxSizing: 'border-box',
              fontSize: `${fontSize}px`,
              fontWeight: 'bold',
              fontFamily: 'monospace',
              textAlign: 'center',
              wordWrap: 'break-word',
              lineHeight: '1.2',
              color: textColor
            }}
          >
            {text}
          </div>
        </foreignObject>
      </g>
    </g>
  );
};

export default OperatorBox;
