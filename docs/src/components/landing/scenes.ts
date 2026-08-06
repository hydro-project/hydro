/**
 * Scene definitions for the landing-page pinned flow graph.
 *
 * Each scene describes the diagram for one section of the scrollytelling
 * landing page. Elements carry stable IDs so that the graph can smoothly
 * morph between scenes: elements that share an ID move/restyle, elements
 * that appear/disappear fade in/out. Scenes may have entirely different
 * layouts and numbers of elements.
 *
 * Coordinate space: viewBox 0 0 440 420 (see VIEWBOX below).
 */

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

export interface SwapPacketsSpec {
  x: number;
  yTop: number;
  a: { label: string; color: ColorToken };
  b: { label: string; color: ColorToken };
  /** Anchor of the fold's result string, rendered next to the result node. */
  output?: { x: number; y: number };
}

export interface Scene {
  groups: GroupSpec[];
  bars: BarSpec[];
  ops: OpSpec[];
  edges: EdgeSpec[];
  labels: LabelSpec[];
  swapPackets?: SwapPacketsSpec;
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

export type SimLogLine =
  | { type: "header" | "context" | "ok"; key: string }
  | { type: "decision"; key: string; member: number; msg: string };

export interface Frame {
  packets: PacketSpec[];
  activeLines: number[];
  instanceNum?: number;
  log?: SimLogLine[];
  flashOp?: string | null;
  flashLines?: number[];
  flashKey?: number;
  activeMember?: number | null;
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

const GROUP_CLIENT = { x: 220, y: 109, w: 380, h: 138 };
const GROUP_SERVER = { x: 220, y: 327, w: 380, h: 138 };

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
        ...GROUP_CLIENT,
        color: "grey",
        dashed: false,
        label: "node 1",
        labelMono: false,
      },
      {
        id: "g-server",
        ...GROUP_SERVER,
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

  /** Section: "Hydro is global" */
  global: {
    groups: [
      {
        id: "g-client",
        ...GROUP_CLIENT,
        color: "client",
        dashed: true,
        label: "Process<Client>",
        labelMono: true,
      },
      {
        id: "g-server",
        ...GROUP_SERVER,
        color: "server",
        dashed: true,
        label: "Process<Server>",
        labelMono: true,
      },
    ],
    bars: [],
    ops: [
      { id: "op-src", x: 180, y: 126, color: "client", label: "requests", labelPos: "top" },
      { id: "op-sink", x: 260, y: 126, color: "client", label: "responses", labelPos: "top" },
      { id: "op-agg", x: 180, y: 330, color: "server" },
      { id: "op-out", x: 260, y: 330, color: "server" },
    ],
    edges: [
      {
        id: "e-req",
        x1: 180,
        y1: 140,
        x2: 180,
        y2: 314,
        color: "network",
        dashed: false,
        opacity: 1,
        label: "TCP",
        labelX: 156,
        labelY: 218,
      },
      {
        id: "e-map",
        x1: 194,
        y1: 330,
        x2: 246,
        y2: 330,
        color: "server",
        dashed: false,
        opacity: 1,
        label: "map(to_uppercase)",
        labelX: 220,
        labelY: 358,
      },
      {
        id: "e-resp",
        x1: 260,
        y1: 314,
        x2: 260,
        y2: 140,
        color: "network",
        dashed: false,
        opacity: 1,
        label: "TCP",
        labelX: 284,
        labelY: 218,
      },
    ],
    labels: [],
  },

  /** Section: "Compile-time distributed correctness" */
  correctness: {
    groups: [
      {
        id: "g-client",
        ...GROUP_CLIENT,
        color: "client",
        dashed: true,
        label: "Cluster<Client>",
        labelMono: true,
        labelBold: true,
        stack: true,
        memberColors: ["client", "chanB", "pink"],
      },
      {
        id: "g-server",
        ...GROUP_SERVER,
        color: "server",
        dashed: true,
        label: "Process<Server>",
        labelMono: true,
      },
    ],
    bars: [],
    ops: [
      { id: "op-src", x: 180, y: 126, color: "client", label: "words", labelPos: "top" },
      { id: "op-agg", x: 180, y: 330, color: "server" },
      {
        id: "op-out",
        x: 260,
        y: 330,
        color: "error",
        error: true,
      },
    ],
    edges: [
      {
        id: "e-req",
        x1: 180,
        y1: 140,
        x2: 180,
        y2: 314,
        color: "network",
        dashed: false,
        opacity: 1,
      },
      {
        id: "e-map",
        x1: 194,
        y1: 330,
        x2: 246,
        y2: 330,
        color: "error",
        dashed: false,
        opacity: 1,
        label: "fold(concat)",
        labelX: 220,
        labelY: 358,
      },
    ],
    labels: [
      {
        id: "l-noorder",
        x: 318,
        y: 210,
        lines: ["messages", "interleave"],
        italic: true,
      },
    ],
    // Two in-flight packets pinned on the UDP edge that continually swap
    // positions (rendered with a looping arc animation, see
    // PinnedFlowGraph), plus the fold's result string shown next to the
    // result node, its letters swapping in sync.
    swapPackets: {
      x: 202,
      yTop: 202,
      a: { label: "a", color: "client" },
      b: { label: "b", color: "chanB" },
      output: { x: 288, y: 330 },
    },
  },

  /** Section: "Hydro lets you write distributed tests" */
  sim: {
    groups: [
      {
        id: "g-client",
        ...GROUP_CLIENT,
        color: "client",
        dashed: true,
        label: "Cluster<Client>",
        labelMono: true,
        stack: true,
        // Card colors for each member of the cluster: the front card is
        // member 0, the cards peeking out behind are members 1 and 2.
        memberColors: ["client", "chanB", "pink"],
      },
      {
        id: "g-server",
        ...GROUP_SERVER,
        color: "server",
        dashed: true,
        label: "Process<Server>",
        labelMono: true,
      },
    ],
    bars: [],
    ops: [
      {
        id: "op-src",
        x: 220,
        y: 126,
        color: "client",
        label: "events",
        labelPos: "top",
      },
      {
        id: "op-agg",
        x: 220,
        y: 330,
        color: "server",
        label: "entries_partially_ordered",
      },
    ],
    edges: [
      {
        id: "e-req",
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

  /** Section: "Native cloud infrastructure" */
  cloud: {
    groups: [
      {
        id: "g-client",
        ...GROUP_CLIENT,
        color: "client",
        dashed: true,
        label: "Process<Client>",
        sublabel: "us-east-1",
        labelMono: true,
        badge: "ECS",
      },
      {
        id: "g-server",
        ...GROUP_SERVER,
        color: "server",
        dashed: true,
        label: "Process<Server>",
        sublabel: "us-west-2",
        labelMono: true,
        badge: "ECS",
      },
    ],
    bars: [],
    ops: [
      { id: "op-src", x: 180, y: 126, color: "client", label: "requests", labelPos: "top" },
      { id: "op-sink", x: 260, y: 126, color: "client", label: "responses", labelPos: "top" },
      { id: "op-agg", x: 180, y: 330, color: "server" },
      { id: "op-out", x: 260, y: 330, color: "server" },
    ],
    edges: [
      {
        id: "e-req",
        x1: 180,
        y1: 140,
        x2: 180,
        y2: 314,
        color: "network",
        dashed: false,
        opacity: 1,
      },
      {
        id: "e-map",
        x1: 194,
        y1: 330,
        x2: 246,
        y2: 330,
        color: "server",
        dashed: false,
        opacity: 1,
        label: "map(to_uppercase)",
        labelX: 220,
        labelY: 358,
      },
      {
        id: "e-resp",
        x1: 260,
        y1: 314,
        x2: 260,
        y2: 140,
        color: "network",
        dashed: false,
        opacity: 1,
      },
    ],
    labels: [],
  },
};

// ---------------------------------------------------------------------------
// Animation frames (step-driven choreography)
// ---------------------------------------------------------------------------

/**
 * Sim scene: a cluster of clients emits events ("a1"/"a2" from member 0,
 * "b1" from member 1) into an ordered event log on the server. Each send is
 * animated one at a time (with the sending member's card highlighted in the
 * stack); the messages accumulate at the input of
 * `entries_partially_ordered`, and the simulator then makes a decision at
 * the `nondet!` point for each release: the operator and the nondet code
 * line flash, one message passes through, and a trace line is appended.
 *
 * The real test for this example lives at
 * `hydro_test/src/distributed/event_log.rs`; the simulator explores exactly
 * 3 interleavings (all arrival orders of [a1, a2] × [b1] that preserve
 * per-member prefix order), which is what we replay here.
 *
 * An instance is 9 steps: three sends, accumulate, three releases, assert,
 * pause.
 */
const SIM_ORDERS = [
  ["a1", "a2", "b1"],
  ["a1", "b1", "a2"],
  ["b1", "a1", "a2"],
];
const SIM_MSGS: Record<string, { member: number; color: ColorToken; sendPhase: number; sendLine: number }> = {
  a1: { member: 0, color: "client", sendPhase: 0, sendLine: 9 },
  a2: { member: 0, color: "client", sendPhase: 1, sendLine: 10 },
  b1: { member: 1, color: "chanB", sendPhase: 2, sendLine: 11 },
};
const SIM_MSG_PRIORITY = ["a1", "a2", "b1"];
const SIM_SPAWN = { x: 220, y: 146 };
const SIM_QUEUE_X = 220;
const SIM_QUEUE_BASE_Y = 292;
const SIM_QUEUE_SPACING = 22;
const SIM_RELEASE_POS = { x: 220, y: 352 };
const SIM_NONDET_LINE = 5;
export const SIM_TOTAL_INSTANCES = 3;
/** Steps per instance x instances; the sim animation runs once and stops here. */
export const SIM_LAST_STEP = SIM_TOTAL_INSTANCES * 9 - 1;

export function simFrame(step: number): Frame {
  const per = 9;
  const inst = Math.floor(step / per) % SIM_TOTAL_INSTANCES;
  const phase = step % per;
  const order = SIM_ORDERS[inst];
  const instanceNum = inst + 1;

  // How many releases have happened so far (releases occur at phases 4..6).
  const released = Math.max(0, Math.min(3, phase - 3));
  const releasedSet = new Set(order.slice(0, released));
  const waiting = SIM_MSG_PRIORITY.filter((m) => !releasedSet.has(m));

  const packets: PacketSpec[] = [];
  for (const msg of SIM_MSG_PRIORITY) {
    const { color, sendPhase } = SIM_MSGS[msg];
    if (phase < sendPhase) continue; // not sent yet
    const justReleased = released > 0 && order[released - 1] === msg;
    if (phase === sendPhase) {
      // Sent this step: spawns at the cluster's `events` operator and
      // travels to its queue slot *while* its send line is highlighted.
      const slot = waiting.indexOf(msg);
      packets.push({
        id: `pkt-${msg}-${inst}`,
        x: SIM_QUEUE_X,
        y: SIM_QUEUE_BASE_Y - slot * SIM_QUEUE_SPACING,
        enterFrom: { x: SIM_SPAWN.x, y: SIM_SPAWN.y },
        color,
        label: msg,
        pill: true,
        opacity: 1,
      });
    } else if (justReleased) {
      // Chosen by the simulator: passes through the operator and fades.
      packets.push({
        id: `pkt-${msg}-${inst}`,
        x: SIM_RELEASE_POS.x,
        y: SIM_RELEASE_POS.y,
        color,
        label: msg,
        pill: true,
        opacity: 0,
      });
    } else if (waiting.includes(msg)) {
      // Accumulated at the operator's input, in arrival order.
      const slot = waiting.indexOf(msg);
      packets.push({
        id: `pkt-${msg}-${inst}`,
        x: SIM_QUEUE_X,
        y: SIM_QUEUE_BASE_Y - slot * SIM_QUEUE_SPACING,
        color,
        label: msg,
        pill: true,
        opacity: 1,
      });
    }
  }

  // Sim log in the real `HYDRO_SIM_LOG=1` trace format. The log accumulates
  // across all instances of the current exploration cycle (resetting when
  // instance 1 starts over), with a "New Simulation Instance" header between
  // instances. The nondet context line appears together with the first
  // released element of each instance.
  const log: SimLogLine[] = [];
  for (let i = 0; i <= inst; i++) {
    const isCurrent = i === inst;
    const instOrder = SIM_ORDERS[i];
    const rel = isCurrent ? released : 3;
    log.push({ type: "header", key: `h${i}` });
    if (rel > 0) {
      log.push({ type: "context", key: `c${i}` });
      for (let k = 0; k < rel; k++) {
        const msg = instOrder[k];
        log.push({
          type: "decision",
          key: `d${i}-${k}`,
          member: SIM_MSGS[msg].member,
          msg,
        });
      }
    }
    if (!isCurrent || phase >= 7) {
      log.push({ type: "ok", key: `ok${i}` });
    }
  }

  // Code line highlights stay in the test body; the nondet line in the
  // dataflow definition *flashes* whenever the simulator makes a decision.
  let activeLines: number[] = [];
  let activeMember: number | null = null;
  if (phase <= 2) {
    const sending = SIM_MSG_PRIORITY.find(
      (m) => SIM_MSGS[m].sendPhase === phase,
    );
    if (sending) {
      activeLines = [SIM_MSGS[sending].sendLine];
      activeMember = SIM_MSGS[sending].member;
    }
  } else if (phase >= 3 && phase <= 6) activeLines = [13];
  else if (phase === 7) activeLines = [14];

  const deciding = phase >= 4 && phase <= 6;
  return {
    packets,
    instanceNum,
    log,
    // The operator and the `nondet!` code line flash on release decisions.
    flashOp: deciding ? "op-agg" : null,
    flashLines: deciding ? [SIM_NONDET_LINE] : [],
    flashKey: step,
    activeMember,
    activeLines,
  };
}

/** Compute the animation frame for the active scene at a given step. */
export function computeFrame(sceneKey: SceneKey, step: number): Frame {
  if (sceneKey === "sim") return simFrame(step);
  return { packets: [], activeLines: [] };
}
