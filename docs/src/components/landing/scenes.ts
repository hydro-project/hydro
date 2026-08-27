/**
 * Scene definitions and animation choreography for the landing-page pinned
 * flow graph.
 *
 * Each scene describes the diagram for one section of the scrollytelling
 * landing page. Elements carry stable IDs so that the graph can smoothly
 * morph between scenes: elements that share an ID move/restyle, elements
 * that appear/disappear fade in/out. Scenes may have entirely different
 * layouts and numbers of elements.
 *
 * The running example is a two-phase-commit protocol with one fixed
 * topology across all sections: a Cluster<Participant> (top) and a
 * Process<Leader> (bottom).
 *
 *  - global:      the prepare round — the leader broadcasts a transaction to
 *                 every participant, each writes its WAL and votes, and the
 *                 votes flow back, all in one function.
 *  - correctness: counting votes over a retrying channel is rejected at
 *                 compile time: a re-sent Yes would be counted twice — a
 *                 phantom quorum. No race here; the delivery *cardinality*
 *                 is the problem.
 *  - sim:         a buggy leader treats a majority (2 of 3) of votes as a
 *                 commit quorum; the simulator finds the vote arrival order
 *                 where both Yes votes win the race and the veto is cut off.
 *
 * Coordinate space: viewBox 0 0 440 420 (see VIEWBOX below).
 */

import { simFrame, simLastStep } from "./sim-script";
import type { SimScript } from "./sim-script";

export const VIEWBOX = { width: 440, height: 420 };

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type ColorToken =
  | "grey"
  | "client"
  | "server"
  | "network"
  | "chanB"
  | "pink"
  | "error"
  | "aws";

export interface GroupSpec {
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
  color: ColorToken;
  dashed: boolean;
  label: string;
  labelMono: boolean;
  /** Emphasize the location kind (the part before "<") in the label. */
  labelBold?: boolean;
  sublabel?: string;
  badge?: string;
  /** Render as a parallax card stack (a cluster). */
  stack?: boolean;
  /** Card colors per cluster member (front card first). */
  memberColors?: ColorToken[];
}

export interface BarSpec {
  id: string;
  x: number;
  y: number;
  w: number;
  /** Group whose (animated) rect clips this bar during transitions. */
  clipGroup?: string;
}

export interface OpSpec {
  id: string;
  x: number;
  y: number;
  color: ColorToken;
  label?: string;
  labelPos?: "top" | "bottom";
  error?: boolean;
}

export interface EdgeSpec {
  id: string;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  color: ColorToken;
  dashed: boolean;
  opacity: number;
  /**
   * Perpendicular offset of the control point: renders as a curved arrow.
   * Curved edges do not morph their geometry (they only fade in/out).
   */
  bend?: number;
  label?: string;
  labelX?: number;
  labelY?: number;
}

export interface LabelSpec {
  id: string;
  x: number;
  y: number;
  lines: string[];
  italic?: boolean;
  mono?: boolean;
}

/**
 * A "ghost" delivery: a message whose *cardinality* is non-deterministic.
 * The ghost pill slowly pulses between barely-there and fully delivered
 * (e.g. a retry that may or may not arrive), and the optional result
 * preview crossfades between two values in sync (e.g. `n = 1` ↔ `n = 2`).
 */
export interface GhostSpec {
  /** A solid delivery drawn above the ghost (optional anchor). */
  solid?: { x: number; y: number };
  /** The pulsing ghost delivery. */
  ghost: { x: number; y: number };
  label: string;
  color: ColorToken;
  /** Note rendered beside the ghost (e.g. "retry?"), pulsing with it. */
  ghostNote?: string;
  output?: {
    x: number;
    y: number;
    prefix: string;
    /** Value shown while the ghost is fully delivered. */
    solid: string;
    /** Value shown while the ghost is pale. */
    pale: string;
  };
}

export interface Scene {
  groups: GroupSpec[];
  bars: BarSpec[];
  ops: OpSpec[];
  edges: EdgeSpec[];
  labels: LabelSpec[];
  ghost?: GhostSpec;
}

export type SceneKey =
  | "intro"
  | "grpc"
  | "global"
  | "correctness"
  | "sim"
  | "cloud";

export interface PacketSpec {
  id: string;
  x: number;
  y: number;
  color: ColorToken;
  label: string;
  opacity: number;
  pill?: boolean;
  /** Mount position; the packet glides to (x, y) within the same step. */
  enterFrom?: { x: number; y: number };
}

/** A colored piece of a simulator-trace line. */
export interface LogPart {
  text: string;
  color?: ColorToken;
  bold?: boolean;
}

