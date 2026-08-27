/**
 * A small declarative "script" format for the landing page's sim-scene
 * choreography (see ./scenes.ts for the script itself).
 *
 * A script describes messages flowing from source operators into a queue at
 * a nondet decision point, and a list of simulation instances, each with its
 * own release schedule (batches) and outcome (a passing or failing
 * assertion). Each instance plays out in phases:
 *
 *   [send × N] [gather] [release × batches] [assert] [hold]
 *
 * Sends animate one message per step from its spawn point into the queue
 * (highlighting the corresponding code line); releases pass one batch per
 * step through the decision operator (flashing it and appending trace
 * lines); the assert phase shows the instance's outcome (✓ or ✗). The trace
 * log accumulates across instances, exactly like a `HYDRO_SIM_LOG=1` trace.
 */

import type {
  ColorToken,
  Frame,
  LogPart,
  PacketSpec,
  SimLogLine,
} from "./scenes";

export interface SimMsgSpec {
  /** Text shown on the packet pill. */
  label: string;
  color: ColorToken;
  /** Cluster member that sends this message (highlights its stack card). */
  member: number | null;
  /** Where the packet spawns (usually the source operator). */
  spawn: { x: number; y: number };
  /** 1-based code line highlighted while this message is sent. */
  sendLine: number;
  /**
   * Column (x) where this message queues and is released; defaults to the
   * script-wide queue/release x. Lets different inputs stack above
   * different operators (e.g. puts above the store, gets above the slice).
   */
  queueX?: number;
}

export interface SimBatchSpec {
  /** Message ids released by this decision, in display order. */
  msgs: string[];
  /** Trace line for this decision. */
  decision: LogPart[];
  /** Extra trace lines appended right after the decision. */
  extra?: LogPart[][];
  /** Override the code lines flashed for this decision. */
  flashLines?: number[];
  /** Override the op flashed for this decision (`null` = no flash). */
  flashOp?: string | null;
  /** Override the code lines highlighted while this batch is processed. */
  activeLines?: number[];
}

export interface SimInstanceSpec {
  batches: SimBatchSpec[];
  outcome:
    | { type: "ok"; text: string }
    | { type: "fail"; lines: string[] };
}

export interface SimScript {
  msgs: Record<string, SimMsgSpec>;
  /** Order in which messages are sent (one per send phase). */
  sendOrder: string[];
  /** Where waiting messages accumulate (stacked upwards from baseY). */
  queue: { x: number; baseY: number; spacing: number };
  /** Where released messages fade out (the decision operator). */
  releasePos: { x: number; y: number };
  /** Op id flashed on each release decision. */
  releaseOpId: string;
  /** The `--> ...` nondet context line in the trace. */
  contextText: string;
  /** Code lines flashed on each release decision (the nondet point). */
  flashLines: number[];
  /** Code lines highlighted while the simulator gathers/releases. */
  collectLines: number[];
  /** Code lines highlighted (or failed) during the assert phase. */
  assertLines: number[];
  instances: SimInstanceSpec[];
}

function instSteps(script: SimScript, instance: SimInstanceSpec): number {
  // sends + gather + releases + assert + hold
  return script.sendOrder.length + instance.batches.length + 3;
}

/** Total steps across all instances; the animation pauses at the last one. */
export function simLastStep(script: SimScript): number {
  return (
    script.instances.reduce((acc, inst) => acc + instSteps(script, inst), 0) -
    1
  );
}

