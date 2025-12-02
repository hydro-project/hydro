import React from 'react';
import AnimationDiagram from '@site/src/components/AnimationDiagram';
import AnimatedMessage from '@site/src/components/AnimatedMessage';
import { Point } from '@site/src/components/animation-utils';
import HydroColors from '@site/src/components/hydro-colors';

const SliceSnapshotAnimation = () => {
    const setupAnimation = (world) => {
        const center = world.getCenter();

        // Define vertical positions for alignment
        const streamY = center.y - 32;
        const singletonY = center.y + 32;

        // Create the main components
        const streamContainer = world.createCollectionBox(
            'stream',
            new Point(55, streamY),
            90, 65, 18,
            "Stream<()>"
        );

        const singletonContainer = world.createCollectionBox(
            'singleton',
            new Point(55, singletonY),
            90, 38, 18,
            "Singleton<i32>"
        );

        // Create sliced region (dotted rectangle) surrounding the processing area
        world.createSlicedRegion(
            'sliced',
            new Point(center.x + 20, center.y),
            200, 125,
            'sliced!'
        );

        // Create operators inside the sliced region
        const useStreamOp = world.createOperatorBox(
            'use-stream',
            new Point(center.x - 25, streamY),
            50, 22,
            "use"
        );

        const useSingletonOp = world.createOperatorBox(
            'use-singleton',
            new Point(center.x - 25, singletonY),
            50, 22,
            "use"
        );

        const crossOp = world.createOperatorBox(
            'cross',
            new Point(center.x + 55, center.y),
            50, 26,
            "cross"
        );

        // Arrows inside sliced region
        world.createArrow('use-stream-to-cross', useStreamOp, crossOp);
        world.createArrow('use-singleton-to-cross', useSingletonOp, crossOp);

        const outputContainer = world.createCollectionBox(
            'output',
            new Point(425, center.y),
            100, 95, 18,
            "Stream<((),i32)>"
        );

        // Add singleton value displays (two separate elements for dissolve effect)
        const singletonBounds = singletonContainer.getContentBounds();
        
        // Value "5" - visible initially
        world.addText('singleton-value-5', new Point(singletonBounds.centerX, singletonBounds.centerY), '5', {
            width: 30,
            fontSize: '11px',
            opacity: 1
        });
        
        // Value "7" - hidden initially
        world.addText('singleton-value-7', new Point(singletonBounds.centerX, singletonBounds.centerY), '7', {
            width: 30,
            fontSize: '11px',
            opacity: 0
        });

        // Request messages (3 total: 2 in batch 1, 1 in batch 2)
        const streamPositions = streamContainer.verticalChildPositions(12, 3);

        // Batch 1 requests
        world.addElement(
            <AnimatedMessage
                key="request-b1-1"
                id="request-b1-1"
                x={streamPositions[0].x}
                y={streamPositions[0].y}
                width={14}
                height={11}
                text="()"
                fontSize={7}
                color={HydroColors.blue}
            />
        );
        world.addElement(
            <AnimatedMessage
                key="request-b1-2"
                id="request-b1-2"
                x={streamPositions[1].x}
                y={streamPositions[1].y}
                width={14}
                height={11}
                text="()"
                fontSize={7}
                color={HydroColors.blue}
            />
        );

        // Batch 2 request
        world.addElement(
            <AnimatedMessage
                key="request-b2-1"
                id="request-b2-1"
                x={streamPositions[2].x}
                y={streamPositions[2].y}
                width={14}
                height={11}
                text="()"
                fontSize={7}
                color={HydroColors.green}
            />
        );

        // Singleton snapshots for each batch
        world.addElement(
            <AnimatedMessage
                key="snapshot-b1"
                id="snapshot-b1"
                x={singletonBounds.centerX}
                y={singletonBounds.centerY}
                width={18}
                height={12}
                text="5"
                fontSize={9}
                color={HydroColors.purple}
                opacity={0}
            />
        );
        world.addElement(
            <AnimatedMessage
                key="snapshot-b2"
                id="snapshot-b2"
                x={singletonBounds.centerX}
                y={singletonBounds.centerY}
                width={18}
                height={12}
                text="7"
                fontSize={9}
                color={HydroColors.purple}
                opacity={0}
            />
        );

        // Output tuples
        const outputPositions = outputContainer.verticalChildPositions(18, 3);
        
        // Batch 1 tuples (value 5)
        world.addElement(
            <AnimatedMessage
                key="tuple-b1-1"
                id="tuple-b1-1"
                x={center.x + 55}
                y={center.y}
                width={44}
                height={14}
                text="((), 5)"
                fontSize={8}
                color={HydroColors.blue}
                opacity={0}
            />
        );
        world.addElement(
            <AnimatedMessage
                key="tuple-b1-2"
                id="tuple-b1-2"
                x={center.x + 55}
                y={center.y}
                width={44}
                height={14}
                text="((), 5)"
                fontSize={8}
                color={HydroColors.blue}
                opacity={0}
            />
        );

        // Batch 2 tuple (value 7)
        world.addElement(
            <AnimatedMessage
                key="tuple-b2-1"
                id="tuple-b2-1"
                x={center.x + 55}
                y={center.y}
                width={44}
                height={14}
                text="((), 7)"
                fontSize={8}
                color={HydroColors.green}
                opacity={0}
            />
        );

        // Create arrows - go directly to use nodes
        world.createArrow('arrow1', streamContainer, useStreamOp);
        world.createArrow('arrow2', singletonContainer, useSingletonOp);
        world.createArrow('arrow3', crossOp, outputContainer);

        // Build timeline
        const timeline = [];
        const useStreamX = center.x - 25;
        const useStreamY = streamY;
        const useSingletonX = center.x - 25;
        const useSingletonY = singletonY;
        const crossX = center.x + 55;
        const crossY = center.y;

        // ========== BATCH 1 (2 elements, singleton = 5) ==========
        
        // Phase 1: Inputs arrive at use operators
        timeline.push({
            elementId: 'snapshot-b1',
            props: { opacity: 1, duration: 80 },
            time: '+=0'
        });
        timeline.push({
            elementId: 'snapshot-b1',
            props: { x: useSingletonX, y: useSingletonY, duration: 350, easing: 'easeInOutCubic' },
            time: '+=30'
        });
        timeline.push({
            elementId: 'request-b1-1',
            props: { x: useStreamX, y: useStreamY - 8, duration: 350, easing: 'easeInOutCubic' },
            time: '<'
        });
        timeline.push({
            elementId: 'request-b1-2',
            props: { x: useStreamX, y: useStreamY + 8, duration: 350, easing: 'easeInOutCubic' },
            time: '<'
        });

        // Phase 2: Pulse and immediately unpulse after elements arrive at use
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

        // Process inside slice
        timeline.push({
            elementId: 'snapshot-b1',
            props: { x: crossX - 12, y: crossY + 10, duration: 250, easing: 'easeInOutCubic' },
            time: '+=50'
        });

        // Request 1 -> cross -> tuple
        timeline.push({
            elementId: 'request-b1-1',
            props: { x: crossX - 12, y: crossY - 10, duration: 200, easing: 'easeInOutCubic' },
            time: '+=100'
        });
        timeline.push({
            elementId: 'request-b1-1',
            props: { opacity: 0, duration: 80 },
            time: '+=80'
        });
        timeline.push({
            elementId: 'tuple-b1-1',
            props: { opacity: 1, y: crossY - 15, duration: 80 },
            time: '<'
        });

        // Request 2 -> cross -> tuple
        timeline.push({
            elementId: 'request-b1-2',
            props: { x: crossX - 12, y: crossY - 10, duration: 200, easing: 'easeInOutCubic' },
            time: '+=80'
        });
        timeline.push({
            elementId: 'request-b1-2',
            props: { opacity: 0, duration: 80 },
            time: '+=80'
        });
        timeline.push({
            elementId: 'tuple-b1-2',
            props: { opacity: 1, y: crossY + 15, duration: 80 },
            time: '<'
        });

        // Snapshot fades
        timeline.push({
            elementId: 'snapshot-b1',
            props: { opacity: 0, duration: 100 },
            time: '+=50'
        });

        // Phase 3: Yield batch 1 results
        timeline.push({
            elementId: 'tuple-b1-1',
            props: { x: outputPositions[0].x, y: outputPositions[0].y, duration: 300, easing: 'easeInOutCubic' },
            time: '+=50'
        });
        timeline.push({
            elementId: 'tuple-b1-2',
            props: { x: outputPositions[1].x, y: outputPositions[1].y, duration: 300, easing: 'easeInOutCubic' },
            time: '<'
        });

        // ========== SINGLETON UPDATES ==========
        // Dissolve from 5 to 7
        timeline.push({
            elementId: 'singleton-value-5',
            props: { opacity: 0, duration: 300 },
            time: '+=200'
        });
        timeline.push({
            elementId: 'singleton-value-7',
            props: { opacity: 1, duration: 300 },
            time: '<'
        });

        // ========== BATCH 2 (1 element, singleton = 7) ==========
        
        // Phase 1: Inputs arrive at use operators
        timeline.push({
            elementId: 'snapshot-b2',
            props: { opacity: 1, duration: 80 },
            time: '+=200'
        });
        timeline.push({
            elementId: 'snapshot-b2',
            props: { x: useSingletonX, y: useSingletonY, duration: 350, easing: 'easeInOutCubic' },
            time: '+=30'
        });
        timeline.push({
            elementId: 'request-b2-1',
            props: { x: useStreamX, y: useStreamY, duration: 350, easing: 'easeInOutCubic' },
            time: '<'
        });

        // Phase 2: Pulse and immediately unpulse after elements arrive at use
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

        // Process inside slice
        timeline.push({
            elementId: 'snapshot-b2',
            props: { x: crossX - 12, y: crossY + 10, duration: 250, easing: 'easeInOutCubic' },
            time: '+=50'
        });

        // Request -> cross -> tuple
        timeline.push({
            elementId: 'request-b2-1',
            props: { x: crossX - 12, y: crossY - 10, duration: 200, easing: 'easeInOutCubic' },
            time: '+=100'
        });
        timeline.push({
            elementId: 'request-b2-1',
            props: { opacity: 0, duration: 80 },
            time: '+=80'
        });
        timeline.push({
            elementId: 'tuple-b2-1',
            props: { opacity: 1, duration: 80 },
            time: '<'
        });

        // Snapshot fades
        timeline.push({
            elementId: 'snapshot-b2',
            props: { opacity: 0, duration: 100 },
            time: '+=50'
        });

        // Phase 3: Yield batch 2 result
        timeline.push({
            elementId: 'tuple-b2-1',
            props: { x: outputPositions[2].x, y: outputPositions[2].y, duration: 300, easing: 'easeInOutCubic' },
            time: '+=50'
        });

        return timeline;
    };

    return (
        <AnimationDiagram
            svgWidth={480}
            svgHeight={150}
            setupAnimation={setupAnimation}
        />
    );
};

export default SliceSnapshotAnimation;