export type SimLogLine =
  | { type: "header"; key: string }
  | { type: "context"; key: string; text: string }
  | { type: "decision"; key: string; parts: LogPart[] }
  | { type: "ok"; key: string; text: string }
  | { type: "fail"; key: string; lines: string[] };

export interface Frame {
  packets: PacketSpec[];
  activeLines: number[];
  instanceNum?: number;
  totalInstances?: number;
  log?: SimLogLine[];
  flashOp?: string | null;
  flashLines?: number[];
  flashKey?: number;
  activeMember?: number | null;
  /** Code lines highlighted in "failing assertion" red. */
  failLines?: number[];
  /** The current sim instance has hit a failing assertion. */
  failed?: boolean;
}

// Color *tokens* are resolved to CSS variables so they adapt to dark mode.
export const COLOR_VARS: Record<ColorToken, string> = {
  grey: "var(--lp-grey)",
  client: "var(--lp-client)",
  server: "var(--lp-server)",
  network: "var(--lp-network)",
  chanB: "var(--lp-chanb)",
  pink: "var(--lp-pink)",
  error: "var(--lp-error)",
  aws: "var(--lp-aws)",
};

// ---------------------------------------------------------------------------
// Shared geometry
// ---------------------------------------------------------------------------

const GROUP_TOP = { x: 220, y: 109, w: 380, h: 138 };
const GROUP_BOTTOM = { x: 220, y: 327, w: 380, h: 138 };

const PARTICIPANTS_GROUP: GroupSpec = {
  id: "g-client",
  ...GROUP_TOP,
  color: "client",
  dashed: true,
  label: "Cluster<Participant>",
  labelMono: true,
  labelBold: true,
  stack: true,
  // Card colors for each member of the cluster: the front card is member 0,
  // the cards peeking out behind are members 1 and 2.
  memberColors: ["client", "chanB", "pink"],
};

const LEADER_GROUP: GroupSpec = {
  id: "g-server",
  ...GROUP_BOTTOM,
  color: "server",
  dashed: true,
  label: "Process<Leader>",
  labelMono: true,
};

// ---------------------------------------------------------------------------
// Scenes
// ---------------------------------------------------------------------------

