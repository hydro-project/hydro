import React from 'react';
import AnimationDiagram from '@site/src/components/AnimationDiagram';
import AnimatedMessage from '@site/src/components/AnimatedMessage';
import { Point } from '@site/src/components/animation-utils';
import HydroColors from '@site/src/components/hydro-colors';

const KeyedSingletonRequestAnimation = () => {
    const setupAnimation = (world) => {
        const center = world.getCenter();

        const inputContainer = world.createCollectionBox(
            'input',
            new Point(102, center.y),
            195, 215, 25,
            "KeyedSingleton<ReqId, Request>"
        );

        const mapOperator = world.createOperatorBox(
            'map',
            new Point(center.x, center.y),
            100, 40,
            ".map(q!(handle))"
        );

        const outputContainer = world.createCollectionBox(
            'output',
            new Point(458, center.y),
            195, 215, 25,
            "KeyedSingleton<ReqId, Response>"
        );

        const requests = [
            { id: 'req1', label: 'req #1', value: 'GET /a', response: '"a" ✓', color: HydroColors.getKeyColor(0) },
            { id: 'req2', label: 'req #2', value: 'GET /b', response: '"b" ✓', color: HydroColors.getKeyColor(1) },
            { id: 'req3', label: 'req #3', value: 'GET /c', response: '"c" ✓', color: HydroColors.getKeyColor(2) }
        ];

        const inputPositions = inputContainer.verticalChildPositions(50, requests.length);
        const outputPositions = outputContainer.verticalChildPositions(50, requests.length);

        const timeline = [];

        requests.forEach((req, index) => {
            const inputGroup = world.createKeyGroup(`input-${req.id}`, inputPositions[index], 160, 50, req.label, req.color);
            const outputGroup = world.createKeyGroup(`output-${req.id}`, outputPositions[index], 160, 50, req.label, req.color);

            const inputCenter = inputGroup.getContentCenter();
            const outputCenter = outputGroup.getContentCenter();

            // The request value, revealed when the key "arrives"; stays in the
            // input collection since it is immutable
            world.addElement(
                <AnimatedMessage
                    key={`request-${req.id}`}
                    id={`request-${req.id}`}
                    x={inputCenter.x}
                    y={inputCenter.y}
                    width={56}
                    text={req.value}
                    color={req.color}
                    opacity={0}
                />
            );

            // A copy of the request that flies to the map operator
            world.addElement(
                <AnimatedMessage
                    key={`flying-${req.id}`}
                    id={`flying-${req.id}`}
                    x={inputCenter.x}
                    y={inputCenter.y}
                    width={56}
                    text={req.value}
                    color={req.color}
                    opacity={0}
                />
            );

            // The response, revealed at the operator and flying to the output
            world.addElement(
                <AnimatedMessage
                    key={`response-${req.id}`}
                    id={`response-${req.id}`}
                    x={center.x}
                    y={center.y}
                    width={56}
                    text={req.response}
                    color={req.color}
                    opacity={0}
                />
            );

            // 1. The key arrives asynchronously with its immutable request value
            timeline.push({
                elementId: `request-${req.id}`,
                props: { opacity: 1, duration: 300, easing: 'easeOutCubic' },
                time: index === 0 ? '+=200' : '+=500'
            });

            // 2. A copy of the request flows to the `.map()` operator
            timeline.push({
                elementId: `flying-${req.id}`,
                props: { opacity: 1, duration: 150 },
                time: '+=150'
            });
            timeline.push({
                elementId: `flying-${req.id}`,
                props: { x: center.x, y: center.y, duration: 450, easing: 'easeInOutCubic' },
                time: '+=0'
            });

            // 3. The operator transforms the request into a response
            timeline.push({
                elementId: `flying-${req.id}`,
                props: { opacity: 0, duration: 150 },
                time: '+=100'
            });
            timeline.push({
                elementId: `response-${req.id}`,
                props: { opacity: 1, duration: 150 },
                time: '<'
            });

            // 4. The response settles into the output for this key, and never
            // changes afterwards
            timeline.push({
                elementId: `response-${req.id}`,
                props: { x: outputCenter.x, y: outputCenter.y, duration: 450, easing: 'easeInOutCubic' },
                time: '+=100'
            });
        });

        world.createArrow('arrow1', inputContainer, mapOperator);
        world.createArrow('arrow2', mapOperator, outputContainer);

        return timeline;
    };

    return (
        <AnimationDiagram
            svgWidth={560}
            svgHeight={240}
            setupAnimation={setupAnimation}
        />
    );
};

export default KeyedSingletonRequestAnimation;
