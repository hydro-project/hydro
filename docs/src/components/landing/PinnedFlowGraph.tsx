/**
 * PinnedFlowGraph: the sticky, morphing dataflow diagram for the landing page.
 *
 * This is a purpose-built SVG renderer (independent of the docs animation
 * framework). Scenes are declarative (see ./scenes.ts); when the scene
 * changes, elements are matched by ID:
 *   - persisting elements smoothly morph (position, size, color, dash) via
 *     CSS transitions (and animejs for line endpoints, which are not
 *     CSS-transitionable),
 *   - entering elements fade in,
 *   - exiting elements fade out and are then unmounted.
 *
 * No static layout is assumed: scenes may add/remove nodes and rearrange
 * everything.
 */

import React, { useEffect, useRef, useState } from "react";
import { animate } from "animejs";

import { COLOR_VARS, VIEWBOX } from "./scenes";
import type {
  BarSpec,
  ColorToken,
  EdgeSpec,
  GroupSpec,
  LabelSpec,
  OpSpec,
  PacketSpec,
  Scene,
  SwapPacketsSpec,
} from "./scenes";
import { runSwapLoop } from "./swap-clock";
import styles from "./landing.module.css";

const MORPH_MS = 650;
const MORPH_EASE = "cubic-bezier(0.4, 0, 0.2, 1)";

/**
 * Merge the current scene's elements with recently-removed ones so exits can
 * animate. Returns [{ item, exiting }].
 */
function useMergedElements<T extends { id: string }>(
  items: T[],
  exitMs: number = MORPH_MS,
): { item: T; exiting: boolean }[] {
  const [entries, setEntries] = useState(() =>
    items.map((item) => ({ item, exiting: false })),
  );

  useEffect(() => {
    setEntries((prev) => {
      const ids = new Set(items.map((i) => i.id));
      const next = items.map((item) => ({ item, exiting: false }));
      for (const entry of prev) {
        if (!ids.has(entry.item.id)) {
          next.push({ item: entry.item, exiting: true });
        }
      }
      return next;
    });

    const timer = setTimeout(() => {
      setEntries((prev) =>
        prev.some((e) => e.exiting) ? prev.filter((e) => !e.exiting) : prev,
      );
    }, exitMs);
    return () => clearTimeout(timer);
  }, [items, exitMs]);

  return entries;
}

const morphStyle = (extra: React.CSSProperties = {}): React.CSSProperties => ({
  transition: `all ${MORPH_MS}ms ${MORPH_EASE}`,
  ...extra,
});

// ---------------------------------------------------------------------------
// Element renderers
// ---------------------------------------------------------------------------

