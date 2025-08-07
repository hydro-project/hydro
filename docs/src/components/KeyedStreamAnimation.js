import React from 'react';
import AnimationDiagram from './AnimationDiagram';
import AnimatedMessage from './AnimatedMessage';
import OutputMessage from './OutputMessage';

const KeyedStreamAnimationRefactored = () => {
    const svgWidth = 480; // Match fold animation width
    const svgHeight = 250; // Match fold animation height

    const setupAnimation = (world) => {
        const center = world.getCenter();

        // Create collection boxes
        const inputContainer = world.createCollectionBox('input', 80, center.y, 160, 200, 25, "KeyedStream<K, V>");
        const outputContainer = world.createCollectionBox('output', 400, center.y, 160, 200, 25, "Stream<(K, V), NoOrder>");

        // Create operator box using world center
        const entriesOperator = world.createOperatorBox('entries', center.x, center.y, 100, 40, ".entries()");

        // Compute positions for key groups using the input container
        const keyGroupPositions = inputContainer.computeChildPositions(50, 3);

        // Create key groups
        const keyGroups = {
            'A': world.createKeyGroup('keyA', inputContainer.getContentCenter().x, keyGroupPositions[0], 140, 50, 'A', 'Key A', '#4CAF50'),
            'B': world.createKeyGroup('keyB', inputContainer.getContentCenter().x, keyGroupPositions[1], 140, 50, 'B', 'Key B', '#2196F3'),
            'C': world.createKeyGroup('keyC', inputContainer.getContentCenter().x, keyGroupPositions[2], 140, 50, 'C', 'Key C', '#FF9800')
        };

        const allMessages = [
            { id: 'A-0', key: 'A', value: 'A1', color: '#4CAF50' },
            { id: 'B-0', key: 'B', value: 'B1', color: '#2196F3' },
            { id: 'A-1', key: 'A', value: 'A2', color: '#4CAF50' },
            { id: 'C-0', key: 'C', value: 'C1', color: '#FF9800' },
            { id: 'B-1', key: 'B', value: 'B2', color: '#2196F3' },
            { id: 'A-2', key: 'A', value: 'A3', color: '#4CAF50' }
        ];

        const timeline = [];
        let outputIndex = 0;
        const keyMessageCounts = { A: 0, B: 0, C: 0 };

        // Compute positions for output messages using the output container
        const outputPositions = outputContainer.computeChildPositions(20, allMessages.length);

        allMessages.forEach((msg, index) => {
            const messageIndex = keyMessageCounts[msg.key];
            const keyGroup = keyGroups[msg.key];
            const contentCenter = keyGroup.getContentCenter();

            // Position first element (index 0) on the right, subsequent elements to the left
            const startX = contentCenter.x + 50 - messageIndex * 25; // Start from right side of content area
            const startY = contentCenter.y; // Use content center Y
            const outputY = outputPositions[outputIndex]; // Use computed position

            world.addElement(
                <AnimatedMessage
                    key={`animating-${msg.id}`}
                    id={`animating-${msg.id}`}
                    x={startX}
                    y={startY}
                    text={msg.value}
                    color={msg.color}
                />
            );

            world.addElement(
                <OutputMessage
                    key={`output-${msg.id}`}
                    id={`output-${msg.id}`}
                    x={540} // Match aligned output position
                    y={outputPositions[outputIndex]} // Use computed position
                    text={`(${msg.key}, ${msg.value})`}
                    color={msg.color}
                />
            );

            timeline.push({
                elementId: `animating-${msg.id}`, props: {
                    x: outputContainer.getContentCenter().x,
                    y: outputY,
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
            });

            timeline.push({
                elementId: `output-${msg.id}`, props: {
                    opacity: 1,
                    duration: 300,
                    easing: 'easeOutCubic'
                }, time: "<<"
            });

            keyMessageCounts[msg.key]++;
            outputIndex++;
        });

        // Create arrows using world
        world.createArrow('arrow1', inputContainer, entriesOperator);
        world.createArrow('arrow2', entriesOperator, outputContainer);

        return timeline;
    };

    return (
        <AnimationDiagram
            svgWidth={svgWidth}
            svgHeight={svgHeight}
            svgId="keyed-stream-svg"
            setupAnimation={setupAnimation}
        />
    );
};

export default KeyedStreamAnimationRefactored;