import React, { useRef, useEffect } from 'react';
import { createScope, createTimeline } from 'animejs';
import styles from './svg-animation.module.css';

const KeyedStreamFoldAnimation = () => {
    const root = useRef(null);
    const scope = useRef(null);

    // SVG coordinate system
    const svgWidth = 650;
    const svgHeight = 310;

    // Precompute all animation data
    const allMessages = [
        { id: 'alice-0', key: 'alice', value: 10, color: '#4CAF50' },
        { id: 'bob-0', key: 'bob', value: 5, color: '#2196F3' },
        { id: 'alice-1', key: 'alice', value: 15, color: '#4CAF50' },
        { id: 'bob-1', key: 'bob', value: 8, color: '#2196F3' },
        { id: 'alice-2', key: 'alice', value: 3, color: '#4CAF50' }
    ];

    const elements = [];
    const timeline = [];
    const accumulators = { alice: 0, bob: 0 };
    const keyMessageCounts = { alice: 0, bob: 0 };

    allMessages.forEach((msg, index) => {
        const messageIndex = keyMessageCounts[msg.key];
        // Position first element (index 0) on the right, subsequent elements to the left
        const startX = 145 - messageIndex * 25;
        const startY = msg.key === 'alice' ? 130 : 220; // Position in respective key groups
        const outputX = 540; // Center of the key containers
        const outputY = msg.key === 'alice' ? 130 : 220; // Center of alice/bob containers

        // Create animating message element
        elements.push(<g key={`animating-${msg.id}`} id={`animating-${msg.id}`}>
            <rect
                x={startX - 12}
                y={startY - 8}
                width="24"
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
                ${msg.value}
            </text>
        </g>);

        // Update accumulator value
        accumulators[msg.key] += msg.value;

        // Animation timeline
        timeline.push({
            elementId: `animating-${msg.id}`, props: {
                translateX: outputX - startX,
                translateY: outputY - startY,
                duration: 1000,
                easing: 'easeInOutCubic'
            }, time: index === 0 ? "+=0" : "+=500"
        });

        // Hide animating message
        timeline.push({
            elementId: `animating-${msg.id}`, props: {
                opacity: 0,
                duration: 300,
                easing: 'easeInCubic'
            }, time: "<"
        });

        // Update output text content
        timeline.push({
            elementId: `output-${msg.key}`, props: {
                innerHTML: `$${accumulators[msg.key]}`,
                duration: 1
            }, time: "<<"
        });

        keyMessageCounts[msg.key]++;
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
            <svg id="keyed-stream-fold-svg" width={svgWidth} height={svgHeight} viewBox={`0 0 ${svgWidth} ${svgHeight}`} ref={root}>
                {/* Input KeyedStream Container */}
                <g>
                    <rect
                        x="20"
                        y="50"
                        width="160"
                        height="250"
                        fill="white"
                        stroke="#666"
                        strokeWidth="2"
                        rx="8"
                    />
                    {/* Header */}
                    <rect
                        x="20"
                        y="50"
                        width="160"
                        height="25"
                        fill="#666"
                        rx="8"
                    />
                    <rect
                        x="20"
                        y="67"
                        width="160"
                        height="8"
                        fill="#666"
                    />
                    <text
                        x="100"
                        y="67"
                        textAnchor="middle"
                        fill="white"
                        fontSize="10"
                        fontWeight="bold"
                        fontFamily="monospace"
                    >
                        KeyedStream&lt;K, V&gt;
                    </text>

                    {/* Alice Key Group */}
                    <rect
                        x="30"
                        y="90"
                        width="140"
                        height="70"
                        fill="white"
                        stroke="#4CAF50"
                        strokeWidth="2"
                        rx="8"
                    />
                    <rect
                        x="30"
                        y="90"
                        width="140"
                        height="18"
                        fill="#4CAF50"
                        rx="8"
                    />
                    <rect
                        x="30"
                        y="100"
                        width="140"
                        height="8"
                        fill="#4CAF50"
                    />
                    <text
                        x="100"
                        y="103"
                        textAnchor="middle"
                        fill="white"
                        fontSize="11"
                        fontWeight="bold"
                    >
                        Key "alice"
                    </text>

                    {/* Bob Key Group */}
                    <rect
                        x="30"
                        y="180"
                        width="140"
                        height="70"
                        fill="white"
                        stroke="#2196F3"
                        strokeWidth="2"
                        rx="8"
                    />
                    <rect
                        x="30"
                        y="180"
                        width="140"
                        height="18"
                        fill="#2196F3"
                        rx="8"
                    />
                    <rect
                        x="30"
                        y="190"
                        width="140"
                        height="8"
                        fill="#2196F3"
                    />
                    <text
                        x="100"
                        y="193"
                        textAnchor="middle"
                        fill="white"
                        fontSize="11"
                        fontWeight="bold"
                    >
                        Key "bob"
                    </text>
                </g>

                {/* Arrow from Input to Fold */}
                <path
                    d="M 190 175 L 225 175"
                    stroke="#666"
                    strokeWidth="2"
                    markerEnd="url(#arrowhead)"
                />

                {/* Fold Operation */}
                <g>
                    <rect
                        x="235"
                        y="140"
                        width="160"
                        height="70"
                        fill="#f5f5f5"
                        stroke="#666"
                        strokeWidth="2"
                        rx="8"
                    />
                    <foreignObject
                        x="245"
                        y="155"
                        width="140"
                        height="50"
                    >
                        <div
                            style={{
                                display: 'flex',
                                alignItems: 'center',
                                justifyContent: 'center',
                                height: '100%',
                                fontSize: '11px',
                                fontWeight: 'bold',
                                fontFamily: 'monospace',
                                textAlign: 'center',
                                wordWrap: 'break-word',
                                lineHeight: '1.2'
                            }}
                        >
                            .fold(|| 0, |acc, amount| *acc += amount)
                        </div>
                    </foreignObject>

                    {/* Arrow from Fold to Output */}
                    <path
                        d="M 405 175 L 440 175"
                        stroke="#666"
                        strokeWidth="2"
                        markerEnd="url(#arrowhead)"
                    />
                </g>

                {/* Output KeyedSingleton Container */}
                <g>
                    <rect
                        x="450"
                        y="50"
                        width="180"
                        height="250"
                        fill="white"
                        stroke="#666"
                        strokeWidth="2"
                        rx="8"
                    />
                    {/* Header */}
                    <rect
                        x="450"
                        y="50"
                        width="180"
                        height="25"
                        fill="#666"
                        rx="8"
                    />
                    <rect
                        x="450"
                        y="67"
                        width="180"
                        height="8"
                        fill="#666"
                    />
                    <text
                        x="540"
                        y="67"
                        textAnchor="middle"
                        fill="white"
                        fontSize="10"
                        fontWeight="bold"
                        fontFamily="monospace"
                    >
                        KeyedSingleton&lt;K, A&gt;
                    </text>

                    {/* Alice Key Container */}
                    <rect
                        x="460"
                        y="90"
                        width="160"
                        height="70"
                        fill="white"
                        stroke="#4CAF50"
                        strokeWidth="2"
                        rx="8"
                    />
                    <rect
                        x="460"
                        y="90"
                        width="160"
                        height="18"
                        fill="#4CAF50"
                        rx="8"
                    />
                    <rect
                        x="460"
                        y="100"
                        width="160"
                        height="8"
                        fill="#4CAF50"
                    />
                    <text
                        x="540"
                        y="103"
                        textAnchor="middle"
                        fill="white"
                        fontSize="11"
                        fontWeight="bold"
                    >
                        Key "alice"
                    </text>

                    {/* Bob Key Container */}
                    <rect
                        x="460"
                        y="180"
                        width="160"
                        height="70"
                        fill="white"
                        stroke="#2196F3"
                        strokeWidth="2"
                        rx="8"
                    />
                    <rect
                        x="460"
                        y="180"
                        width="160"
                        height="18"
                        fill="#2196F3"
                        rx="8"
                    />
                    <rect
                        x="460"
                        y="190"
                        width="160"
                        height="8"
                        fill="#2196F3"
                    />
                    <text
                        x="540"
                        y="193"
                        textAnchor="middle"
                        fill="white"
                        fontSize="11"
                        fontWeight="bold"
                    >
                        Key "bob"
                    </text>

                    {/* Static output text elements */}
                    <foreignObject
                        x="520"
                        y="120"
                        width="40"
                        height="20"
                    >
                        <div
                            style={{
                                display: 'flex',
                                alignItems: 'center',
                                justifyContent: 'center',
                                height: '100%',
                                fontSize: '10px',
                                fontWeight: 'bold',
                                fontFamily: 'monospace',
                                color: '#333'
                            }}
                        >
                            <span id="output-alice">$0</span>
                        </div>
                    </foreignObject>
                    <foreignObject
                        x="520"
                        y="210"
                        width="40"
                        height="20"
                    >
                        <div
                            style={{
                                display: 'flex',
                                alignItems: 'center',
                                justifyContent: 'center',
                                height: '100%',
                                fontSize: '10px',
                                fontWeight: 'bold',
                                fontFamily: 'monospace',
                                color: '#333'
                            }}
                        >
                            <span id="output-bob">$0</span>
                        </div>
                    </foreignObject>
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

export default KeyedStreamFoldAnimation;