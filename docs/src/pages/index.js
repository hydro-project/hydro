import Link from "@docusaurus/Link";
import Layout from "@theme/Layout";
import CodeBlock from '@theme/CodeBlock';

import styles from "./index.module.css";
import Head from "@docusaurus/Head";

export default function Home() {
  return (
    <Layout>
      <Head>
        <title>Hydro - a Rust framework for correct and performant distributed systems</title>
        <meta property="og:title" content="Hydro - a Rust framework for correct and performant distributed systems" />
      </Head>
      <main>
        <div className={styles["jumbo"]}>
          <img
            src="/img/hydro-logo.svg"
            alt="Hydro Logo"
            style={{
              width: "550px",
              marginLeft: "auto",
              marginRight: "auto",
            }}
          />
          <h2 className={styles["indexTitle"]}>
            A Rust framework for correct and performant distributed systems
          </h2>

          <div style={{ marginTop: "20px" }}>
            <div
              style={{
                display: "flex",
                flexDirection: "row",
                marginTop: "10px",
                marginBottom: "30px",
                justifyContent: "center",
                flexWrap: "wrap",
              }}
            >
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
        </div>
        <div className={styles["panel"]}>
          <div style={{
            flexGrow: 1,
            maxWidth: "650px"
          }}>
            <h1>Distributed Safety Built-In</h1>
            <p>
              Hydro helps you avoid distributed systems bugs at each stage of development. Just like Rust ensures memory safety through the borrow checker, Hydro ensures <i>distributed safety</i> through <b>stream types</b>. These types have <b>zero runtime overhead</b>; you retain full control over the network protocol, compute placement, and serialization format.
            </p>
            <p>Hydro automatically flags situations where messages may be out-of-order, or duplicated, and guides you to appropriately handle them. These are surfaced through the Rust type system, visible to your editor, language server, and agents.</p>
            <div className={styles["inDevPanel"]}>
              <b>In Preview (<a href={"https://github.com/hydro-project/hydro/pull/2158"}>#2158</a>)</b>
              <p style={{
                fontSize: "0.95em",
                marginTop: "5px",
                marginBottom: 0,
              }}>Hydro offers built-in <b>deterministic simulation testing</b>, which lets you simulate distributed programs on your laptop and use cutting-edge fuzzers to quickly find complex edge-cases.</p>
            </div>
          </div>

          <div className={styles["panelImage"]}>
            <iframe
              style={{
                display: "block",
                marginLeft: "auto",
                marginRight: "auto",
                width: "100%",
                aspectRatio: "16 / 9",
                borderRadius: "15px"
              }}
              src="https://www.youtube.com/embed/LdZ94m7anTw?si=5duyR1MjSRRdPJId"
              title="YouTube video player"
              frameBorder="0"
              allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
              referrerPolicy="strict-origin-when-cross-origin"
              allowFullScreen
            ></iframe>
          </div>
        </div>

        <div className={styles["panel"]}>
          <div style={{
            flexGrow: 1,
            maxWidth: "650px"
          }}>
            <h1>A New Kind of Modularity</h1>
            <p>
              Hydro is the first production framework with <b>location-oriented programming</b>, where a single function can encapsulate logic spanning several machines. Instead of splitting your app on network boundaries with opaque RPC calls, Hydro encourages you to split your app into <b>logical modules</b> that can be independently tested.
            </p>
            <p style={{
              marginBottom: 0
            }}>Quorum counting? Two-Phase Commit? Each a single function in Hydro's standard library. Want just the leader-election piece of Paxos? It's already a separate module. Hydro unlocks new opportunities to share distributed code across your organization with confidence.</p>
          </div>

          <div className={styles["panelImage"]} style={{ fontSize: "16px", flexShrink: 1, flexBasis: "10%", marginBottom: "calc(-1 * var(--ifm-leading))" }}>
            <CodeBlock
          language="rust">
            {`fn reduce_from_cluster(
  data: Stream<usize, Cluster<Worker>>,
  leader: &Process<Leader>
) -> Singleton<usize, Process<Leader>> {
  data
    .send(leader, TCP.bincode())
    // Stream<(MemberId<Worker>, usize), Process<Leader>, ..., NoOrder>
    .map(q!(|v| v.1)) // drop the ID
    .fold_commutative(q!(0), q!(|acc, v| *acc += v))
}`}
            </CodeBlock>
          </div>
        </div>

        <div className={styles["panel"]}>
          <div style={{
            flexGrow: 1,
            maxWidth: "650px"
          }}>
            <h1>Bare-Metal Performance</h1>
            <p>
              Hydro is powered by the <b>Dataflow Intermediate Representation (DFIR)</b>, a compiler and low-level runtime for stream processing. DFIR uses a microbatch processing architecture, which enables automatic vectorization and efficient scheduling without restricting your application logic.
            </p>
            <p>DFIR emits rich profiling information for observability and performance tuning, and can automatically generate Mermaid diagrams to visualize your streaming logic. It already powers production database engines like <a style={{ fontWeight: "bold" }} href="https://github.com/GreptimeTeam/greptimedb">GreptimeDB</a>.</p>
            <div className={styles["comingSoonPanel"]}>
              <b>Coming Soon (<a href={"https://github.com/hydro-project/hydro/issues/1890"}>#1890</a>)</b>
              <p style={{
                fontSize: "0.9em",
                marginTop: "5px",
                marginBottom: 0,
              }}>We are developing an io-uring backend for DFIR that enables even higher network performance and zero-copy I/O, all without any changes to your high-level Hydro logic.</p>
            </div>
          </div>

          <div style={{ minWidth: "260px", width: 0 }} className={styles["panelImage"]}>
            <img
              src="/img/dfir-profile.png"
              style={{
                display: "block",
                minWidth: "0px",
                width: "100%",
                borderRadius: "15px"
              }}
            ></img>
          </div>
        </div>

        <div className={styles["panel"]}>
          <div style={{
            flexGrow: 1,
            maxWidth: "650px"
          }}>
            <h1>Research Backed. Production Ready.</h1>
            <p>
              Hydro has its roots in foundational distributed systems research at UC Berkeley, such as the CALM theorem. It is now co-led by a team at Berkeley and AWS, with contributions from the open-source community.
            </p>
            <p>Hydro continues to lead the way with cutting-edge capabilities, such as automatically optimizing distributed protocols, while supporting production use with cloud integrations and observability tooling.</p>
          </div>

          <div style={{ minWidth: "260px", width: 0, marginBottom: 0 }} className={styles["panelImage"]}>
            <img
              src="/img/hydro-papers.png"
              style={{
                display: "block",
                minWidth: "0px",
                width: "100%",
                borderRadius: "15px"
              }}
            ></img>
          </div>
        </div>
      </main>
    </Layout>
  );
}