function Group({
  group,
  exiting,
  activeMember,
}: {
  group: GroupSpec;
  exiting: boolean;
  activeMember: number | null;
}) {
  const color = COLOR_VARS[group.color];
  const top = group.y - group.h / 2;

  // Cluster card stack lifecycle: when the group stops being a stack
  // (scrolling to a scene where this location is a single process), keep
  // the cards mounted briefly so they can retract behind the front card
  // instead of vanishing.
  const [stackPhase, setStackPhase] = useState<"in" | "out" | "none">(
    group.stack ? "in" : "none",
  );
  useEffect(() => {
    if (group.stack) setStackPhase("in");
    else setStackPhase((prev) => (prev === "in" ? "out" : prev));
  }, [group.stack]);
  useEffect(() => {
    if (stackPhase !== "out") return undefined;
    const timer = setTimeout(() => setStackPhase("none"), 600);
    return () => clearTimeout(timer);
  }, [stackPhase]);
  const showStack = stackPhase !== "none";

  // Remember the member colors so retracting cards keep their tint even
  // after the target scene drops `memberColors`.
  const lastMemberColors = useRef(group.memberColors);
  if (group.stack && group.memberColors) {
    lastMemberColors.current = group.memberColors;
  }
  const memberColors =
    (group.stack ? group.memberColors : lastMemberColors.current) || [];
  const cardColor = (i: number) => COLOR_VARS[memberColors[i]] || color;
  const cardActive = (i: number) => activeMember === i;
  // The member-highlight properties react quickly (in sync with the packet
  // starting to move), while geometry keeps the slower morph transition.
  // The resting filter is a zero-blur shadow so it interpolates smoothly
  // (`none` -> drop-shadow is a discrete jump).
  const highlightTransition = `all ${MORPH_MS}ms ${MORPH_EASE}, opacity 200ms ease, filter 200ms ease, stroke-width 200ms ease`;
  const cardGlow = (i: number) =>
    `drop-shadow(0 0 ${cardActive(i) ? 5 : 0}px ${cardColor(i)})`;
  return (
    <g
      className={styles.elementEnter}
      style={morphStyle({ opacity: exiting ? 0 : 1 })}
    >
      {showStack && (
        <g clipPath={`url(#lp-stack-clip-${group.id})`}>
          {/* Parallax card stack: the other cluster members peek out from
              beneath the top rectangle (clipped so only the part above the
              front card shows through its translucent fill). Each card is
              tinted with its member's color and lights up when that member
              is sending. */}
          <defs>
            <clipPath id={`lp-stack-clip-${group.id}`}>
              <rect x={0} y={0} width={VIEWBOX.width} height={top + 1} />
            </clipPath>
          </defs>
          <rect
            className={
              stackPhase === "out"
                ? styles.stackCardExitFar
                : styles.stackCardEnterFar
            }
            x={group.x - group.w / 2 + 26}
            y={top - 17}
            width={group.w - 52}
            height={40}
            rx={12}
            fill="var(--lp-island-bg)"
            stroke={cardColor(2)}
            strokeWidth={cardActive(2) ? 3 : 2}
            strokeDasharray={group.dashed ? "10 8" : undefined}
            style={{
              ...morphStyle({
                opacity: cardActive(2) ? 1 : 0.45,
                filter: cardGlow(2),
              }),
              transition: highlightTransition,
            }}
          />
          <rect
            className={
              stackPhase === "out"
                ? styles.stackCardExit
                : styles.stackCardEnter
            }
            x={group.x - group.w / 2 + 13}
            y={top - 9}
            width={group.w - 26}
            height={40}
            rx={14}
            fill="var(--lp-island-bg)"
            stroke={cardColor(1)}
            strokeWidth={cardActive(1) ? 3 : 2}
            strokeDasharray={group.dashed ? "10 8" : undefined}
            style={{
              ...morphStyle({
                opacity: cardActive(1) ? 1 : 0.6,
                filter: cardGlow(1),
              }),
              transition: highlightTransition,
            }}
          />
        </g>
      )}
      <defs>
        {/* Animated clip matching this group's rect, used to keep the
            code-bar placeholders inside the box during morphs. */}
        <clipPath id={`lp-group-clip-${group.id}`}>
          <rect
            style={morphStyle({
              x: group.x - group.w / 2,
              y: group.y - group.h / 2,
              width: group.w,
              height: group.h,
            })}
            rx={16}
          />
        </clipPath>
      </defs>
      <rect
        style={{
          ...morphStyle({
            x: group.x - group.w / 2,
            y: group.y - group.h / 2,
            width: group.w,
            height: group.h,
            stroke: color,
            strokeDasharray: group.dashed ? "10 8" : "10 0.001",
            fill: color,
            fillOpacity: 0.05,
            filter: group.stack ? cardGlow(0) : undefined,
          }),
          transition: highlightTransition,
        }}
        rx={16}
        strokeWidth={group.stack && cardActive(0) ? 3 : 2}
      />
      <text
        className={styles.groupLabel}
        style={morphStyle({
          transform: `translate(${group.x - group.w / 2 + 16}px, ${
            group.y - group.h / 2 + 26
          }px)`,
          fill: color,
          fontFamily: group.labelMono
            ? "var(--ifm-font-family-monospace)"
            : "var(--ifm-font-family-base)",
        })}
      >
        {group.labelBold && group.label.includes("<") ? (
          <>
            <tspan fontWeight={800}>{group.label.split("<")[0]}</tspan>
            <tspan>{"<" + group.label.split("<").slice(1).join("<")}</tspan>
          </>
        ) : (
          group.label
        )}
      </text>
      {group.sublabel && (
        <text
          className={styles.groupSubLabel}
          style={morphStyle({
            transform: `translate(${group.x - group.w / 2 + 16}px, ${
              group.y - group.h / 2 + 44
            }px)`,
          })}
        >
          {group.sublabel}
        </text>
      )}
      {group.badge && (
        <g
          style={morphStyle({
            transform: `translate(${group.x + group.w / 2 - 42}px, ${
              group.y - group.h / 2 + 18
            }px)`,
          })}
        >
          <rect
            x={-26}
            y={-11}
            width={52}
            height={22}
            rx={6}
            fill={COLOR_VARS.aws}
          />
          <text
            className={styles.badgeText}
            textAnchor="middle"
            dominantBaseline="central"
            fill="#1b1b1d"
          >
            {group.badge}
          </text>
        </g>
      )}
    </g>
  );
}