export const SCENES: Record<SceneKey, Scene> = {
  /** Section: "Distributed systems deserve a native framework" */
  intro: {
    groups: [
      {
        id: "g-client",
        x: 220,
        y: 96,
        w: 150,
        h: 84,
        color: "grey",
        dashed: false,
        label: "",
        labelMono: false,
      },
      {
        id: "g-server",
        x: 110,
        y: 300,
        w: 150,
        h: 84,
        color: "grey",
        dashed: false,
        label: "",
        labelMono: false,
      },
      {
        id: "g-third",
        x: 330,
        y: 300,
        w: 150,
        h: 84,
        color: "grey",
        dashed: false,
        label: "",
        labelMono: false,
      },
    ],
    bars: [],
    ops: [],
    edges: [
      {
        id: "e-tri-1",
        x1: 272,
        y1: 143,
        x2: 348,
        y2: 253,
        color: "grey",
        dashed: false,
        opacity: 0.75,
        bend: 26,
      },
      {
        id: "e-tri-2",
        x1: 282,
        y1: 348,
        x2: 158,
        y2: 348,
        color: "grey",
        dashed: false,
        opacity: 0.75,
        bend: 26,
      },
      {
        id: "e-tri-3",
        x1: 92,
        y1: 253,
        x2: 168,
        y2: 143,
        color: "grey",
        dashed: false,
        opacity: 0.75,
        bend: 26,
      },
    ],
    labels: [],
  },

  /** Section: "Today's frameworks make networks implicit" */
  grpc: {
    groups: [
      {
        id: "g-client",
        ...GROUP_TOP,
        color: "grey",
        dashed: false,
        label: "node 1",
        labelMono: false,
      },
      {
        id: "g-server",
        ...GROUP_BOTTOM,
        color: "grey",
        dashed: false,
        label: "node 2",
        labelMono: false,
      },
    ],
    // Greyed-out "imperative code" placeholder bars inside each node.
    bars: [
      { id: "b-c1", x: 50, y: 87, w: 190, clipGroup: "g-client" },
      { id: "b-c2", x: 50, y: 107, w: 240, clipGroup: "g-client" },
      { id: "b-c3", x: 50, y: 127, w: 150, clipGroup: "g-client" },
      { id: "b-c4", x: 50, y: 147, w: 205, clipGroup: "g-client" },
      { id: "b-s1", x: 50, y: 306, w: 215, clipGroup: "g-server" },
      { id: "b-s2", x: 50, y: 326, w: 160, clipGroup: "g-server" },
      { id: "b-s3", x: 50, y: 346, w: 235, clipGroup: "g-server" },
      { id: "b-s4", x: 50, y: 366, w: 180, clipGroup: "g-server" },
    ],
    ops: [],
    edges: [
      {
        id: "e-req",
        x1: 180,
        y1: 182,
        x2: 180,
        y2: 251,
        color: "grey",
        dashed: true,
        opacity: 0.5,
      },
      {
        id: "e-resp",
        x1: 260,
        y1: 254,
        x2: 260,
        y2: 185,
        color: "grey",
        dashed: true,
        opacity: 0.5,
      },
    ],
    labels: [
      {
        id: "l-implicit",
        x: 322,
        y: 212,
        lines: ["implicit", "network"],
        italic: true,
      },
    ],
  },

  /** Section: "Hydro is global" — the prepare round of 2PC. */
  global: {
    groups: [PARTICIPANTS_GROUP, LEADER_GROUP],
    bars: [],
    ops: [
      { id: "op-prep", x: 180, y: 126, color: "client" },
      { id: "op-src", x: 260, y: 126, color: "client" },
      { id: "op-txn", x: 180, y: 330, color: "server", label: "txns" },
      { id: "op-agg", x: 260, y: 330, color: "server", label: "votes" },
    ],
    edges: [
      {
        id: "e-req",
        x1: 180,
        y1: 314,
        x2: 180,
        y2: 140,
        color: "network",
        dashed: false,
        opacity: 1,
        label: "broadcast",
        labelX: 138,
        labelY: 228,
      },
      {
        id: "e-map",
        x1: 194,
        y1: 126,
        x2: 246,
        y2: 126,
        color: "client",
        dashed: false,
        opacity: 1,
        label: "map(prepare)",
        labelX: 220,
        labelY: 100,
      },
      // The votes pipeline (op-src → e-resp → op-agg) persists into the
      // correctness and sim scenes: it slides left as one unit.
      {
        id: "e-resp",
        x1: 260,
        y1: 140,
        x2: 260,
        y2: 314,
        color: "network",
        dashed: false,
        opacity: 1,
        label: "TCP",
        labelX: 284,
        labelY: 228,
      },
    ],
    labels: [],
  },

  /**
   * Section: "Hydro catches distributed bugs at compile time" — counting
   * votes on an at-least-once channel is rejected.
   */
  correctness: {
    groups: [PARTICIPANTS_GROUP, LEADER_GROUP],
    bars: [],
    ops: [
      { id: "op-src", x: 180, y: 126, color: "client", label: "votes", labelPos: "top" },
      { id: "op-agg", x: 180, y: 330, color: "server" },
      { id: "op-out", x: 260, y: 330, color: "error", error: true },
    ],
    edges: [
      {
        id: "e-resp",
        x1: 180,
        y1: 140,
        x2: 180,
        y2: 314,
        color: "network",
        dashed: false,
        opacity: 1,
        label: "TCP",
        labelX: 156,
        labelY: 228,
      },
      {
        id: "e-fold",
        x1: 194,
        y1: 330,
        x2: 246,
        y2: 330,
        color: "error",
        dashed: false,
        opacity: 1,
        label: "fold(+1)",
        labelX: 220,
        labelY: 358,
      },
    ],
    labels: [
      {
        id: "l-noorder",
        x: 320,
        y: 210,
        lines: ["one vote,", "delivered twice?"],
        italic: true,
      },
    ],
    // A vote and its retry: the second copy pulses between barely-there and
    // fully delivered, and the tally next to the result node flickers
    // between 1 and 2 in sync — the count is not well-defined.
    ghost: {
      solid: { x: 204, y: 196 },
      ghost: { x: 204, y: 230 },
      label: "Yes",
      color: "client",
      ghostNote: "retry?",
      output: { x: 286, y: 330, prefix: "n = ", solid: "2", pale: "1" },
    },
  },

  /**
   * Section: "Hydro lets you write distributed tests" — three votes race to
   * a leader with a majority-quorum bug.
   */
  sim: {
    groups: [PARTICIPANTS_GROUP, LEADER_GROUP],
    bars: [],
    ops: [
      {
        id: "op-src",
        x: 220,
        y: 126,
        color: "client",
        label: "votes",
        labelPos: "top",
      },
      {
        id: "op-agg",
        x: 220,
        y: 330,
        color: "server",
        label: "limit(2) “quorum”",
      },
    ],
    edges: [
      {
        id: "e-resp",
        x1: 220,
        y1: 140,
        x2: 220,
        y2: 314,
        color: "network",
        dashed: false,
        opacity: 1,
      },
    ],
    labels: [],
  },

  /** Section: "Native cloud infrastructure" (section currently disabled) */
  cloud: {
    groups: [
      {
        ...PARTICIPANTS_GROUP,
        sublabel: "us-east-1",
        badge: "ECS",
      },
      {
        ...LEADER_GROUP,
        sublabel: "us-west-2",
        badge: "ECS",
      },
    ],
    bars: [],
    ops: [
      { id: "op-prep", x: 180, y: 126, color: "client" },
      { id: "op-src", x: 260, y: 126, color: "client" },
      { id: "op-txn", x: 180, y: 330, color: "server", label: "txns" },
      { id: "op-agg", x: 260, y: 330, color: "server", label: "votes" },
    ],
    edges: [
      {
        id: "e-req",
        x1: 180,
        y1: 314,
        x2: 180,
        y2: 140,
        color: "network",
        dashed: false,
        opacity: 1,
      },
      {
        id: "e-map",
        x1: 194,
        y1: 126,
        x2: 246,
        y2: 126,
        color: "client",
        dashed: false,
        opacity: 1,
        label: "map(prepare)",
        labelX: 220,
        labelY: 100,
      },
      {
        id: "e-resp",
        x1: 260,
        y1: 140,
        x2: 260,
        y2: 314,
        color: "network",
        dashed: false,
        opacity: 1,
      },
    ],
    labels: [],
  },
};

