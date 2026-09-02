/**
 * The Hydro landing page: a scrollytelling walkthrough with a pinned,
 * morphing dataflow graph. See docs/src/components/landing/ for the
 * graph/code-panel building blocks and the scene definitions.
 *
 * The running example is a two-phase-commit protocol on one fixed topology
 * (a Cluster<Participant> and a Process<Leader>): the prepare round shows
 * location-oriented programming, a vote tally over a retrying channel shows
 * the compile-time checks, and a majority-quorum bug shows the simulator
 * catching a protocol race.
 */

import React, { useEffect, useRef, useState } from "react";
import Link from "@docusaurus/Link";
import Layout from "@theme/Layout";
import Head from "@docusaurus/Head";

import PinnedFlowGraph from "../components/landing/PinnedFlowGraph";
import GraphExtras from "../components/landing/GraphExtras";
import CodePanel from "../components/landing/CodePanel";
import {
  SCENES,
  COLOR_VARS,
  computeFrame,
  SIM_LAST_STEP,
} from "../components/landing/scenes";
import type { Frame, SceneKey } from "../components/landing/scenes";

import styles from "./index.module.css";

const STEP_MS = 900;

// ---------------------------------------------------------------------------
// Code snippets (stylized, but close to real Hydro syntax)
// ---------------------------------------------------------------------------

const GRPC_SERVER_CODE = `
impl Echo for EchoService {
    async fn echo(&self, req: Request<Msg>)
        -> Result<Response<Msg>, Status> {
        // where did this request come from? in what order?
        let text = req.into_inner().text;
        Ok(Response::new(Msg { text: text.to_uppercase() }))
    }
}
`;

const GRPC_CLIENT_CODE = `
let mut client = EchoClient::connect("http://node2:5000").await?;

// retries? reordering? failures? not visible here.
let reply = client.echo(Msg { text: "hello".into() }).await?;
`;

const GLOBAL_CODE = `
pub fn prepare_round<'a>(
    txns: Stream<Txn, Process<'a, Leader>>,
    leader: &Process<'a, Leader>,
    parts: &Cluster<'a, Participant>,
) -> KeyedStream<MemberId<Participant>, Vote, Process<'a, Leader>> {
    txns
        .broadcast(parts, TCP.fail_stop().bincode())
        // ⇒ Stream<Txn, Cluster<Participant>>, on every participant
        .map(q!(|txn| wal.prepare(txn)))
        .send(leader, TCP.fail_stop().bincode())
        // ⇒ every participant's vote, back on the leader
}
`;

const CORRECTNESS_CODE = `
pub fn count_yes<'a>(
    votes: Stream<Vote, Cluster<'a, Participant>>,
    leader: &Process<'a, Leader>,
) -> Singleton<usize, Process<'a, Leader>> {
    votes
        // re-sent on reconnect, so no vote is ever lost…
        .send(leader, TCP.retry_on_fail().bincode())
        // ⇒ Stream<Vote, …, NoOrder, AtLeastOnce>
        .fold(q!(|| 0), q!(|n, v| if v == Vote::Yes { *n += 1 }))
}
`;

const SIM_CODE = `
let (vote_port, votes) = participants.sim_input();

let quorum = votes
    .send(&leader, TCP.fail_stop().bincode())
    .entries_partially_ordered(
        nondet!(/** votes from members interleave: NYY? YNY? YYN? */))
    .map(q!(|(_participant, vote)| vote))
    .limit(q!(2)) // BUG: 2 of 3 is a majority, not unanimity
    .sim_output();

flow.sim().with_cluster_size(&participants, 3).exhaustive(async || {
    vote_port.send(0, Vote::No); // this participant must veto
    vote_port.send(1, Vote::Yes);
    vote_port.send(2, Vote::Yes);

    let seen: Vec<_> = quorum.collect().await;
    assert!(seen.contains(&Vote::No)); // the veto must be heard
});
`;

/* Kept for the temporarily-disabled "cloud" section below.
const CLOUD_CODE = `
let mut deployment = EcsDeploy::new();

let _nodes = flow
    .with_process(&leader, deployment.add_ecs_process())
    .with_cluster(&participants, deployment.add_ecs_cluster())
    .deploy(&mut deployment);

deployment.deploy().await.unwrap();
`;
*/

// Shared token paints, matching the diagram's color coding.
const LOCATION_PAINTS = [
  { match: "Leader", color: COLOR_VARS.server },
  { match: "Participant", color: COLOR_VARS.client },
  { match: "Cluster", color: COLOR_VARS.client, bold: true },
];

