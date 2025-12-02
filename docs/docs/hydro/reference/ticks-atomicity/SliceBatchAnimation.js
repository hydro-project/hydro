import React from 'react';
import AnimationDiagram from '@site/src/components/AnimationDiagram';
import AnimatedMessage from '@site/src/components/AnimatedMessage';
import { Point } from '@site/src/components/animation-utils';
import HydroColors from '@site/src/components/hydro-colors';

const SliceBatchAnimation = () => {
    const setupAnimation = (world) => {
        const center = world.getCenter();

        // Create the main components
        const inputContainer = world.createCollectionBox(
            'input',
            new Point(60, center.y),
            100, 160, 25,
            "Stream<i32>"
        );

        // Create sliced region (dotted rectangle) surrounding the processing area
        const slicedRegion = world.createSlicedRegion(
            'sliced',
            new Point(center.x, center.y),
            190, 110,
            'sliced!'
        );

        // Create operators inside the sliced region - use and transform
        const useOp = world.createOperatorBox(
            'use-op',
            new Point(center.x - 45, center.y),
            55, 28,
            "use"
        );

        const mapOp = world.createOperatorBox(
            'map-op',
            new Point(center.x + 45, center.y),
            55, 28,
            "map"
        );

        // Arrow between use and map inside the sliced region
        world.createArrow('use-to-map', useOp, mapOp);

        const outputContainer = world.createCollectionBox(
            'output',
            new Point(420, center.y),
            100, 160, 25,
            "Stream<String>"
        );

        // Input messages - map converts i32 to String
        const inputMessages = [
            { id: 'in-1', value: 1, outValue: '"1"', color: HydroColors.blue },
            { id: 'in-2', value: 2, outValue: '"2"', color: HydroColors.blue },
            { id: 'in-3', value: 3, outValue: '"3"', color: HydroColors.teal },
            { id: 'in-4', value: 4, outValue: '"4"', color: HydroColors.teal },
            { id: 'in-5', value: 5, outValue: '"5"', color: HydroColors.green },
        ];

        // Position input messages vertically
        const inputPositions = inputContainer.verticalChildPositions(20, inputMessages.length);
        const outputPositions = outputContainer.verticalChildPositions(20, inputMessages.length);

        // Add input messages
        inputMessages.forEach((msg, index) => {
            world.addElement(
                <AnimatedMessage
                    key={`input-${msg.id}`}
                    id={`input-${msg.id}`}
                    x={inputPositions[index].x}
                    y={inputPositions[index].y}
                    width={24}
                    text={`${msg.value}`}
                    color={msg.color}
                />
            );
        });

        // Add transformed output messages (initially hidden, will appear after map)
        inputMessages.forEach((msg) => {
            world.addElement(
                <AnimatedMessage
                    key={`transformed-${msg.id}`}
                    id={`transformed-${msg.id}`}
                    x={center.x + 45}
                    y={center.y}
                    width={28}
                    text={`${msg.outValue}`}
                    color={msg.color}
                    opacity={0}
                />
            );
        });

        // Create arrows - go directly to operators
        world.createArrow('arrow1', inputContainer, useOp);
        world.createArrow('arrow2', mapOp, outputContainer);

        // Build timeline - process in batches
        const timeline = [];
        const useX = center.x - 45;
        const mapX = center.x + 45;

        // Batch 1: elements 1, 2
        // Move to use operator
        timeline.push({
            elementId: 'input-in-1',
            props: { x: useX, y: center.y - 8, duration: 400, easing: 'easeInOutCubic' },
            time: '+=0'
        });
        timeline.push({
            elementId: 'input-in-2',
            props: { x: useX, y: center.y + 8, duration: 400, easing: 'easeInOutCubic' },
            time: '<'
        });

        // Pulse and immediately unpulse after elements arrive at use
        timeline.push({
            elementId: 'sliced',
            props: { stroke: HydroColors.blue, duration: 150 },
            time: '<+=50'
        });
        timeline.push({
            elementId: 'sliced',
            props: { stroke: '#888', duration: 200 },
            time: '<+=150'
        });

        // Move to map operator
        timeline.push({
            elementId: 'input-in-1',
            props: { x: mapX, y: center.y - 8, duration: 300, easing: 'easeInOutCubic' },
            time: '+=100'
        });
        timeline.push({
            elementId: 'input-in-2',
            props: { x: mapX, y: center.y + 8, duration: 300, easing: 'easeInOutCubic' },
            time: '<'
        });

        // Hide inputs, show transformed outputs at map position
        timeline.push({
            elementId: 'input-in-1',
            props: { opacity: 0, duration: 150 },
            time: '+=150'
        });
        timeline.push({
            elementId: 'input-in-2',
            props: { opacity: 0, duration: 150 },
            time: '<'
        });
        timeline.push({
            elementId: 'transformed-in-1',
            props: { opacity: 1, y: center.y - 8, duration: 150 },
            time: '<'
        });
        timeline.push({
            elementId: 'transformed-in-2',
            props: { opacity: 1, y: center.y + 8, duration: 150 },
            time: '<'
        });

        // Animate transformed elements to output container
        timeline.push({
            elementId: 'transformed-in-1',
            props: { x: outputPositions[0].x, y: outputPositions[0].y, duration: 400, easing: 'easeInOutCubic' },
            time: '+=100'
        });
        timeline.push({
            elementId: 'transformed-in-2',
            props: { x: outputPositions[1].x, y: outputPositions[1].y, duration: 400, easing: 'easeInOutCubic' },
            time: '<'
        });

        // Batch 2: elements 3, 4
        // Move to use operator
        timeline.push({
            elementId: 'input-in-3',
            props: { x: useX, y: center.y - 8, duration: 400, easing: 'easeInOutCubic' },
            time: '+=200'
        });
        timeline.push({
            elementId: 'input-in-4',
            props: { x: useX, y: center.y + 8, duration: 400, easing: 'easeInOutCubic' },
            time: '<'
        });

        // Pulse and immediately unpulse after elements arrive at use
        timeline.push({
            elementId: 'sliced',
            props: { stroke: HydroColors.teal, duration: 150 },
            time: '<+=50'
        });
        timeline.push({
            elementId: 'sliced',
            props: { stroke: '#888', duration: 200 },
            time: '<+=150'
        });

        timeline.push({
            elementId: 'input-in-3',
            props: { x: mapX, y: center.y - 8, duration: 300, easing: 'easeInOutCubic' },
            time: '+=100'
        });
        timeline.push({
            elementId: 'input-in-4',
            props: { x: mapX, y: center.y + 8, duration: 300, easing: 'easeInOutCubic' },
            time: '<'
        });

        timeline.push({
            elementId: 'input-in-3',
            props: { opacity: 0, duration: 150 },
            time: '+=150'
        });
        timeline.push({
            elementId: 'input-in-4',
            props: { opacity: 0, duration: 150 },
            time: '<'
        });
        timeline.push({
            elementId: 'transformed-in-3',
            props: { opacity: 1, y: center.y - 8, duration: 150 },
            time: '<'
        });
        timeline.push({
            elementId: 'transformed-in-4',
            props: { opacity: 1, y: center.y + 8, duration: 150 },
            time: '<'
        });

        // Animate to output
        timeline.push({
            elementId: 'transformed-in-3',
            props: { x: outputPositions[2].x, y: outputPositions[2].y, duration: 400, easing: 'easeInOutCubic' },
            time: '+=100'
        });
        timeline.push({
            elementId: 'transformed-in-4',
            props: { x: outputPositions[3].x, y: outputPositions[3].y, duration: 400, easing: 'easeInOutCubic' },
            time: '<'
        });

        // Batch 3: element 5
        // Move to use operator
        timeline.push({
            elementId: 'input-in-5',
            props: { x: useX, y: center.y, duration: 400, easing: 'easeInOutCubic' },
            time: '+=200'
        });

        // Pulse and immediately unpulse after element arrives at use
        timeline.push({
            elementId: 'sliced',
            props: { stroke: HydroColors.green, duration: 150 },
            time: '<+=50'
        });
        timeline.push({
            elementId: 'sliced',
            props: { stroke: '#888', duration: 200 },
            time: '<+=150'
        });

        timeline.push({
            elementId: 'input-in-5',
            props: { x: mapX, y: center.y, duration: 300, easing: 'easeInOutCubic' },
            time: '+=100'
        });

        timeline.push({
            elementId: 'input-in-5',
            props: { opacity: 0, duration: 150 },
            time: '+=150'
        });
        timeline.push({
            elementId: 'transformed-in-5',
            props: { opacity: 1, duration: 150 },
            time: '<'
        });

        // Animate to output
        timeline.push({
            elementId: 'transformed-in-5',
            props: { x: outputPositions[4].x, y: outputPositions[4].y, duration: 400, easing: 'easeInOutCubic' },
            time: '+=100'
        });

        return timeline;
    };

    return (
        <AnimationDiagram
            svgWidth={480}
            svgHeight={180}
            setupAnimation={setupAnimation}
        />
    );
};

export default SliceBatchAnimation;
