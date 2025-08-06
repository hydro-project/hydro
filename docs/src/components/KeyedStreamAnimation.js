import React, { useRef, useEffect } from 'react';
import { createScope, createTimeline } from 'animejs';
import styles from './svg-animation.module.css';

const KeyedStreamAnimation = () => {
    const root = useRef(null);
    const scope = useRef(null);

    const keyGroups = [
        { key: 'A', color: '#4CAF50', y: 110 },
        { key: 'B', color: '#2196F3', y: 180 },
        { key: 'C', color: '#FF9800', y: 250 }
    ];

    // SVG coordinate system
    const svgWidth = 600;
    const svgHeight = 310;
    const outputX = 490;

    // Precompute all animation data including initial state
    const allMessages = [
        { id: 'A-0', key: 'A', value: 'A1', color: '#4CAF50', groupY: 110 },
        { id: 'B-0', key: 'B', value: 'B1', color: '#2196F3', groupY: 180 },
        { id: 'A-1', key: 'A', value: 'A2', color: '#4CAF50', groupY: 110 },
        { id: 'C-0', key: 'C', value: 'C1', color: '#FF9800', groupY: 250 },
        { id: 'B-1', key: 'B', value: 'B2', color: '#2196F3', groupY: 180 },
        { id: 'A-2', key: 'A', value: 'A3', color: '#4CAF50', groupY: 110 }
    ];

    const elements = [];
    const timeline = [];
    let outputIndex = 0;
    const keyMessageCounts = { A: 0, B: 0, C: 0 };

    allMessages.forEach((msg, index) => {
        const messageIndex = keyMessageCounts[msg.key];
        // Position first element (index 0) on the right, subsequent elements to the left
        const startX = 165 - messageIndex * 25;
        const startY = msg.groupY + 10;
        const outputY = 100 + outputIndex * 30;

        elements.push(<g key={`animating-${msg.id}`} id={`animating-${msg.id}`}>
            <rect
                x={startX - 10}
                y={startY - 8}
                width="20"
                height="16"
                fill={msg.color}
                rx="8"
                style={{
                    filter: 'drop-shadow(0 2px 4px rgba(0,0,0,0.2))',
                    stroke: 'rgba(255,255,255,0.3)',
                    strokeWidth: 1
                }}
            />
            <text
                x={startX}
                y={startY + 2}
                textAnchor="middle"
                fill="white"
                fontSize="10"
                fontWeight="bold"
            >
                {msg.value}
            </text>
        </g>);

        elements.push(<g key={`output-${msg.id}`} id={`output-${msg.id}`} opacity={0}>
            <rect
                x="430"
                y={90 + outputIndex * 30}
                width="120"
                height="20"
                fill={msg.color}
                rx="10"
            />
            <text
                x="490"
                y={103 + outputIndex * 30}
                textAnchor="middle"
                fill="white"
                fontSize="10"
                fontWeight="bold"
                fontFamily="monospace"
            >
                ({msg.key}, {msg.value})
            </text>
        </g>);

        timeline.push({
            elementId: `animating-${msg.id}`, props: {
                translateX: outputX - startX,
                translateY: outputY - startY,
                duration: 1000,
                easing: 'easeInOutCubic'
            }, time: index === 0 ? "+=0" : "+=500"
        });

        // Hide animating message and show output
        timeline.push({
            elementId: `animating-${msg.id}`, props: {
                opacity: 0,
                duration: 300,
                easing: 'easeInCubic'
            }, time: "<"
        }); // end of previous

        timeline.push({
            elementId: `output-${msg.id}`, props: {
                opacity: 1,
                duration: 300,
                easing: 'easeOutCubic'
            }, time: "<<"
        }); // start of previous

        keyMessageCounts[msg.key]++;
        outputIndex++;
    });

    useEffect(() => {
        scope.current = createScope({ root }).add(self => {
            const animeTimeline = createTimeline({
                autoplay: false
            });

            timeline.forEach((event) => {
                animeTimeline.add(`#${event.elementId}`, event.props, event.time);
            });

            self.add('start', () => {
                animeTimeline.restart();
            });
        });

        return () => scope.current.revert();
    }, []);

    return (
        <div className={styles.container}>
            <svg id="keyed-stream-svg" width={svgWidth} height={svgHeight} viewBox={`0 0 ${svgWidth} ${svgHeight}`} ref={root}>
                {/* Keyed Stream Groups */}
                <g>
                    {/* Keyed Stream Container */}
                    <rect
                        x="40"
                        y="50"
                        width="160"
                        height="250"
                        fill="white"
                        stroke="#666"
                        strokeWidth="2"
                        rx="8"
                    />
                    {/* Keyed Stream Header */}
                    <rect
                        x="40"
                        y="50"
                        width="160"
                        height="25"
                        fill="#666"
                        rx="8"
                    />
                    <rect
                        x="40"
                        y="67"
                        width="160"
                        height="8"
                        fill="#666"
                    />
                    <text
                        x="120"
                        y="67"
                        textAnchor="middle"
                        fill="white"
                        fontSize="10"
                        fontWeight="bold"
                        fontFamily="monospace"
                    >
                        KeyedStream&lt;K, V&gt;
                    </text>
                    {keyGroups.map(group => (
                        <g key={group.key}>
                            {/* Group box */}
                            <rect
                                x="50"
                                y={group.y - 25}
                                width="140"
                                height="50"
                                fill="white"
                                stroke={group.color}
                                strokeWidth="2"
                                rx="8"
                            />
                            {/* Group header */}
                            <rect
                                x="50"
                                y={group.y - 25}
                                width="140"
                                height="18"
                                fill={group.color}
                                rx="8"
                            />
                            <rect
                                x="50"
                                y={group.y - 15}
                                width="140"
                                height="8"
                                fill={group.color}
                            />
                            <text
                                x="120"
                                y={group.y - 12}
                                textAnchor="middle"
                                fill="white"
                                fontSize="11"
                                fontWeight="bold"
                            >
                                Key {group.key}
                            </text>
                        </g>
                    ))}
                </g>

                {/* Arrow from Keyed Stream to Entries */}
                <path
                    d="M 210 175 L 245 175"
                    stroke="#666"
                    strokeWidth="2"
                    markerEnd="url(#arrowhead)"
                />

                {/* Entries Operator */}
                <g>
                    <rect
                        x="255"
                        y="155"
                        width="100"
                        height="40"
                        fill="#f5f5f5"
                        stroke="#666"
                        strokeWidth="2"
                        rx="8"
                    />
                    <text
                        x="305"
                        y="180"
                        textAnchor="middle"
                        fontSize="12"
                        fontWeight="bold"
                        fontFamily="monospace"
                    >
                        .entries()
                    </text>

                    {/* Arrow from Entries to Output */}
                    <path
                        d="M 365 175 L 410 175"
                        stroke="#666"
                        strokeWidth="2"
                        markerEnd="url(#arrowhead)"
                    />
                </g>

                {/* Output Stream */}
                <g>
                    <rect
                        x="420"
                        y="50"
                        width="140"
                        height="250"
                        fill="white"
                        stroke="#666"
                        strokeWidth="2"
                        rx="8"
                    />
                    {/* Header */}
                    <rect
                        x="420"
                        y="50"
                        width="140"
                        height="25"
                        fill="#666"
                        rx="8"
                    />
                    <rect
                        x="420"
                        y="67"
                        width="140"
                        height="8"
                        fill="#666"
                    />
                    <text
                        x="490"
                        y="67"
                        textAnchor="middle"
                        fill="white"
                        fontSize="10"
                        fontWeight="bold"
                        fontFamily="monospace"
                    >
                        Stream&lt;(K, V), NoOrder&gt;
                    </text>
                </g>

                {elements}

                {/* Arrow marker definition */}
                <defs>
                    <marker
                        id="arrowhead"
                        markerWidth="6"
                        markerHeight="6"
                        refX="5"
                        refY="3"
                        orient="auto"
                    >
                        <polygon points="0 0, 6 3, 0 6" fill="#666" />
                    </marker>
                </defs>
            </svg>

            <div className={styles.controls}>
                <button onClick={() => {
                    scope.current.methods.start()
                }} className={styles.button}>
                    {"Play"}
                </button>
            </div>
        </div>
    );
};

export default KeyedStreamAnimation;