function Bar({ bar, exiting }: { bar: BarSpec; exiting: boolean }) {
  const rect = (
    <rect
      className={styles.elementEnter}
      style={morphStyle({
        x: bar.x,
        y: bar.y,
        width: bar.w,
        opacity: exiting ? 0 : 0.3,
      })}
      height={9}
      rx={4.5}
      fill={COLOR_VARS.grey}
    />
  );
  if (!bar.clipGroup) return rect;
  return <g clipPath={`url(#lp-group-clip-${bar.clipGroup})`}>{rect}</g>;
}

/**
 * Edge: an arrow between two points. Line endpoints are animated with
 * animejs (SVG attributes are not CSS-transitionable); everything else uses
 * CSS transitions. The endpoint attributes rendered by React are *frozen* at
 * their mount-time values (so SSR/hydration produce a correct initial
 * picture and React never fights the animation); animejs owns them
 * afterwards.
 */
function Edge({ edge, exiting }: { edge: EdgeSpec; exiting: boolean }) {
  const lineRef = useRef<SVGLineElement>(null);
  const initial = useRef(edge).current;
  const prev = useRef(initial);

  useEffect(() => {
    const el = lineRef.current;
    if (!el || edge.bend != null) return;
    const last = prev.current;
    if (
      last.x1 !== edge.x1 ||
      last.y1 !== edge.y1 ||
      last.x2 !== edge.x2 ||
      last.y2 !== edge.y2
    ) {
      animate(el, {
        x1: edge.x1,
        y1: edge.y1,
        x2: edge.x2,
        y2: edge.y2,
        duration: MORPH_MS,
        ease: "inOutQuad",
      });
    }
    prev.current = edge;
  }, [edge.x1, edge.y1, edge.x2, edge.y2]);

  const color = COLOR_VARS[edge.color];

  // Curved edges render as a static quadratic path (they fade in/out
  // rather than morphing); straight edges are animejs-animated lines.
  let curvePath: string | null = null;
  if (edge.bend != null) {
    const dx = edge.x2 - edge.x1;
    const dy = edge.y2 - edge.y1;
    const len = Math.hypot(dx, dy) || 1;
    const cx = (edge.x1 + edge.x2) / 2 + (dy / len) * edge.bend;
    const cy = (edge.y1 + edge.y2) / 2 - (dx / len) * edge.bend;
    curvePath = `M ${edge.x1} ${edge.y1} Q ${cx} ${cy} ${edge.x2} ${edge.y2}`;
  }

  return (
    <g
      className={styles.elementEnter}
      style={morphStyle({ opacity: exiting ? 0 : edge.opacity })}
    >
      {curvePath ? (
        <path
          d={curvePath}
          fill="none"
          style={morphStyle({
            stroke: color,
            strokeDasharray: edge.dashed ? "7 6" : undefined,
          })}
          strokeWidth={2.5}
          markerEnd={`url(#lp-arrow-${edge.color})`}
        />
      ) : (
        <line
          ref={lineRef}
          x1={initial.x1}
          y1={initial.y1}
          x2={initial.x2}
          y2={initial.y2}
          style={morphStyle({
            stroke: color,
            strokeDasharray: edge.dashed ? "7 6" : "7 0.001",
          })}
          strokeWidth={2.5}
          markerEnd={`url(#lp-arrow-${edge.color})`}
        />
      )}      {edge.label && (
        <text
          className={styles.edgeLabel}
          textAnchor="middle"
          style={morphStyle({
            transform: `translate(${edge.labelX}px, ${edge.labelY}px)`,
            fill: color,
          })}
        >
          {edge.label}
        </text>
      )}
    </g>
  );
}

function Op({
  op,
  exiting,
  flash,
  flashKey,
}: {
  op: OpSpec;
  exiting: boolean;
  flash: boolean;
  flashKey: number;
}) {
  const color = COLOR_VARS[op.color];
  return (
    <g
      className={styles.elementEnter}
      style={morphStyle({
        transform: `translate(${op.x}px, ${op.y}px)`,
        opacity: exiting ? 0 : 1,
      })}
    >
      {op.error && (
        <circle
          className={styles.errorPulse}
          r={17}
          fill="none"
          stroke={COLOR_VARS.error}
          strokeWidth={2.5}
        />
      )}
      {flash && (
        <circle
          key={flashKey}
          className={styles.opFlash}
          r={11}
          fill="none"
          stroke={COLOR_VARS.error}
          strokeWidth={3}
        />
      )}
      <circle
        r={10}
        fill="var(--lp-node-fill)"
        style={morphStyle({ stroke: color })}
        strokeWidth={2.5}
      />
      <text
        className={styles.opLabel}
        textAnchor="middle"
        y={op.labelPos === "top" ? -20 : 28}
      >
        {op.label}
      </text>
    </g>
  );
}

