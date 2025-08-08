import React, { useRef, useEffect } from 'react';
import { createScope, createTimeline } from 'animejs';
import styles from './svg-animation.module.css';
import { World } from './animation-utils';
import { ArrowMarker } from './Arrow';

const AnimationDiagram = ({ 
    svgWidth, 
    svgHeight, 
    setupAnimation 
}) => {
    const root = useRef(null);
    const scope = useRef(null);

    // Create world and setup animation
    const world = new World(svgWidth, svgHeight);
    const timeline = setupAnimation(world);

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
    }, [timeline]);

    return (
        <div className={styles.container}>
            <svg
                width={svgWidth} 
                height={svgHeight} 
                viewBox={`0 0 ${svgWidth} ${svgHeight}`} 
                ref={root}
            >
                {/* Render all world elements */}
                {world.getElements()}

                {/* Arrow marker definition */}
                <defs>
                    <ArrowMarker />
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

export default AnimationDiagram;