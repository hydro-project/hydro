import React from 'react';
import AnimationDiagram from '@site/src/components/AnimationDiagram';
import AnimatedMessage from '@site/src/components/AnimatedMessage';
import OutputMessage from '@site/src/components/OutputMessage';
import { Point } from '@site/src/components/animation-utils';
import HydroColors from '@site/src/components/hydro-colors';

const KeyedStreamAnimation = () => {
    const setupAnimation = (world) => {
        const center = world.getCenter();

        const inputContainer = world.createCollectionBox('input', new Point(80, center.y), 160, 200, 25, "KeyedStream<K, V>");
        const entriesOperator = world.createOperatorBox('entries', new Point(center.x, center.y), 100, 40, ".entries()");
        const outputContainer = world.createCollectionBox('output', new Point(400, center.y), 160, 200, 25, "Stream<(K, V), NoOrder>");

        const keyGroupPositions = inputContainer.verticalChildPositions(50, 3);
        const keyGroups = {
            'A': world.createKeyGroup('keyA', keyGroupPositions[0], 140, 50, 'Key A', HydroColors.getKeyColor(0)),
            'B': world.createKeyGroup('keyB', keyGroupPositions[1], 140, 50, 'Key B', HydroColors.getKeyColor(1)),
            'C': world.createKeyGroup('keyC', keyGroupPositions[2], 140, 50, 'Key C', HydroColors.getKeyColor(2))
        };

        const allMessages = [
            { id: 'A-0', key: 'A', value: 'A1', color: HydroColors.getKeyColor(0) },
            { id: 'B-0', key: 'B', value: 'B1', color: HydroColors.getKeyColor(1) },
            { id: 'A-1', key: 'A', value: 'A2', color: HydroColors.getKeyColor(0) },
            { id: 'C-0', key: 'C', value: 'C1', color: HydroColors.getKeyColor(2) },
            { id: 'B-1', key: 'B', value: 'B2', color: HydroColors.getKeyColor(1) },
            { id: 'A-2', key: 'A', value: 'A3', color: HydroColors.getKeyColor(0) }
        ];

        const timeline = [];
        let outputIndex = 0;
        const keyMessageCounts = { A: 0, B: 0, C: 0 };

        const outputPositions = outputContainer.verticalChildPositions(20, allMessages.length);

        allMessages.forEach((msg, index) => {
            const messageIndex = keyMessageCounts[msg.key];
            const keyGroup = keyGroups[msg.key];
            const contentCenter = keyGroup.getContentCenter();

            // Position first element (index 0) on the right, subsequent elements to the left
            const startX = contentCenter.x + 50 - messageIndex * 25; // Start from right side of content area
            const outputPosition = outputPositions[outputIndex]; // Use computed position

            world.addElement(
                <AnimatedMessage
                    key={`animating-${msg.id}`}
                    id={`animating-${msg.id}`}
                    x={startX}
                    y={contentCenter.y}
                    text={msg.value}
                    color={msg.color}
                />
            );

            world.addElement(
                <OutputMessage
                    key={`output-${msg.id}`}
                    id={`output-${msg.id}`}
                    x={outputPosition.x}
                    y={outputPosition.y}
                    text={`(${msg.key}, ${msg.value})`}
                    color={msg.color}
                />
            );

            timeline.push({
                elementId: `animating-${msg.id}`, props: {
                    ...outputPosition,
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

        world.createArrow('arrow1', inputContainer, entriesOperator);
        world.createArrow('arrow2', entriesOperator, outputContainer);

        return timeline;
    };

    return (
        <AnimationDiagram
            svgWidth={480}
            svgHeight={200}
            setupAnimation={setupAnimation}
        />
    );
};

export default KeyedStreamAnimation;