function FreeLabel({ label, exiting }: { label: LabelSpec; exiting: boolean }) {
  return (
    <text
      className={`${styles.freeLabel} ${styles.elementEnter} ${
        label.italic ? styles.freeLabelItalic : ""
      } ${label.mono ? styles.freeLabelMono : ""}`}
      textAnchor="middle"
      style={morphStyle({
        transform: `translate(${label.x}px, ${label.y}px)`,
        opacity: exiting ? 0 : 1,
      })}
    >
      {label.lines.map((line, i) => (
        <tspan key={i} x={0} dy={i === 0 ? 0 : 16}>
          {line}
        </tspan>
      ))}
    </text>
  );
}

/**
 * Packets are transient elements. A packet with `enterFrom` mounts at that
 * position and immediately glides to its target within the same step (so
 * e.g. a sim message animates *while* its send line is highlighted);
 * afterwards it transitions between step waypoints. `pill` packets are
 * rounded rectangles that fit a short message label.
 */
function Packet({ packet, stepMs }: { packet: PacketSpec; stepMs: number }) {
  const [entered, setEntered] = useState(!packet.enterFrom);
  useEffect(() => {
    if (entered) return undefined;
    // Double rAF: let the browser paint the spawn position first so the
    // move to the target transitions instead of snapping.
    let inner: number | undefined;
    const outer = requestAnimationFrame(() => {
      inner = requestAnimationFrame(() => setEntered(true));
    });
    return () => {
      cancelAnimationFrame(outer);
      if (inner !== undefined) cancelAnimationFrame(inner);
    };
  }, [entered]);

  // Guard against `enterFrom` disappearing on a later render while the
  // enter flip is still pending (rAF may be suspended in background tabs
  // while the step interval keeps running).
  const from = !entered && packet.enterFrom ? packet.enterFrom : null;
  const x = from ? from.x : packet.x;
  const y = from ? from.y : packet.y;
  return (
    <g
      className={styles.elementEnter}
      style={{
        transform: `translate(${x}px, ${y}px)`,
        transition: `transform ${stepMs}ms cubic-bezier(0.4, 0, 0.2, 1), opacity ${
          packet.opacity === 0 ? stepMs : 400
        }ms ease`,
        opacity: packet.opacity,
      }}
    >
      {packet.pill ? (
        <rect
          x={-15}
          y={-9}
          width={30}
          height={18}
          rx={9}
          fill={COLOR_VARS[packet.color]}
        />
      ) : (
        <circle r={8.5} fill={COLOR_VARS[packet.color]} />
      )}
      <text
        className={styles.packetLabel}
        textAnchor="middle"
        dominantBaseline="central"
      >
        {packet.label}
      </text>
    </g>
  );
}

/**
 * Two in-flight packets pinned next to an edge that continually swap
 * positions along opposite elliptical arcs, plus the fold's result string
 * rendered next to the result node with its letters swapping in sync.
 * Positions are computed per-frame from the shared swap clock (see
 * ./swap-clock.ts) and written directly to the DOM — smooth true-arc
 * motion, everything phase-locked.
 */
const SWAP_SLOT_DY = 32;
const SWAP_BULGE = 12;
const OUT_LETTER_DX = 11;
const OUT_LETTER_BULGE = 7;

