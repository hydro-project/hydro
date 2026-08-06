/**
 * The Hydro landing page: a scrollytelling walkthrough with a pinned,
 * morphing dataflow graph. See docs/src/components/landing/ for the
 * graph/code-panel building blocks.
 */

import React, { useEffect, useRef, useState } from "react";
import Link from "@docusaurus/Link";
import Layout from "@theme/Layout";
import Head from "@docusaurus/Head";

import PinnedFlowGraph from "../components/landing/PinnedFlowGraph";
import GraphExtras from "../components/landing/GraphExtras";
import CodePanel from "../components/landing/CodePanel";
import type { PaintRule } from "../components/landing/CodePanel";
import { SCENES, COLOR_VARS, computeFrame } from "../components/landing/scenes";
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
pub fn echo<'a>(
    client: &Process<'a, Client>,
    server: &Process<'a, Server>,
    requests: Stream<String, Process<'a, Client>>,
) -> Stream<String, Process<'a, Client>> {
    requests
        .send(server, TCP.fail_stop().bincode())
        // ⇒ Stream<String, Process<Server>>
        .map(q!(|s| s.to_uppercase()))
        // ⇒ Stream<String, Process<Server>>
        .send(client, TCP.fail_stop().bincode())
        // ⇒ Stream<String, Process<Client>>
}
`;

const CORRECTNESS_CODE = `
pub fn concat_words<'a>(
    server: &Process<'a, Server>,
    words: Stream<String, Cluster<'a, Client>>,
) -> Singleton<String, Process<'a, Server>> {
    words
        .send(server, TCP.fail_stop().bincode())
        // ⇒ Stream<String, Process<Server>, Unbounded, NoOrder>
        .fold(q!(String::new), q!(|acc, x| *acc += &x))
}
`;

const SIM_CODE = `
let (event_port, events) = clients.sim_input();

let log = events
    .send(&server, TCP.fail_stop().bincode())
    .entries_partially_ordered(nondet!(/** arrival order */))
    .sim_output();

flow.sim().with_cluster_size(&clients, 3).exhaustive(async || {
    event_port.send(0, "a1");
    event_port.send(0, "a2");
    event_port.send(1, "b1");

    let entries: Vec<_> = log.collect().await;
    assert!(pos("a1") < pos("a2")); // per-member order holds
});
`;

const CLOUD_CODE = `
let mut deployment = EcsDeploy::new();

let _nodes = flow
    .with_process(&client, deployment.add_ecs_process())
    .with_process(&server, deployment.add_ecs_process())
    .deploy(&mut deployment);

deployment.deploy().await.unwrap();
`;

// Shared token paints, matching the diagram's color coding.
const LOCATION_PAINTS: PaintRule[] = [
  { match: "Client", color: COLOR_VARS.client },
  { match: "Server", color: COLOR_VARS.server },
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
        title="src/echo.rs"
        code={GLOBAL_CODE}
        paints={[
          ...LOCATION_PAINTS,
          { match: "TCP", color: COLOR_VARS.network },
          { match: "send", color: COLOR_VARS.network },
          { match: "map", color: COLOR_VARS.server },
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
          Just like Rust ensures memory safety through the borrow checker,
          Hydro ensures <i>distributed safety</i> through <b>stream types</b>{" "}
          that track network behaviors end-to-end. Locations shape these
          types: when a <code>Cluster</code> of machines sends to a single{" "}
          <code>Process</code>, messages from different members arrive
          interleaved, so the resulting stream is unordered.
        </p>
        <p>
          If your logic relies on a message ordering that the network does
          not guarantee, Hydro rejects the program at compile time. Errors
          are surfaced through the Rust type system, visible to your editor,
          language server, and agents. To resolve them, you can prove
          properties like commutativity and idempotence, or explicitly
          handle the non-determinism.
        </p>
      </>
    ),
    render: () => (
      <CodePanel
        code={CORRECTNESS_CODE}
        paints={[
          { match: "Cluster", color: COLOR_VARS.client, bold: true },
          { match: "NoOrder", color: COLOR_VARS.error },
        ]}
        error={{
          line: 8,
          match: "fold",
          title:
            "`fold` requires an ordered stream, but this stream is `NoOrder`",
          notes: [
            "note: messages from different cluster members may be interleaved in any order",
            "help: prove the closure is commutative with `commutative = manual_proof!(...)`",
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
          Hydro offers built-in <b>deterministic simulation testing</b>,
          which lets you simulate distributed programs on your laptop. Tests
          run against varied distributed schedules, including message
          interleavings, batch boundaries, and state snapshots, to catch
          concurrency bugs and race conditions.
        </p>
        <p>
          Because Hydro's type system enforces determinism, the simulator
          only needs to explore <code>nondet!</code> decision points, so even
          large protocols can be checked <b>exhaustively</b>: your assertions
          become guarantees about <em>every possible</em> execution.
        </p>
      </>
    ),
    render: (frame, isActive) => (
      <CodePanel
        code={SIM_CODE}
        activeLines={isActive ? frame.activeLines : []}
        flashLines={isActive ? frame.flashLines || [] : []}
        flashKey={frame.flashKey || 0}
        paints={[
          { match: '"a1"', color: COLOR_VARS.client },
          { match: '"a2"', color: COLOR_VARS.client },
          { match: '"b1"', color: COLOR_VARS.chanB },
          { match: "nondet!", color: COLOR_VARS.error, bold: true },
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

  // Logical animation clock. The timeline is reset *synchronously* (during
  // render, via the derived-state pattern) whenever the scene changes, so
  // the first committed frame of a new scene is always step 0 — otherwise
  // packets would mount mid-animation at their end positions.
  const [anim, setAnim] = useState<{ sceneKey: SceneKey; step: number }>({
    sceneKey,
    step: 0,
  });
  if (anim.sceneKey !== sceneKey) {
    setAnim({ sceneKey, step: 0 });
  }
  useEffect(() => {
    const id = setInterval(
      () => setAnim((prev) => ({ ...prev, step: prev.step + 1 })),
      STEP_MS,
    );
    return () => clearInterval(id);
  }, [sceneKey]);

  const step = anim.sceneKey === sceneKey ? anim.step : 0;
  const frame = computeFrame(sceneKey, step);

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