// ---------------------------------------------------------------------------
// Sim choreography
// ---------------------------------------------------------------------------

/**
 * Three participants vote on a transaction (No from member 0, Yes from
 * members 1 and 2) and a buggy leader decides on the first two votes to
 * arrive — a majority, not unanimity. The simulator explores the three
 * distinct arrival orders; in the last one both Yes votes win the race, the
 * assertion panics, and the veto is still sitting in the queue.
 */
const VOTE_SPECS = {
  n0: { member: 0, color: "client" as const, text: "No" },
  y1: { member: 1, color: "chanB" as const, text: "Yes" },
  y2: { member: 2, color: "pink" as const, text: "Yes" },
};

const arrival = (id: keyof typeof VOTE_SPECS) => {
  const spec = VOTE_SPECS[id];
  return {
    msgs: [id as string],
    decision: [
      { text: "     ^ releasing items: [(" },
      { text: `MemberId(${spec.member})`, color: spec.color, bold: true },
      { text: `, ${spec.text})]` },
    ],
  };
};

const OK_TEXT = "✓ assert!(seen.contains(&No)) passed";

const SIM_SCRIPT: SimScript = {
  msgs: {
    n0: { label: "No", color: "client", member: 0, spawn: { x: 220, y: 146 }, sendLine: 12 },
    y1: { label: "Yes", color: "chanB", member: 1, spawn: { x: 220, y: 146 }, sendLine: 13 },
    y2: { label: "Yes", color: "pink", member: 2, spawn: { x: 220, y: 146 }, sendLine: 14 },
  },
  sendOrder: ["n0", "y1", "y2"],
  queue: { x: 220, baseY: 292, spacing: 22 },
  releasePos: { x: 220, y: 352 },
  releaseOpId: "op-agg",
  contextText: "--> .entries_partially_ordered(nondet!(…))",
  flashLines: [5, 6],
  collectLines: [16],
  assertLines: [17],
  instances: [
    // `collect()` returns as soon as `limit(2)` completes the stream, so
    // each instance releases exactly two votes; the third is never even
    // looked at (it stays in the queue).
    {
      // The No arrives within the first two votes: the veto is seen.
      batches: [arrival("n0"), arrival("y1")],
      outcome: { type: "ok", text: OK_TEXT },
    },
    {
      batches: [arrival("y1"), arrival("n0")],
      outcome: { type: "ok", text: OK_TEXT },
    },
    {
      // Both Yes votes win the race: `collect()` returns [Yes, Yes] and
      // the assertion panics on the spot — the veto still in the queue.
      batches: [arrival("y1"), arrival("y2")],
      outcome: {
        type: "fail",
        lines: [
          "✗ panicked: assertion failed: seen.contains(&Vote::No)",
        ],
      },
    },
  ],
};

/** Last animation step of the sim scene; the animation pauses here. */
export const SIM_LAST_STEP = simLastStep(SIM_SCRIPT);

/** Compute the animation frame for the active scene at a given step. */
export function computeFrame(sceneKey: SceneKey, step: number): Frame {
  if (sceneKey === "sim") return simFrame(SIM_SCRIPT, step);
  return { packets: [], activeLines: [] };
}