function SwapPackets({ cfg }: { cfg: SwapPacketsSpec }) {
  const aRef = useRef<SVGGElement>(null);
  const bRef = useRef<SVGGElement>(null);
  const outARef = useRef<SVGGElement>(null);
  const outBRef = useRef<SVGGElement>(null);

  useEffect(
    () =>
      runSwapLoop(({ x, y }) => {
        const ax = x * SWAP_BULGE;
        const ay = y * SWAP_SLOT_DY;
        if (aRef.current) {
          aRef.current.setAttribute("transform", `translate(${ax}, ${ay})`);
        }
        if (bRef.current) {
          bRef.current.setAttribute(
            "transform",
            `translate(${-ax}, ${SWAP_SLOT_DY - ay})`,
          );
        }
        // Output letters: horizontal analogue of the same pose. The *bottom*
        // packet is processed first, so its letter takes the first slot:
        // at rest (a on top, b on bottom) the string reads "ba", and after
        // the swap it reads "ab".
        const lx = y * OUT_LETTER_DX;
        const ly = x * OUT_LETTER_BULGE;
        if (outARef.current) {
          outARef.current.setAttribute(
            "transform",
            `translate(${-lx}, ${ly})`,
          );
        }
        if (outBRef.current) {
          outBRef.current.setAttribute(
            "transform",
            `translate(${lx}, ${-ly})`,
          );
        }
      }),
    [],
  );

  const renderPacket = (
    spec: { label: string; color: ColorToken },
    ref: React.RefObject<SVGGElement | null>,
    initialDy: number,
  ) => (
    <g ref={ref} transform={`translate(0, ${initialDy})`}>
      <circle r={8.5} fill={COLOR_VARS[spec.color]} />
      <text
        className={styles.packetLabel}
        textAnchor="middle"
        dominantBaseline="central"
      >
        {spec.label}
      </text>
    </g>
  );

  return (
    <>
      <g
        className={styles.swapPacketsEnter}
        transform={`translate(${cfg.x}, ${cfg.yTop})`}
      >
        {renderPacket(cfg.a, aRef, 0)}
        {renderPacket(cfg.b, bRef, SWAP_SLOT_DY)}
      </g>
      {cfg.output && (
        <g
          className={styles.swapPacketsEnter}
          transform={`translate(${cfg.output.x}, ${cfg.output.y})`}
        >
          <text className={styles.swapOutQuote} x={0} dominantBaseline="central">
            "
          </text>
          <g ref={outARef}>
            <text
              className={styles.swapOutLetter}
              x={8 + OUT_LETTER_DX}
              dominantBaseline="central"
              fill={COLOR_VARS[cfg.a.color]}
            >
              {cfg.a.label}
            </text>
          </g>
          <g ref={outBRef}>
            <text
              className={styles.swapOutLetter}
              x={8}
              dominantBaseline="central"
              fill={COLOR_VARS[cfg.b.color]}
            >
              {cfg.b.label}
            </text>
          </g>
          <text
            className={styles.swapOutQuote}
            x={8 + 2 * OUT_LETTER_DX}
            dominantBaseline="central"
          >
            "
          </text>
        </g>
      )}
    </>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

function ArrowMarkers() {
  return (
    <defs>
      {Object.entries(COLOR_VARS).map(([token, color]) => (
        <marker
          key={token}
          id={`lp-arrow-${token}`}
          viewBox="0 0 10 10"
          refX="9"
          refY="5"
          markerWidth="7"
          markerHeight="7"
          orient="auto-start-reverse"
        >
          <path d="M 0 1 L 9 5 L 0 9 z" fill={color} />
        </marker>
      ))}
    </defs>
  );
}

export default function PinnedFlowGraph({
  scene,
  packets = [],
  stepMs = 900,
  flashOp = null,
  flashKey = 0,
  activeMember = null,
}: {
  scene: Scene;
  packets?: PacketSpec[];
  stepMs?: number;
  flashOp?: string | null;
  flashKey?: number;
  activeMember?: number | null;
}) {
  const groups = useMergedElements(scene.groups);
  const bars = useMergedElements(scene.bars);
  const edges = useMergedElements(scene.edges);
  const ops = useMergedElements(scene.ops);
  const labels = useMergedElements(scene.labels);

  return (
    <svg
      className={styles.graphSvg}
      viewBox={`0 0 ${VIEWBOX.width} ${VIEWBOX.height}`}
      role="img"
      aria-label="Diagram of a Hydro dataflow graph spanning multiple machines"
    >
      <ArrowMarkers />
      {groups.map(({ item, exiting }) => (
        <Group
          key={item.id}
          group={item}
          exiting={exiting}
          activeMember={item.stack ? activeMember : null}
        />
      ))}
      {bars.map(({ item, exiting }) => (
        <Bar key={item.id} bar={item} exiting={exiting} />
      ))}
      {edges.map(({ item, exiting }) => (
        <Edge key={item.id} edge={item} exiting={exiting} />
      ))}
      {ops.map(({ item, exiting }) => (
        <Op
          key={item.id}
          op={item}
          exiting={exiting}
          flash={flashOp === item.id}
          flashKey={flashKey}
        />
      ))}
      {labels.map(({ item, exiting }) => (
        <FreeLabel key={item.id} label={item} exiting={exiting} />
      ))}
      {scene.swapPackets && <SwapPackets cfg={scene.swapPackets} />}
      {packets.map((packet) => (
        <Packet key={packet.id} packet={packet} stepMs={stepMs} />
      ))}
    </svg>
  );
}