// ---------------------------------------------------------------------------
// Sections
// ---------------------------------------------------------------------------

interface Section {
  key: SceneKey;
  title: string;
  blurb: React.ReactNode;
  render: (frame: Frame, isActive: boolean) => React.ReactNode;
}

const SECTIONS: Section[] = [
  {
    key: "intro",
    title: "Distributed systems deserve a native framework",
    blurb: (
      <>
        <p>
          Nearly every application today is a distributed system: services
          call other services, replicas coordinate state, and data flows
          across regions. Distribution is how modern software scales,
          survives failures, and stays close to its users.
        </p>
        <p>
          Hydro is a Rust framework that treats distribution as a{" "}
          <b>first-class concern</b>. Instead of assembling a system from
          parts and relying on manual review to catch mistakes at their
          boundaries, you express, check, test, and deploy the whole
          distributed system as one program.
        </p>
      </>
    ),
    render: () => null,
  },
  {
    key: "grpc",
    title: "Today's frameworks make networks implicit",
    blurb: (
      <>
        <p>
          Most frameworks split a distributed system into single-machine
          programs that communicate through opaque RPC calls. The network—the
          part that makes your system <em>distributed</em>—is hidden inside
          client stubs and <code>await</code> points, invisible to the
          compiler and your tools.
        </p>
        <p>
          Reordering, duplication, and partial failure all live in the gap
          between these files. Because the language cannot see the network,
          it cannot help you reason about what happens across machines.
        </p>
      </>
    ),
    render: () => (
      <div className={styles.codeStack}>
        <CodePanel
          title="node1/src/main.rs"
          code={GRPC_CLIENT_CODE}
          paints={[
            { match: "connect", color: COLOR_VARS.grey },
            { match: "await", color: COLOR_VARS.grey },
          ]}
        />
        <CodePanel
          title="node2/src/service.rs"
          code={GRPC_SERVER_CODE}
          paints={[
            { match: "// where did this request come from? in what order?", color: COLOR_VARS.grey },
          ]}
        />
      </div>
    ),
  },
  {
    key: "global",
    title: "Hydro is global",
    blurb: (
      <>
        <p>
          Hydro is the first production framework with{" "}
          <b>location-oriented programming</b>: a single function can
          encapsulate logic spanning several machines. Distributed locations
          are captured in <b>types</b>, and sending data across the network
          is an explicit, type-checked operation.
        </p>
        <p>
          These abstractions are <b>zero-cost</b>: Hydro compiles to the same
          networked binaries you would write by hand, and you retain full
          control over the network protocol, compute placement, and
          serialization format.
        </p>
      </>
    ),
    render: () => (
      <CodePanel
        title="src/two_pc.rs"
        code={GLOBAL_CODE}
        paints={[
          ...LOCATION_PAINTS,
          { match: "TCP", color: COLOR_VARS.network },
          { match: "broadcast", color: COLOR_VARS.network },
          { match: "send", color: COLOR_VARS.network },
          { match: "map", color: COLOR_VARS.client },
        ]}
      />
    ),
  },
  {
    key: "correctness",
    title: "Hydro catches distributed bugs at compile time",
    blurb: (
      <>
        <p>
          Hydro encodes distributed behavior in the type system, end-to-end:
          every stream carries types that track <b>ordering guarantees</b>{" "}
          and <b>retries</b>, derived from your distributed logic.
        </p>
        <p>
          Hydro uses these types to enforce <b>eventual determinism</b>: code
          whose result could be affected by timing or duplication does not
          compile. You discharge the obligation with an algebraic proof —
          idempotence, commutativity — that the compiler holds you to, or you
          scope the non-determinism explicitly with <code>nondet!</code>.
          Whole classes of distributed systems bugs become unrepresentable.
        </p>
      </>
    ),
    render: () => (
      <CodePanel
        code={CORRECTNESS_CODE}
        paints={[
          ...LOCATION_PAINTS,
          { match: "TCP", color: COLOR_VARS.network },
          { match: "retry_on_fail", color: COLOR_VARS.network },
          { match: "send", color: COLOR_VARS.network },
          { match: "AtLeastOnce", color: COLOR_VARS.error },
        ]}
        error={{
          line: 9,
          match: "fold",
          title:
            "`fold` requires `ExactlyOnce` delivery, but this stream is `AtLeastOnce`",
          notes: [
            "help: prove the closure is idempotent with `idempotent = manual_proof!(...)`, or acknowledge the non-determinism with `assume_retries(nondet!(…))`",
          ],
        }}
      />
    ),
  },
  {
    key: "sim",
    title: "Hydro lets you write distributed tests",
    blurb: (
      <>
        <p>
          The compiler and the simulator split the work: the type system
          guides you to eliminate sources of non-determinism, and the
          simulator exhaustively explores the <code>nondet!</code> points you
          kept — the only places non-determinism can live.
        </p>
        <p>
          Because the simulator only focuses on <code>nondet!</code> points,
          it is incredibly efficient: entire distributed protocols can be
          exhaustively checked on your laptop. That turns assertions into
          guarantees about <em>every possible execution</em>. And when a
          test fails, you get the exact schedule that broke it, replayable
          deterministically instead of flaking.
        </p>
      </>
    ),
    render: (frame, isActive) => (
      <CodePanel
        code={SIM_CODE}
        activeLines={isActive ? frame.activeLines : []}
        flashLines={isActive ? frame.flashLines || [] : []}
        failLines={isActive ? frame.failLines || [] : []}
        flashKey={frame.flashKey || 0}
        paints={[
          { match: "nondet!", color: COLOR_VARS.error, bold: true },
          { match: "// BUG: 2 of 3 is a majority, not unanimity", color: COLOR_VARS.error },
          { match: "No", color: COLOR_VARS.client },
          { match: "Yes", color: COLOR_VARS.chanB, line: 13 },
          { match: "Yes", color: COLOR_VARS.pink, line: 14 },
        ]}
      />
    ),
  },
  // Temporarily disabled; see git history to restore.
  /*
  {
    key: "cloud",
    title: "Native cloud infrastructure",
    blurb: (
      <>
        <p>
          The same program deploys <b>unchanged</b>. Hydro Deploy provisions
          machines and wires up the network on AWS ECS, EC2, GCP, and
          more—each location in your code becomes a real service in the
          cloud.
        </p>
        <p>
          Deployment scripts are written in plain Rust and share your
          program's topology, so your infrastructure always stays in sync
          with your application logic, with observability tooling built in.
        </p>
      </>
    ),
    render: () => (
      <CodePanel
        code={CLOUD_CODE}
        staticLines={[1, 4, 5]}
        paints={[
          ...LOCATION_PAINTS,
          { match: "EcsDeploy", color: COLOR_VARS.aws },
          { match: "add_ecs_process", color: COLOR_VARS.aws },
          { match: "add_ecs_cluster", color: COLOR_VARS.aws },
        ]}
      />
    ),
  },
  */
];

