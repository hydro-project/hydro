import React from 'react';
import AnimationDiagram from '@site/src/components/AnimationDiagram';
import AnimatedMessage from '@site/src/components/AnimatedMessage';
import OutputMessage from '@site/src/components/OutputMessage';
import Arrow from '@site/src/components/Arrow';
import { Point } from '@site/src/components/animation-utils';
import HydroColors from '@site/src/components/hydro-colors';

const AtomicConsistencyAnimation = () => {
    const setupAnimation = (world) => {
        const center = world.getCenter();

        // Layout: increment flow on top, get flow on bottom
        // Shifted down to make room for atomic label above
        const topY = center.y - 32;
        const countY = topY + 15; // count path slightly below
        const ackY = topY - 15;   // ack path on top
        const bottomY = center.y + 45;

        // Left side: inputs
        const incrementInput = world.createCollectionBox(
            'incInput',
            new Point(55, topY),
            90, 40, 16,
            "increment"
        );

        const getInput = world.createCollectionBox(
            'getInput',
            new Point(55, bottomY + 20),
            90, 40, 16,
            "get"
        );

        // Atomic region containing end_atomic (ack path), count (count path), and use::atomic
        // The atomic region extends down to include use::atomic
        const atomicX = 170;
        const useAtomicY = bottomY - 15;
        const atomicTop = ackY - 18;
        const atomicBottom = useAtomicY + 18;
        world.createSlicedRegion(
            'atomic',
            new Point(atomicX, (atomicTop + atomicBottom) / 2),
            100, atomicBottom - atomicTop,
            'atomic',
            { strokeColor: HydroColors.purple, labelColor: HydroColors.purple }
        );

        // Inside atomic: end_atomic on ack path (top), count on count path (middle), use::atomic (bottom)
        const endAtomicOp = world.createOperatorBox(
            'end-atomic',
            new Point(atomicX, ackY),
            50, 20,
            "end"
        );

        const countOp = world.createOperatorBox(
            'count',
            new Point(atomicX, countY),
            50, 22,
            "count"
        );

        // use::atomic is below count, inside both atomic and sliced regions (overlapping)
        const useSingletonOp = world.createOperatorBox(
            'use-singleton',
            new Point(atomicX, useAtomicY),
            85, 20,
            "use::atomic"
        );

        // Sliced region (bottom area) - starts to the left to include use::atomic
        const slicedX = 210;
        const slicedWidth = 200;
        world.createSlicedRegion(
            'sliced',
            new Point(slicedX, bottomY),
            slicedWidth, 80,
            'sliced!',
            { labelPosition: 'right' }
        );

        const useStreamOp = world.createOperatorBox(
            'use-stream',
            new Point(atomicX, bottomY + 20),
            45, 20,
            "use"
        );

        const crossOp = world.createOperatorBox(
            'cross',
            new Point(slicedX + 65, bottomY),
            50, 26,
            "cross"
        );

        // Arrows inside sliced
        world.createArrow('use-singleton-to-cross', useSingletonOp, crossOp);
        world.createArrow('use-stream-to-cross', useStreamOp, crossOp);

        // Right side: outputs
        const ackOutput = world.createCollectionBox(
            'ackOutput',
            new Point(425, ackY),
            90, 40, 16,
            "ack"
        );

        const responseOutput = world.createCollectionBox(
            'responseOutput',
            new Point(425, bottomY),
            90, 40, 16,
            "response"
        );

        // Dashed arrow from count to use::atomic (vertical, both in atomic region)
        world.createArrow('count-to-use', countOp, useSingletonOp, { dashed: true, startSide: 'bottom', endSide: 'top' });

        // External arrows
        const incBoundsBox = incrementInput.getBounds();
        const countBoundsBox = countOp.getBounds();
        const endAtomicBounds = endAtomicOp.getBounds();
        const ackBoundsBox = ackOutput.getBounds();
        // Arrow from increment to end_atomic (ack path)
        world.addElement(
            <Arrow
                key="inc-to-end-arrow"
                startX={incBoundsBox.right}
                startY={ackY}
                endX={endAtomicBounds.left}
                endY={ackY}
            />
        );
        // Arrow from end_atomic to ack
        world.addElement(
            <Arrow
                key="end-to-ack"
                startX={endAtomicBounds.right}
                startY={ackY}
                endX={ackBoundsBox.left}
                endY={ackY}
            />
        );
        // Arrow from increment to count (count path)
        world.addElement(
            <Arrow
                key="inc-to-count-arrow"
                startX={incBoundsBox.right}
                startY={countY}
                endX={countBoundsBox.left}
                endY={countY}
            />
        );
        world.createArrow('arr3', getInput, useStreamOp);
        world.createArrow('arr4', crossOp, responseOutput);

        // Input messages - two copies of +1 for ack and count paths
        const incBounds = incrementInput.getContentBounds();
        world.addElement(
            <AnimatedMessage
                key="inc-to-ack"
                id="inc-to-ack"
                x={incBounds.centerX}
                y={incBounds.centerY}
                width={18}
                height={12}
                text="+1"
                fontSize={8}
                color={HydroColors.blue}
                opacity={0}
            />
        );
        world.addElement(
            <AnimatedMessage
                key="inc-to-count"
                id="inc-to-count"
                x={incBounds.centerX}
                y={incBounds.centerY}
                width={18}
                height={12}
                text="+1"
                fontSize={8}
                color={HydroColors.blue}
            />
        );

        const getBounds = getInput.getContentBounds();
        world.addElement(
            <AnimatedMessage
                key="get-req"
                id="get-req"
                x={getBounds.centerX}
                y={getBounds.centerY}
                width={18}
                height={12}
                text="?"
                fontSize={8}
                color={HydroColors.teal}
            />
        );

        // Counter value message - initially shows "0" traveling from count to use
        const countBounds = countOp.getBounds();
        world.addElement(
            <AnimatedMessage
                key="count-zero"
                id="count-zero"
                x={countBounds.centerX}
                y={countBounds.bottom}
                width={16}
                height={12}
                text="0"
                fontSize={8}
                color="#666"
            />
        );

        // Slow-moving "1" value (starts hidden, appears after increment)
        world.addElement(
            <AnimatedMessage
                key="count-one"
                id="count-one"
                x={countBounds.centerX}
                y={countBounds.bottom}
                width={16}
                height={12}
                text="1"
                fontSize={8}
                color={HydroColors.blue}
                opacity={0}
            />
        );

        // Output messages
        
        const ackContentBounds = ackOutput.getContentBounds();
        world.addElement(
            <OutputMessage
                key="ack-msg"
                id="ack-msg"
                x={ackContentBounds.centerX}
                y={ackContentBounds.centerY}
                width={28}
                height={14}
                text="ok"
                fontSize={8}
                color={HydroColors.blue}
            />
        );

        const respBounds = responseOutput.getContentBounds();
        world.addElement(
            <OutputMessage
                key="resp-msg"
                id="resp-msg"
                x={respBounds.centerX}
                y={respBounds.centerY}
                width={28}
                height={14}
                text="1"
                fontSize={8}
                color={HydroColors.green}
            />
        );

        // Snapshotted "1" value that will move from use-singleton to cross
        const useSingletonBounds = useSingletonOp.getBounds();
        world.addElement(
            <AnimatedMessage
                key="snapshot-one"
                id="snapshot-one"
                x={useSingletonBounds.centerX}
                y={useSingletonBounds.top}
                width={16}
                height={12}
                text="1"
                fontSize={8}
                color={HydroColors.green}
                opacity={0}
            />
        );

        // Success indicator
        world.addElement(
            <g key="success-indicator" id="success-indicator" opacity={0}>
                <text
                    x={425}
                    y={bottomY + 35}
                    textAnchor="middle"
                    fontSize="9"
                    fontWeight="bold"
                    fill={HydroColors.green}
                >
                    ✓ Consistent!
                </text>
            </g>
        );

        const crossBounds = crossOp.getBounds();

        // Timeline
        const timeline = [];

        // Initial "0" value propagates quickly from count to use-singleton (ends at top)
        timeline.push({
            elementId: 'count-zero',
            props: { x: useSingletonBounds.centerX, y: useSingletonBounds.top, duration: 500, easing: 'easeOutCubic' },
            time: '+=0'
        });

        // Both +1 messages start animating at the same time
        // Show the ack +1 (was hidden)
        timeline.push({
            elementId: 'inc-to-ack',
            props: { opacity: 1, duration: 100 },
            time: '+=400'
        });
        // Ack path - travels to end_atomic center (arrives and waits there, blocked)
        timeline.push({
            elementId: 'inc-to-ack',
            props: { x: endAtomicBounds.centerX, y: ackY, duration: 500, easing: 'easeInOutCubic' },
            time: '<'
        });
        // Count path - travels to count operator
        timeline.push({
            elementId: 'inc-to-count',
            props: { x: countBoundsBox.left, y: countY, duration: 600, easing: 'easeInOutCubic' },
            time: '<<'
        });

        // +1 at count fades, "1" starts propagating slowly
        timeline.push({
            elementId: 'inc-to-count',
            props: { opacity: 0, duration: 200 },
            time: '<+=200'
        });

        // Start the slow "1" value propagation
        timeline.push({
            elementId: 'count-one',
            props: { opacity: 1, duration: 150 },
            time: '<+=100'
        });
        
        // "1" moves slowly toward use::atomic (ends at top, this blocks ack release until it passes)
        timeline.push({
            elementId: 'count-one',
            props: { 
                x: useSingletonBounds.centerX, 
                y: useSingletonBounds.top,
                duration: 4000, 
                easing: 'linear' 
            },
            time: '<'
        });

        // Pulse atomic region while waiting
        timeline.push({
            elementId: 'atomic',
            props: { stroke: HydroColors.purple, duration: 200 },
            time: '<<+=200'
        });

        // After "1" has propagated (4000ms), ack can be released from end_atomic
        // (ack waits for count to propagate before releasing)
        timeline.push({
            elementId: 'inc-to-ack',
            props: { x: ackBoundsBox.left, y: ackY, duration: 500, easing: 'easeInOutCubic' },
            time: '<<+=4200'
        });
        
        // Unpulse atomic region as ack releases
        timeline.push({
            elementId: 'atomic',
            props: { stroke: HydroColors.purple, duration: 300 },
            time: '<'
        });

        timeline.push({
            elementId: 'inc-to-ack',
            props: { opacity: 0, duration: 200 },
            time: '<+=100'
        });
        timeline.push({
            elementId: 'ack-msg',
            props: { opacity: 1, duration: 200 },
            time: '<'
        });

        // Get request arrives AFTER ack (so it will see consistent "1")
        const useStreamBounds = useStreamOp.getBounds();
        timeline.push({
            elementId: 'get-req',
            props: { x: useStreamBounds.centerX, y: useStreamBounds.centerY, duration: 900, easing: 'easeInOutCubic' },
            time: '+=300'
        });

        // Pulse the sliced region border to show it's executing
        timeline.push({
            elementId: 'sliced',
            props: { stroke: HydroColors.teal, duration: 300 },
            time: '<+=150'
        });
        timeline.push({
            elementId: 'sliced',
            props: { stroke: '#888', duration: 500 },
            time: '<<+=400'
        });

        // Hide the old "0" value as "1" arrives
        timeline.push({
            elementId: 'count-zero',
            props: { opacity: 0, duration: 200 },
            time: '<<+=200'
        });

        // Show the snapshotted 1 appearing from use-singleton
        timeline.push({
            elementId: 'snapshot-one',
            props: { opacity: 1, duration: 200 },
            time: '<+=100'
        });

        // Snapshotted 1 moves into cross first
        timeline.push({
            elementId: 'snapshot-one',
            props: { x: crossBounds.centerX, y: crossBounds.centerY - 8, duration: 600, easing: 'easeInOutCubic' },
            time: '<+=150'
        });

        // Then get-req moves through cross operator (positioned below the snapshot)
        timeline.push({
            elementId: 'get-req',
            props: { x: crossBounds.centerX, y: crossBounds.centerY + 8, duration: 600, easing: 'easeInOutCubic' },
            time: '<+=150'
        });

        // Response with correct value (1)
        timeline.push({
            elementId: 'get-req',
            props: { opacity: 0, duration: 250 },
            time: '<+=200'
        });
        timeline.push({
            elementId: 'snapshot-one',
            props: { opacity: 0, duration: 200 },
            time: '<'
        });
        timeline.push({
            elementId: 'resp-msg',
            props: { opacity: 1, duration: 200 },
            time: '<'
        });

        // Show success indicator
        timeline.push({
            elementId: 'success-indicator',
            props: { opacity: 1, duration: 350 },
            time: '<<+=350'
        });

        // Hide count-one after it arrives
        timeline.push({
            elementId: 'count-one',
            props: { opacity: 0, duration: 200 },
            time: '<<+=1000'
        });

        return timeline;
    };

    return (
        <AnimationDiagram
            svgWidth={480}
            svgHeight={175}
            setupAnimation={setupAnimation}
        />
    );
};

export default AtomicConsistencyAnimation;
