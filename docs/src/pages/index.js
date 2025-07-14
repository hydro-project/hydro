import Link from "@docusaurus/Link";
import Layout from "@theme/Layout";

import styles from "./index.module.css";
import Head from "@docusaurus/Head";

export default function Home() {
  return (
    <Layout>
      <Head>
        <title>Hydro - a Rust framework for correct and performant distributed systems</title>
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
                to="/docs/hydro/quickstart/"
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
                to="/docs/hydro/"
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
            <h1>Zero-Cost Correctness</h1>
            <p>
              Hydro is a correctness-first framework, helping you avoid distributed systems bugs at each stage of development. Just like Rust ensures memory safety through the borrow checker, Hydro ensures <i>distributed safety</i> through <b>stream types</b>. These types have <b>zero runtime overhead</b>; you retain full control over the network protocol, compute placement, and serialization format.
            </p>
            <p>Hydro automatically flags situations where messages may be out-of-order, or duplicated, and guides you to appropriately handle them. These are surfaced through the Rust type system, visible to your editor, language server, and agents.</p>
            <div className={styles["inDevPanel"]}>
              <b>In Development (<a href={"https://github.com/hydro-project/hydro/issues/1876"}>#1876</a>)</b>
              <p style={{
                fontSize: "0.9em",
                marginBottom: 0,
              }}>Hydro will soon offer built-in <b>deterministic simulation testing</b>, allowing you to locally simulate various distributed scenarios, including network partitions, message delays, and failures.</p>
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
            <h1>Bare-Metal Performance</h1>
            <p>
              Hydro is powered by the <b>Dataflow Intermediate Representation (DFIR)</b>, a compiler and low-level runtime for stream processing. DFIR uses a microbatch processing architecture, which enables automatic vectorization and efficient scheduling without restricting your application logic.
            </p>
            <p>DFIR emits rich profiling information for observability and performance tuning, and can automatically generate Mermaid diagrams to visualize your streaming logic. It already powers production database engines like <a style={{ fontWeight: "bold" }} href="https://github.com/GreptimeTeam/greptimedb">GreptimeDB</a>.</p>
            <div className={styles["comingSoonPanel"]}>
              <b>Coming Soon (<a href={"https://github.com/hydro-project/hydro/issues/1890"}>#1890</a>)</b>
              <p style={{
                fontSize: "0.9em",
                marginBottom: 0,
              }}>We are developing an io-uring backend for DFIR that enables even higher network performance and zero-copy I/O, all without any changes to your high-level Hydro logic.</p>
            </div>
          </div>

          <div style={{ width: 0 }} className={styles["panelImage"]}>
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
              Hydro has its roots in foundational distributed systems research at UC Berkeley, such as the CALM theorem. It is now co-led by a team at Amazon Web Services and Berkeley, with contributions from the open-source community.
            </p>
            <p>Hydro continues to lead the way with cutting-edge capabilities, such as automatically optimizing distributed protocols, while supporting production use with cloud integrations and observability tooling.</p>
          </div>

          <div style={{ width: 0, marginBottom: 0 }} className={styles["panelImage"]}>
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
