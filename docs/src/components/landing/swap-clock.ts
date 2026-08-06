/**
 * Shared clock + parametric pose for the "swap" animation used by the
 * correctness scene (in-flight packets swapping on the UDP edge, and the
 * output-preview letters swapping in sync).
 *
 * The animation is driven by wall-clock time (performance.now()), so any
 * number of independent rAF loops (SVG packets, HTML letters) derive the
 * exact same phase and stay perfectly synchronized.
 *
 * The cycle: hold → swap along an elliptical arc → hold (swapped) → swap
 * back along the mirrored arc. During a swap the pose traces a true
 * half-ellipse (sin/cos), with smoothstep easing on the sweep so motion
 * starts and ends with zero velocity — no corners, no blockiness.
 *
 * `swapPose(now)` returns the *normalized* offset of the first element:
 *   y ∈ [0, 1] — progress from its own slot (0) to the other slot (1)
 *   x ∈ [0, 1] — lateral bulge; the swap-back retraces the same arc
 * The second element is the point reflection: (-x, 1 - y) relative to its
 * own slot.
 */

const PERIOD_MS = 4400;
const HOLD1_END = 0.3;
const SWAP1_END = 0.5;
const HOLD2_END = 0.8;

function smoothstep(t: number): number {
  return t * t * (3 - 2 * t);
}

export interface SwapPose {
  x: number;
  y: number;
}

export function swapPose(nowMs: number): SwapPose {
  const t = (nowMs % PERIOD_MS) / PERIOD_MS;
  if (t < HOLD1_END) {
    return { x: 0, y: 0 };
  }
  if (t < SWAP1_END) {
    const p = smoothstep((t - HOLD1_END) / (SWAP1_END - HOLD1_END));
    const theta = Math.PI * p;
    return { x: Math.sin(theta), y: (1 - Math.cos(theta)) / 2 };
  }
  if (t < HOLD2_END) {
    return { x: 0, y: 1 };
  }
  const p = smoothstep((t - HOLD2_END) / (1 - HOLD2_END));
  const theta = Math.PI * p;
  // Swap back along the *same* arc (retracing the path), rather than
  // continuing around clockwise.
  return { x: Math.sin(theta), y: (1 + Math.cos(theta)) / 2 };
}

/**
 * Run `onFrame(pose)` every animation frame with the current shared pose.
 * Honors prefers-reduced-motion by rendering the resting pose once.
 * Returns a cleanup function.
 */
export function runSwapLoop(onFrame: (pose: SwapPose) => void): () => void {
  if (
    typeof window !== "undefined" &&
    window.matchMedia &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  ) {
    onFrame({ x: 0, y: 0 });
    return () => {};
  }
  let handle: number;
  const tick = () => {
    onFrame(swapPose(performance.now()));
    handle = requestAnimationFrame(tick);
  };
  handle = requestAnimationFrame(tick);
  return () => cancelAnimationFrame(handle);
}
