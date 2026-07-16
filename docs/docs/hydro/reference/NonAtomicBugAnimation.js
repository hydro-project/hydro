import AnimationDiagram from '@site/src/components/AnimationDiagram';
import AnimatedMessage from '@site/src/components/AnimatedMessage';
import OutputMessage from '@site/src/components/OutputMessage';
import Arrow from '@site/src/components/Arrow';
import { Point } from '@site/src/components/animation-utils';
import HydroColors from '@site/src/components/hydro-colors';

const NonAtomicBugAnimation = () => {
    const setupAnimation = (world) => {
        const center = world.getCenter();

        // Layout: increment flow on top, get flow on bottom
        const topY = center.y - 40;
        const countY = topY + 15; // count path slightly below
        const ackY = topY - 15;   // ack path on top
        const bottomY = center.y + 40;

        // Left side: inputs
        const incrementInput = world.createCollectionBox(
            'incInput',
            new Point(55, topY),
            90, 40, 16,
            "increment"
        );

        const getInput = world.createCollectionBox(
            'getInput',
            new Point(55, bottomY),
            90, 40, 16,
            "get"
        );

        // Middle: count operator (count path)
        const countOp = world.createOperatorBox(
            'count',
            new Point(170, countY),
            50, 26,
            "count"
        );

        // Sliced region (bottom area, spanning middle)
        const slicedX = 260;
        const slicedWidth = 140;
        world.createSlicedRegion(
            'sliced',
            new Point(slicedX, bottomY),
            slicedWidth, 70,
            'sliced!'
        );



        // Inside sliced: use for singleton (top), use for stream (bottom), cross
        const useSingletonOp = world.createOperatorBox(
            'use-singleton',
            new Point(slicedX - 40, bottomY - 15),
            45, 20,
            "use"
        );

        const useStreamOp = world.createOperatorBox(
            'use-stream',
            new Point(slicedX - 40, bottomY + 15),
            45, 20,
            "use"
        );

        const crossOp = world.createOperatorBox(
            'cross',
            new Point(slicedX + 35, bottomY),
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

        // Dashed arrow from count operator to use-singleton
        world.createArrow('count-to-use', countOp, useSingletonOp, { dashed: true, startSide: 'bottom', endSide: 'left' });

        // External arrows (added before animated messages for correct z-index)
        const incBoundsBox = incrementInput.getBounds();
        const ackBoundsBox = ackOutput.getBounds();
        const countBoundsBox = countOp.getBounds();
        world.addElement(
            <Arrow
                key="inc-to-ack-arrow"
                startX={incBoundsBox.right}
                startY={ackY}
                endX={ackBoundsBox.left}
                endY={ackY}
            />
        );
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
                opacity={0}
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
        const ackBounds = ackOutput.getContentBounds();
        world.addElement(
            <OutputMessage
                key="ack-msg"
                id="ack-msg"
                x={ackBounds.centerX}
                y={ackBounds.centerY}
                width={28}
                height={14}
                text="ok"
                fontSize={8}
                color={HydroColors.blue}
            />
        );

        const respBounds = responseOutput.getContentBounds();
        const crossBounds = crossOp.getBounds();
        world.addElement(
            <AnimatedMessage
                key="resp-msg"
                id="resp-msg"
                x={crossBounds.centerX}
                y={crossBounds.centerY}
                width={28}
                height={14}
                text="0"
                fontSize={8}
                color={HydroColors.coral}
                opacity={0}
            />
        );

        // Snapshotted "0" value that will move from use-singleton to cross
        // Starts at the same position where count-zero ends up (left of use-singleton)
        const useSingletonBounds = useSingletonOp.getBounds();
        world.addElement(
            <AnimatedMessage
                key="snapshot-zero"
                id="snapshot-zero"
                x={useSingletonBounds.left}
                y={useSingletonBounds.centerY}
                width={16}
                height={12}
                text="0"
                fontSize={8}
                color={HydroColors.coral}
                opacity={0}
            />
        );

        // Bug indicator
        world.addElement(
            <g key="bug-indicator" id="bug-indicator" opacity={0}>
                <text
                    x={425}
                    y={bottomY + 35}
                    textAnchor="middle"
                    fontSize="9"
                    fontWeight="bold"
                    fill={HydroColors.coral}
                >
                    ⚠ Stale read!
                </text>
            </g>
        );

        // Timeline
        const timeline = [];

        // Initial "0" value propagates quickly from count to use-singleton
        timeline.push({
            elementId: 'count-zero',
            props: { x: useSingletonBounds.left, y: useSingletonBounds.centerY, duration: 500, easing: 'easeOutCubic' },
            time: '+=0'
        });

        // Both +1 messages start animating at the same time
        // Show the second +1 (was hidden)
        timeline.push({
            elementId: 'inc-to-count',
            props: { opacity: 1, duration: 100 },
            time: '+=400'
        });
        // Ack path (shorter, finishes first)
        timeline.push({
            elementId: 'inc-to-ack',
            props: { x: ackBoundsBox.left, y: ackY, duration: 600, easing: 'easeInOutCubic' },
            time: '<'
        });
        // Count path (longer, finishes after ack)
        timeline.push({
            elementId: 'inc-to-count',
            props: { x: countBoundsBox.left, y: countY, duration: 1000, easing: 'easeInOutCubic' },
            time: '<<'
        });

        // Ack sent when ack message arrives (after 600ms)
        timeline.push({
            elementId: 'inc-to-ack',
            props: { opacity: 0, duration: 200 },
            time: '<<+=600'
        });
        timeline.push({
            elementId: 'ack-msg',
            props: { opacity: 1, duration: 200 },
            time: '<'
        });

        // Count message fades after it arrives (after 1000ms from start)
        timeline.push({
            elementId: 'inc-to-count',
            props: { opacity: 0, duration: 200 },
            time: '<<+=400'
        });

        // Start the slow "1" value propagation (will take a long time - runs continuously in background)
        timeline.push({
            elementId: 'count-one',
            props: { opacity: 1, duration: 150 },
            time: '<+=100'
        });
        
        // "1" moves slowly toward use-singleton (continuous animation, arrives after the bug is shown)
        timeline.push({
            elementId: 'count-one',
            props: { 
                x: useSingletonBounds.left, 
                y: useSingletonBounds.centerY,
                duration: 15000, 
                easing: 'linear' 
            },
            time: '<'
        });

        // Get request arrives BEFORE count-one reaches (positioned relative to start of slow "1" animation)
        timeline.push({
            elementId: 'get-req',
            props: { x: slicedX - 40, y: bottomY + 15, duration: 900, easing: 'easeInOutCubic' },
            time: '<<+=500'
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

        // Show the snapshotted 0 appearing from use-singleton
        timeline.push({
            elementId: 'snapshot-zero',
            props: { opacity: 1, duration: 200 },
            time: '<<+=200'
        });

        // Snapshotted 0 moves into cross first
        timeline.push({
            elementId: 'snapshot-zero',
            props: { x: crossBounds.centerX, y: crossBounds.centerY - 8, duration: 600, easing: 'easeInOutCubic' },
            time: '<+=150'
        });

        // Then get-req moves through cross operator (uses stale "0" value)
        timeline.push({
            elementId: 'get-req',
            props: { x: slicedX + 35, y: bottomY, duration: 600, easing: 'easeInOutCubic' },
            time: '<+=150'
        });

        // Response with stale value (0)
        timeline.push({
            elementId: 'get-req',
            props: { opacity: 0, duration: 250 },
            time: '<+=200'
        });
        timeline.push({
            elementId: 'snapshot-zero',
            props: { opacity: 0, duration: 200 },
            time: '<'
        });
        timeline.push({
            elementId: 'resp-msg',
            props: { opacity: 1, duration: 200 },
            time: '<'
        });
        // Animate response to output collection
        timeline.push({
            elementId: 'resp-msg',
            props: { x: respBounds.centerX, y: respBounds.centerY, duration: 400, easing: 'easeInOutCubic' },
            time: '<+=100'
        });

        // Show bug indicator
        timeline.push({
            elementId: 'bug-indicator',
            props: { opacity: 1, duration: 350 },
            time: '<<+=350'
        });

        // Hide the old "0" value as "1" arrives (the "1" animation completes around 5150ms)
        timeline.push({
            elementId: 'count-zero',
            props: { opacity: 0, duration: 200 },
            time: '<<+=2300'
        });

        return timeline;
    };

    return (
        <AnimationDiagram
            svgWidth={480}
            svgHeight={160}
            setupAnimation={setupAnimation}
        />
    );
};

export default NonAtomicBugAnimation;