export function simFrame(script: SimScript, step: number): Frame {
  // Locate the current instance and the phase within it.
  let inst = 0;
  let rem = Math.max(0, step);
  while (
    inst < script.instances.length - 1 &&
    rem >= instSteps(script, script.instances[inst])
  ) {
    rem -= instSteps(script, script.instances[inst]);
    inst += 1;
  }
  const instance = script.instances[inst];
  const nSend = script.sendOrder.length;
  const nBatch = instance.batches.length;
  const phase = Math.min(rem, instSteps(script, instance) - 1);
  const firstRelease = nSend + 1;
  const assertPhase = nSend + nBatch + 1;

  const releasedBatches = Math.max(0, Math.min(nBatch, phase - nSend));
  const releasedMsgs = new Set(
    instance.batches.slice(0, releasedBatches).flatMap((b) => b.msgs),
  );
  const releasingNow =
    phase >= firstRelease && phase <= nSend + nBatch
      ? instance.batches[phase - firstRelease]
      : null;

  const packets: PacketSpec[] = [];
  const sentIds = script.sendOrder.filter((_, i) => phase >= i);
  const waiting = sentIds.filter((id) => !releasedMsgs.has(id));
  const colX = (id: string) => script.msgs[id].queueX ?? script.queue.x;
  // Messages still queued when the instance ends (e.g. votes the test never
  // looked at) dissolve during the final hold phase, so the reset to the
  // next instance doesn't pop them out abruptly. The last instance keeps
  // them visible — a stranded message is part of the story.
  const dissolving =
    phase === instSteps(script, instance) - 1 &&
    inst < script.instances.length - 1;
  for (const id of sentIds) {
    const msg = script.msgs[id];
    if (releasedMsgs.has(id)) {
      // Released this step: passes through the operator and fades. (Earlier
      // releases are fully gone.)
      if (releasingNow && releasingNow.msgs.includes(id)) {
        const k = releasingNow.msgs.indexOf(id);
        const spread = (k - (releasingNow.msgs.length - 1) / 2) * 36;
        packets.push({
          id: `pkt-${id}-${inst}`,
          x: colX(id) + spread,
          y: script.releasePos.y,
          color: msg.color,
          label: msg.label,
          pill: true,
          opacity: 0,
        });
      }
      continue;
    }
    // Waiting in the queue (or gliding into it, if sent this step). Slots
    // stack per column, so inputs queued above different operators don't
    // overlap.
    const slot = waiting.filter((w) => colX(w) === colX(id)).indexOf(id);
    packets.push({
      id: `pkt-${id}-${inst}`,
      x: colX(id),
      y: script.queue.baseY - slot * script.queue.spacing,
      color: msg.color,
      label: msg.label,
      pill: true,
      opacity: dissolving ? 0 : 1,
      ...(phase === script.sendOrder.indexOf(id)
        ? { enterFrom: msg.spawn }
        : {}),
    });
  }

  // Trace log, accumulated across all instances of the current cycle.
  const log: SimLogLine[] = [];
  for (let i = 0; i <= inst; i++) {
    const isCurrent = i === inst;
    const inst_i = script.instances[i];
    const rel = isCurrent ? releasedBatches : inst_i.batches.length;
    log.push({ type: "header", key: `h${i}` });
    if (rel > 0) {
      log.push({ type: "context", key: `c${i}`, text: script.contextText });
      for (let k = 0; k < rel; k++) {
        const batch = inst_i.batches[k];
        log.push({ type: "decision", key: `d${i}-${k}`, parts: batch.decision });
        (batch.extra ?? []).forEach((parts, j) => {
          log.push({ type: "decision", key: `d${i}-${k}-x${j}`, parts });
        });
      }
    }
    if (!isCurrent || phase >= assertPhase) {
      if (inst_i.outcome.type === "ok") {
        log.push({ type: "ok", key: `ok${i}`, text: inst_i.outcome.text });
      } else {
        log.push({ type: "fail", key: `f${i}`, lines: inst_i.outcome.lines });
      }
    }
  }

  // Code-line highlights per phase.
  let activeLines: number[] = [];
  let activeMember: number | null = null;
  let failLines: number[] = [];
  const failed = instance.outcome.type === "fail" && phase >= assertPhase;
  if (phase < nSend) {
    const id = script.sendOrder[phase];
    activeLines = [script.msgs[id].sendLine];
    activeMember = script.msgs[id].member;
  } else if (phase < assertPhase) {
    activeLines = releasingNow?.activeLines ?? script.collectLines;
  } else if (instance.outcome.type === "ok") {
    activeLines = script.assertLines;
  } else {
    failLines = script.assertLines;
  }

  return {
    packets,
    activeLines,
    instanceNum: inst + 1,
    totalInstances: script.instances.length,
    log,
    // The operator and the `nondet!` code line flash on release decisions.
    flashOp: releasingNow
      ? releasingNow.flashOp !== undefined
        ? releasingNow.flashOp
        : script.releaseOpId
      : null,
    flashLines: releasingNow
      ? (releasingNow.flashLines ?? script.flashLines)
      : [],
    flashKey: step,
    activeMember,
    failLines,
    failed,
  };
}
