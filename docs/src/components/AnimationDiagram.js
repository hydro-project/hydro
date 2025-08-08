import React, { useRef, useEffect, useMemo, useState } from 'react';
import { animate, createScope, createTimeline } from 'animejs';
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
    const [isPlaying, setIsPlaying] = useState(false);

    const [world, timeline] = useMemo(() => {
        const world = new World(svgWidth, svgHeight);
        const timeline = setupAnimation(world);
        return [world, timeline];
    }, [setupAnimation]);

    useEffect(() => {
        scope.current = createScope({ root: root.current }).add(self => {
            const animeTimeline = createTimeline({
                autoplay: false,
                onComplete: () => {
                    setIsPlaying(false);
                }
            });

            timeline.forEach((event) => {
                animeTimeline.add(`#${event.elementId}`, event.props, event.time);
            });

            animeTimeline.add(
                "#progress-fill",
                {
                    width: ['0%', '100%'],
                    duration: animeTimeline.duration,
                    easing: 'linear'
                },
                0
            );

            self.add('start', () => {
                setIsPlaying(true);
                animeTimeline.restart();
            });

            self.add('restart', () => {
                animeTimeline.pause();
                animate(animeTimeline, {
                    currentTime: 0,
                    duration: 250,
                    ease: 'inOutQuad',
                });
                setIsPlaying(false);
            });
        });

        return () => scope.current?.revert();
    }, [world, timeline]);

    return (
        <div className={styles.container} ref={root}>
            <svg
                width={svgWidth}
                height={svgHeight}
                viewBox={`0 0 ${svgWidth} ${svgHeight}`}
            >
                {/* Render all world elements */}
                {world.getElements()}

                {/* Arrow marker definition */}
                <defs>
                    <ArrowMarker />
                </defs>
            </svg>

            <div className={styles.controlsContainer}>
                <div className={styles.playControls}>
                    <button
                        onClick={() => {
                            if (isPlaying) {
                                scope.current?.methods.restart();
                            } else {
                                scope.current?.methods.start();
                            }
                        }}
                        className={styles.playButton}
                    >
                        {isPlaying ? (
                            <svg className={styles.playIcon} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                                <path d="M1 4v6h6" />
                                <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
                            </svg>
                        ) : (
                            <svg className={styles.playIcon} viewBox="0 0 24 24" fill="currentColor">
                                <path d="M8 5v14l11-7z" />
                            </svg>
                        )}
                    </button>
                    <div className={styles.progressContainer}>
                        <div className={styles.progressBar}>
                            <div
                                id="progress-fill"
                                className={styles.progressFill}
                                style={{ width: '0%' }}
                            />
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
};

export default AnimationDiagram;
