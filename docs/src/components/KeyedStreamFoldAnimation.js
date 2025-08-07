import React from 'react';
import AnimationDiagram from './AnimationDiagram';
import AnimatedMessage from './AnimatedMessage';

const KeyedStreamFoldAnimationRefactored = () => {
    // SVG coordinate system - adjusted for center coordinates
    const svgWidth = 480; // 450 + 180 = 630 (right edge of output container)
    const svgHeight = 250; // 50 + 200 = 250 (bottom edge of containers)

    const setupAnimation = (world) => {

        // Precompute all animation data
        const allMessages = [
            { id: 'alice-0', key: 'alice', value: 10, color: '#4CAF50' },
            { id: 'bob-0', key: 'bob', value: 5, color: '#2196F3' },
            { id: 'alice-1', key: 'alice', value: 15, color: '#4CAF50' },
            { id: 'bob-1', key: 'bob', value: 8, color: '#2196F3' },
            { id: 'alice-2', key: 'alice', value: 3, color: '#4CAF50' }
        ];

        const center = world.getCenter();

        const inputContainer = world.createCollectionBox('input', 80, center.y, 160, 200, 25, "KeyedStream<K, V>");
        const outputContainer = world.createCollectionBox('output', 400, center.y, 160, 200, 25, "KeyedSingleton<K, A>");

        const foldOperator = world.createOperatorBox('fold', center.x, center.y, 120, 70, ".fold(|| 0, |acc, amount| *acc += amount)");

        // Compute positions for key groups using the containers
        const inputKeyGroupPositions = inputContainer.computeChildPositions(50, 2);
        const outputKeyGroupPositions = outputContainer.computeChildPositions(50, 2);

        // Create KeyGroup instances
        const inputKeyGroups = {
            "alice": world.createKeyGroup('inputAlice', inputContainer.getContentCenter().x, inputKeyGroupPositions[0], 140, 50, 'alice', 'Key "alice"', '#4CAF50'),
            "bob": world.createKeyGroup('inputBob', inputContainer.getContentCenter().x, inputKeyGroupPositions[1], 140, 50, 'bob', 'Key "bob"', '#2196F3')
        };

        const outputKeyGroups = {
            "alice": world.createKeyGroup('outputAlice', outputContainer.getContentCenter().x, outputKeyGroupPositions[0], 140, 50, 'alice', 'Key "alice"', '#4CAF50'),
            "bob": world.createKeyGroup('outputBob', outputContainer.getContentCenter().x, outputKeyGroupPositions[1], 140, 50, 'bob', 'Key "bob"', '#2196F3')
        };

        const timeline = [];
        const accumulators = { alice: 0, bob: 0 };
        const keyMessageCounts = { alice: 0, bob: 0 };

        allMessages.forEach((msg, index) => {
            const messageIndex = keyMessageCounts[msg.key];
            const inputKeyGroup = inputKeyGroups[msg.key];
            const outputKeyGroup = outputKeyGroups[msg.key];
            const inputContentCenter = inputKeyGroup.getContentCenter();
            const outputContentCenter = outputKeyGroup.getContentCenter();

            // Position first element (index 0) on the right, subsequent elements to the left
            const startX = inputContentCenter.x + 25 - messageIndex * 25; // Start from right side of input content area
            const startY = inputContentCenter.y; // Use input content center Y
            const outputX = outputContentCenter.x; // Use output content center X
            const outputY = outputContentCenter.y; // Use output content center Y

            // Create animating message element
            world.addElement(
                <AnimatedMessage
                    key={`animating-${msg.id}`}
                    id={`animating-${msg.id}`}
                    x={startX}
                    y={startY}
                    width={24}
                    text={`$${msg.value}`}
                    color={msg.color}
                />
            );

            // Update accumulator value
            accumulators[msg.key] += msg.value;

            // Animation timeline
            timeline.push({
                elementId: `animating-${msg.id}`, props: {
                    x: outputX,
                    y: outputY,
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

        // Create arrows using world
        world.createArrow('arrow1', inputContainer, foldOperator);
        world.createArrow('arrow2', foldOperator, outputContainer);

        // Add output value elements to world using the new addText method
        const aliceCenter = outputKeyGroups['alice'].getContentCenter();
        const bobCenter = outputKeyGroups['bob'].getContentCenter();

        world.addText('output-alice', aliceCenter.x, aliceCenter.y, '$0');
        world.addText('output-bob', bobCenter.x, bobCenter.y, '$0');

        return timeline;
    };

    return (
        <AnimationDiagram
            svgWidth={svgWidth}
            svgHeight={svgHeight}
            svgId="keyed-stream-fold-svg"
            setupAnimation={setupAnimation}
        />
    );
};

export default KeyedStreamFoldAnimationRefactored;