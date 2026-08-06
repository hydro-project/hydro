/**
 * GraphExtras: the panel rendered at the bottom of the diagram island —
 * the simulator trace for the sim scene, nothing otherwise (the container
 * smoothly collapses).
 */

import React, { useEffect, useLayoutEffect, useRef, useState } from "react";

// Avoid the useLayoutEffect SSR warning; on the server this only sets
// initial state, which the browser pass redoes.
const useIsomorphicLayoutEffect =
  typeof window !== "undefined" ? useLayoutEffect : useEffect;

import { COLOR_VARS, SIM_TOTAL_INSTANCES } from "./scenes";
import type { Frame, Scene, SceneKey } from "./scenes";
import styles from "./landing.module.css";

/**
 * The simulator log, formatted like a real `HYDRO_SIM_LOG=1` trace. The log
 * accumulates across all explored instances (with a header between each)
 * before resetting, and the scroll region stays pinned to the bottom so the
 * tail is always visible.
 */
function SimLog({ frame }: { frame: Frame }) {
  const memberColors: Record<number, string> = {
    0: COLOR_VARS.client,
    1: COLOR_VARS.chanB,
    2: COLOR_VARS.pink,
  };
  const log = frame.log ?? [];
  const scrollRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = scrollRef.current;
    if (el) el.scrollTo({ top: el.scrollHeight, behavior: "smooth" });
  }, [log.length]);

  return (
    <div className={`${styles.extraPanel} ${styles.simLogPanel}`}>
      <span className={`${styles.instanceChip} ${styles.instanceChipCorner}`}>
        instance {frame.instanceNum}/{SIM_TOTAL_INSTANCES}
      </span>
      <div className={styles.logLines} ref={scrollRef}>
        {log.map((line) => {
          switch (line.type) {
            case "header":
              return (
                <div key={line.key} className={styles.logMeta}>
                  ==== New Simulation Instance ====
                </div>
              );
            case "context":
              return (
                <div key={line.key} className={styles.logContext}>
                  {"--> "}.entries_partially_ordered(nondet!(…))
                </div>
              );
            case "decision":
              return (
                <div key={line.key} className={styles.logLine}>
                  {"     ^ observed interleaving: [("}
                  <span
                    style={{
                      color: memberColors[line.member],
                      fontWeight: 700,
                    }}
                  >
                    MemberId({line.member})
                  </span>
                  {`, "${line.msg}")]`}
                </div>
              );
            case "ok":
              return (
                <div key={line.key} className={styles.logOk}>
                  ✓ assert!(pos("a1") &lt; pos("a2")) passed
                </div>
              );
            default:
              return null;
          }
        })}
      </div>
    </div>
  );
}

export default function GraphExtras({
  sceneKey,
  frame,
}: {
  sceneKey: SceneKey;
  scene: Scene;
  frame: Frame;
}) {
  // Measure the content and animate the container's height so the island
  // card smoothly morphs as panels appear/disappear/grow between scenes.
  const innerRef = useRef<HTMLDivElement>(null);
  const [height, setHeight] = useState<number | null>(null);
  useIsomorphicLayoutEffect(() => {
    const el = innerRef.current;
    if (!el) return;
    const measure = () => setHeight(el.offsetHeight);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  return (
    <div
      className={styles.graphExtras}
      style={{ height: height == null ? "auto" : height }}
    >
      {/* overflow:hidden makes this a BFC so the panel's margins are
          included in the measured height. */}
      <div ref={innerRef} style={{ overflow: "hidden" }}>
        {sceneKey === "sim" && <SimLog frame={frame} />}
      </div>
    </div>
  );
}
