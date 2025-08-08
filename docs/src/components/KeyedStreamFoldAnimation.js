import React from 'react';
import AnimationDiagram from './AnimationDiagram';
import AnimatedMessage from './AnimatedMessage';
import { Point } from './animation-utils';

const KeyedStreamFoldAnimation = () => {
    const setupAnimation = (world) => {
        const allMessages = [
            { id: 'alice-0', key: 'alice', value: 10, color: '#4CAF50' },
            { id: 'bob-0', key: 'bob', value: 5, color: '#2196F3' },
            { id: 'alice-1', key: 'alice', value: 15, color: '#4CAF50' },
            { id: 'bob-1', key: 'bob', value: 8, color: '#2196F3' },
            { id: 'alice-2', key: 'alice', value: 3, color: '#4CAF50' }
        ];

        const center = world.getCenter();

        const inputContainer = world.createCollectionBox('input', new Point(80, center.y), 160, 200, 25, "KeyedStream<K, V>");
        const foldOperator = world.createOperatorBox('fold', new Point(center.x, center.y), 120, 70, ".fold(|| 0, |acc, amount| *acc += amount)");
        const outputContainer = world.createCollectionBox('output', new Point(400, center.y), 160, 200, 25, "KeyedSingleton<K, A>");

        const inputKeyGroupPositions = inputContainer.verticalChildPositions(50, 2);
        const inputKeyGroups = {
            "alice": world.createKeyGroup('inputAlice', new Point(inputContainer.getContentCenter().x, inputKeyGroupPositions[0]), 140, 50, 'alice', 'Key "alice"', '#4CAF50'),
            "bob": world.createKeyGroup('inputBob', new Point(inputContainer.getContentCenter().x, inputKeyGroupPositions[1]), 140, 50, 'bob', 'Key "bob"', '#2196F3')
        };

        const outputKeyGroupPositions = outputContainer.verticalChildPositions(50, 2);
        const outputKeyGroups = {
            "alice": world.createKeyGroup('outputAlice', new Point(outputContainer.getContentCenter().x, outputKeyGroupPositions[0]), 140, 50, 'alice', 'Key "alice"', '#4CAF50'),
            "bob": world.createKeyGroup('outputBob', new Point(outputContainer.getContentCenter().x, outputKeyGroupPositions[1]), 140, 50, 'bob', 'Key "bob"', '#2196F3')
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

            accumulators[msg.key] += msg.value;

            timeline.push({
                elementId: `animating-${msg.id}`, props: {
                    ...outputContentCenter,
                    duration: 1000,
                    easing: 'easeInOutCubic'
                }, time: index === 0 ? "+=0" : "+=500"
            });

            timeline.push({
                elementId: `animating-${msg.id}`, props: {
                    opacity: 0,
                    duration: 300,
                    easing: 'easeInCubic'
                }, time: "<"
            });

            timeline.push({
                elementId: `output-${msg.key}`, props: {
                    innerHTML: `$${accumulators[msg.key]}`,
                    duration: 1
                }, time: "<<"
            });

            keyMessageCounts[msg.key]++;
        });

        world.createArrow('arrow1', inputContainer, foldOperator);
        world.createArrow('arrow2', foldOperator, outputContainer);

        world.addText('output-alice', outputKeyGroups['alice'].getContentCenter(), '$0');
        world.addText('output-bob', outputKeyGroups['bob'].getContentCenter(), '$0');

        return timeline;
    };

    return (
        <AnimationDiagram
            svgWidth={480}
            svgHeight={250}
            setupAnimation={setupAnimation}
        />
    );
};

export default KeyedStreamFoldAnimation;