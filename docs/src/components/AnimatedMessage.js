import React from 'react';

const AnimatedMessage = ({
  id,
  x,
  y,
  width = 20,
  height = 16,
  text,
  color,
  textColor = 'white',
  fontSize = 10,
  rx = 8,
  opacity = 1,
  shadow = true
}) => {
  const style = shadow ? {
    filter: 'drop-shadow(0 2px 4px rgba(0,0,0,0.2))',
    stroke: 'rgba(255,255,255,0.3)',
    strokeWidth: 1
  } : {};

  return (
    <svg id={id} opacity={opacity} x={x} y={y} style={{
      transform: `translateX(-${width/2}px) translateY(-${height/2}px)`,
      overflow: "visible"
    }}>
      <rect
        x={0}
        y={0}
        width={width}
        height={height}
        fill={color}
        rx={rx}
        style={style}
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
            color: textColor,
            textAlign: 'center'
          }}
        >
          {text}
        </div>
      </foreignObject>
    </svg>
  );
};

export default AnimatedMessage;