// ---------------------------------------------------------------------------
// Scrollytelling wiring
// ---------------------------------------------------------------------------

function useActiveSection(
  sectionRefs: React.RefObject<(HTMLElement | null)[]>,
) {
  const [active, setActive] = useState(0);
  useEffect(() => {
    const mobileQuery = window.matchMedia("(max-width: 996px)");
    const onScroll = () => {
      // Desktop: switch when a section crosses the viewport centerline.
      // Mobile: the island is pinned across the top of the viewport, so use
      // a lower line that clears the island's *maximum* height (graph cap +
      // trace panel cap). It must be a stable constant — deriving it from
      // the island's current height would oscillate, since the island's
      // height depends on which section is active.
      const line = window.innerHeight * (mobileQuery.matches ? 0.75 : 0.55);
      let best = 0;
      sectionRefs.current.forEach((el, i) => {
        if (el && el.getBoundingClientRect().top <= line) {
          best = i;
        }
      });
      setActive(best);
    };
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", onScroll);
    return () => {
      window.removeEventListener("scroll", onScroll);
      window.removeEventListener("resize", onScroll);
    };
  }, [sectionRefs]);
  return active;
}

function ScrollyStory() {
  const sectionRefs = useRef<(HTMLElement | null)[]>([]);
  const active = useActiveSection(sectionRefs);
  const sceneKey = SECTIONS[active].key;

  // Logical animation clock. The timeline is reset *synchronously* whenever
  // the scene changes, using React's supported "adjust state during render"
  // pattern (see react.dev: "You Might Not Need an Effect"); React restarts
  // the render before committing, and this stays safe under concurrent
  // rendering. Resetting in an effect instead would briefly commit a frame
  // from the *previous* scene's clock, mounting packets mid-animation at
  // their end positions.
  const [anim, setAnim] = useState<{ sceneKey: SceneKey; step: number }>({
    sceneKey,
    step: 0,
  });
  if (anim.sceneKey !== sceneKey) {
    setAnim({ sceneKey, step: 0 });
  }
  useEffect(() => {
    const id = setInterval(
      () =>
        setAnim((prev) => {
          // The sim animation plays once and then waits for a restart.
          if (prev.sceneKey === "sim" && prev.step >= SIM_LAST_STEP) {
            return prev;
          }
          return { ...prev, step: prev.step + 1 };
        }),
      STEP_MS,
    );
    return () => clearInterval(id);
  }, [sceneKey]);

  const step = anim.sceneKey === sceneKey ? anim.step : 0;
  const frame = computeFrame(sceneKey, step);
  const simDone = sceneKey === "sim" && step >= SIM_LAST_STEP;

  return (
    <div className={styles.scrolly}>
      {/* The graph column comes first in the DOM so it can stick to the top
          of the viewport on mobile; flex `order` puts it on the right on
          desktop. */}
      <div className={styles.scrollyGraphCol} aria-hidden="true">
        <div className={styles.graphSticky}>
          <div className={styles.graphIsland}>
            <PinnedFlowGraph
              scene={SCENES[sceneKey]}
              packets={frame.packets || []}
              stepMs={STEP_MS}
              flashOp={frame.flashOp || null}
              flashKey={frame.flashKey || 0}
              activeMember={frame.activeMember ?? null}
            />
            <GraphExtras
              sceneKey={sceneKey}
              scene={SCENES[sceneKey]}
              frame={frame}
              simDone={simDone}
              onRestart={() => setAnim({ sceneKey, step: 0 })}
            />
          </div>
        </div>
      </div>
      <div className={styles.scrollySections}>
        {SECTIONS.map((section, i) => (
          <section
            key={section.key}
            ref={(el) => {
              sectionRefs.current[i] = el;
            }}
            className={`${styles.storySection} ${
              i === active ? styles.storySectionActive : ""
            }`}
          >
            <h2 className={styles.storyTitle}>{section.title}</h2>
            <div className={styles.storyBlurb}>{section.blurb}</div>
            {section.render(frame, i === active)}
          </section>
        ))}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function Home() {
  return (
    <Layout>
      <Head>
        <title>
          Hydro - a Rust framework for correct and performant distributed
          systems
        </title>
        <meta
          property="og:title"
          content="Hydro - a Rust framework for correct and performant distributed systems"
        />
      </Head>
      <main className={styles.landingRoot}>
        <div className={styles.jumbo}>
          <img
            src="/img/hydro-logo.svg"
            alt="Hydro Logo"
            style={{
              width: "550px",
              maxWidth: "100%",
              marginLeft: "auto",
              marginRight: "auto",
            }}
          />
          <h2 className={styles.indexTitle}>
            A Rust framework for correct and performant distributed systems
          </h2>

          <div className={styles.heroButtons}>
            <Link
              to="/docs/hydro/learn/quickstart/"
              className="button button--primary button--lg"
              style={{
                margin: "10px",
                marginTop: 0,
                fontSize: "1.4em",
                color: "white",
              }}
            >
              Get Started
            </Link>

            <Link
              to="/docs/hydro/reference/"
              className="button button--outline button--secondary button--lg"
              style={{
                margin: "10px",
                marginTop: 0,
                fontSize: "1.4em",
              }}
            >
              Learn More
            </Link>
          </div>
        </div>

        <ScrollyStory />

        <div className={styles.panel}>
          <div
            style={{
              flexGrow: 1,
              maxWidth: "650px",
            }}
          >
            <h1>Research Backed. Production Ready.</h1>
            <p>
              Hydro has its roots in foundational distributed systems research
              at UC Berkeley, such as the CALM theorem. It is now co-led by a
              team at Berkeley and AWS, with contributions from the open-source
              community.
            </p>
            <p>
              Hydro continues to lead the way with cutting-edge capabilities,
              such as automatically optimizing distributed protocols, while
              supporting production use with cloud integrations and
              observability tooling.
            </p>
            <div style={{ marginTop: "25px" }}>
              <Link
                to="/docs/hydro/learn/quickstart/"
                className="button button--primary button--lg"
                style={{ color: "white" }}
              >
                Get Started with Hydro
              </Link>
            </div>
          </div>

          <div
            style={{ minWidth: "260px", width: 0, marginBottom: 0 }}
            className={styles.panelImage}
          >
            <img
              src="/img/hydro-papers.png"
              alt="Hydro research papers"
              style={{
                display: "block",
                minWidth: "0px",
                width: "100%",
                borderRadius: "15px",
              }}
            ></img>
          </div>
        </div>
      </main>
    </Layout>
  );
}
