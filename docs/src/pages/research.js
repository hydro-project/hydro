import React from 'react';
import Link from '@docusaurus/Link';
import Layout from '@theme/Layout';
import Image from '@theme/IdealImage';

import styles from './research.module.css';
import Head from '@docusaurus/Head';

const papers = [
  {
    title: "Flo: a Semantic Foundation for Progressive Stream Processing",
    pdf: "pathname:///papers/flo.pdf",
    thumb: require("./img/papers/flo.png"),
    authors: <>Shadaj Laddad, Alvin Cheung, Joseph M. Hellerstein, Mae Milano</>,
    description: [
      <>Existing streaming languages have a variety of semantic models and guarantees that are often incompatible. In this paper, we identify two general yet precise semantic properties: streaming progress and eager execution. We formally define these properties in the context of Flo, a parameterized streaming language that abstracts over dataflow operators and the underlying structure of streams.</>,
      <>To demonstrate the generality of our properties, we show how key ideas from representative streaming and incremental computation systems—Flink, LVars, and DBSP—have semantics that can be modeled in Flo and guarantees that map to our properties.</>
    ],
    conf: "POPL 2025",
    links: <><Link href="pathname:///papers/flo.pdf">PDF</Link> / <Link href="https://arxiv.org/abs/2411.08274">arXiv</Link></>
  },
  {
    title: "Optimizing Distributed Protocols with Query Rewrites",
    pdf: "pathname:///papers/david-sigmod-2024.pdf",
    thumb: require("./img/papers/david-sigmod-2024.png"),
    authors: <>David Chu, Rithvik Panchapakesan, Shadaj Laddad, Lucky Katahanas, Chris Liu, Kaushik Shivakumar, Natacha Crooks, Joseph M. Hellerstein, & Heidi Howard</>,
    description: [
      <>Distributed protocols such as 2PC and Paxos lie at the core of many systems in the cloud, but standard implementations do not scale. New scalable distributed protocols are developed through careful analysis and rewrites, but this process is ad hoc and error-prone. This paper presents an approach for scaling any distributed protocol by applying rule-driven rewrites, borrowing from query optimization.</>,
      <>Distributed protocol rewrites entail a new burden: reasoning about spatiotemporal correctness. We leverage order-insensitivity and data dependency analysis to systematically identify correct coordination-free scaling opportunities. We apply this analysis to create preconditions and mechanisms for coordination-free decoupling and partitioning, two fundamental vertical and horizontal scaling techniques. Manual rule-driven applications of decoupling and partitioning improve the throughput of 2PC by 5x and Paxos by 3x, and match state-of-the-art throughput in recent work. These results point the way toward automated optimizers for distributed protocols based on correct-by-construction rewrite rules.</>
    ],
    conf: "SIGMOD 2024",
    links: <><Link href="pathname:///papers/david-sigmod-2024.pdf">PDF</Link> / <Link href="https://arxiv.org/abs/2404.01593">Tech Report</Link> / <Link href="https://github.com/rithvikp/autocomp">GitHub</Link></>
  },
  {
    title: "SkyPIE: A Fast & Accurate Oracle for Object Placement",
    pdf: "pathname:///papers/tiemo-sigmod-2024.pdf",
    thumb: require("./img/papers/tiemo-sigmod-2024.png"),
    authors: <>Tiemo Bang, Chris Douglas, Natacha Crooks and Joseph M. Hellerstein</>,
    description: [
      <>Cloud object stores offer vastly different price points for object storage as a function of workload and geography. Poor object placement can thus lead to significant cost overheads. Prior cost-saving techniques attempt to optimize placement policies on the fly, deciding object placements for each object individually. In practice, these techniques do not scale to the size of the modern cloud. In this work, we leverage the static nature and pay-per-use pricing model of cloud environments to explore a different approach. Rather than computing object placements on the fly, we precompute a SkyPIE oracle---a lookup structure representing all possible placement policies and the workloads for which they are optimal.</>,
      <>Internally, SkyPIE represents placement policies as a matrix of cost-hyperplanes, which we effectively precompute through pruning and convex optimization. By leveraging a fast geometric algorithm, online queries then are 1 to 8 orders of magnitude faster but as accurate as Integer-Linear-Programming. This makes exact optimization tractable for real workloads and we show {">"}10x cost savings compared to state-of-the-art heuristic approaches.</>
    ],
    conf: "SIGMOD 2024",
    links: <><Link href="pathname:///papers/tiemo-sigmod-2024.pdf">PDF</Link> / <Link href="https://github.com/hydro-project/cloud_oracle_skypie">GitHub</Link></>
  },
  {
    title: "Suki: Choreographed Distributed Dataflow in Rust",
    pdf: "pathname:///papers/suki.pdf",
    thumb: require("./img/papers/suki.png"),
    authors: <>Shadaj Laddad, Alvin Cheung, Joseph M. Hellerstein</>,
    description: [
      <>Programming models for distributed dataflow have long focused on analytical workloads that allow the runtime to dynamically place and schedule compute logic. Meanwhile, models that enable fine-grained control over placement, such as actors, make global optimization difficult. In this extended abstract, we present Suki, an embedded Rust DSL that lets developers implement streaming dataflow with explicit placement of computation.</>,
      <>Key to this choreographic programming approach is our use of staged programming, which lets us expose a high-level Rust API while compiling local compute units into individual binaries with zero-overhead.</>
    ],
    conf: "CP 2024",
    links: <><Link href="pathname:///papers/suki.pdf">PDF</Link> / <Link href="https://arxiv.org/abs/2406.14733">arXiv</Link></>
  },
  {
    title: "Bigger, not Badder: Safely Scaling BFT Protocols",
    pdf: "pathname:///papers/david-papoc-2024.pdf",
    thumb: require("./img/papers/david-papoc-2024.png"),
    authors: <>David Chu, Chris Liu, Natacha Crooks, Joseph M. Hellerstein, & Heidi Howard</>,
    description: [
      <>Byzantine Fault Tolerant (BFT) protocols provide powerful guarantees in the presence of arbitrary machine failures, yet they do not scale. The process of creating new, scalable BFT protocols requires expert analysis and is often error-prone. Recent work suggests that localized, rule-driven rewrites can be mechanically applied to scale existing (non-BFT) protocols, including Paxos. We modify these rewrites--- decoupling and partitioning---so they can be safely applied to BFT protocols, and apply these rewrites to the critical path of PBFT, improving its throughput by 5x. We prove the correctness of the modified rewrites on any BFT protocol by formally modeling the arbitrary logic of a Byzantine node. We define the Borgesian simulator, a theoretical node that simulates a Byzantine node through randomness, and show that in any BFT protocol, the messages that a Borgesian simulator can generate before and after optimization is the same. Our initial results point the way towards an automatic optimizer for BFT protocols.</>
    ],
    conf: "PaPoC 2024",
    links: <><Link href="pathname:///papers/david-papoc-2024.pdf">PDF</Link> / <Link href="https://github.com/rithvikp/autocomp">GitHub</Link></>
  },
  {
    title: "Wrapping Rings in Lattices: An Algebraic Symbiosis of Incremental View Maintenance and Eventual Consistency",
    pdf: "pathname:///papers/conor-papoc-2024.pdf",
    thumb: require("./img/papers/conor-papoc-2024.png"),
    authors: <>Conor Power, Saikrishna Achalla, Ryan Cottone, Nathaniel Macasaet & Joseph M. Hellerstein</>,
    description: [
      <>We reconcile the use of semi-lattices in CRDTs and the use of groups and rings in incremental view maintenance to construct systems with strong eventual consistency, incremental computation, and database query optimization.</>
    ],
    conf: "PaPoC 2024",
    links: <><Link href="pathname:///papers/conor-papoc-2024.pdf">PDF</Link></>
  },
  {
    title: "Optimizing the cloud? Don't train models. Build oracles!",
    pdf: "pathname:///papers/tiemo-cidr-2024.pdf",
    thumb: require("./img/papers/tiemo-cidr-2024.png"),
    authors: <>Tiemo Bang, Conor Power, Siavash Ameli, Natacha Crooks & Joseph M. Hellerstein</>,
    description: [
      <>We propose cloud oracles as an alternative to machine learning for online optimization of cloud configurations. Our cloud oracle approach guarantees complete accuracy and explainability of decisions for problems that can be formulated as parametric convex optimizations. We give experimental evidence of this technique’s efficacy and share a vision of research directions for expanding its applicability.</>
    ],
    conf: "CIDR 2024",
    links: <><Link href="pathname:///papers/tiemo-cidr-2024.pdf">PDF</Link></>
  },
  {
    title: "Keep CALM and CRDT On",
    pdf: "pathname:///papers/keep-calm-and-crdt-on.pdf",
    thumb: require("./img/papers/keep-calm-and-crdt-on.png"),
    authors: <>Shadaj Laddad<sup>*</sup>, Conor Power<sup>*</sup>, Mae Milano, Alvin Cheung, Natacha Crooks, Joseph M. Hellerstein</>,
    description: [
      <>Conflict-free replicated datatypes (CRDTs) are a promising line of work that enable coordination-free replication and offer certain eventual consistency guarantees in a relatively simple object-oriented API. Yet CRDT guarantees extend only to data updates; observations of CRDT state are unconstrained and unsafe. We propose an agenda that embraces the simplicity of CRDTs, but provides richer, more uniform guarantees.</>,
      <>We extend CRDTs with a query model that reasons about which queries are safe without coordination by applying monotonicity results from the CALM Theorem, and lay out a larger agenda for developing CRDT data stores that let developers safely and efficiently interact with replicated application state.</>
    ],
    conf: "VLDB 2023",
    links: <><Link href="pathname:///papers/keep-calm-and-crdt-on.pdf">PDF</Link> / <Link href="https://arxiv.org/abs/2210.12605">arXiv</Link></>
  },
  {
    title: "Invited Paper: Initial Steps Toward a Compiler for Distributed Programs",
    pdf: "pathname:///papers/joe-applied-2023.pdf",
    thumb: require("./img/papers/joe-applied-2023.png"),
    authors: <>Joseph M. Hellerstein, Shadaj Laddad, Mae Milano, Conor Power & Mingwei Samuel</>,
    description: [
      <>In the Hydro project we are designing a compiler toolkit that can optimize for the concerns of distributed systems, including scale-up and scale-down, availability, and consistency of outcomes across replicas. This invited paper overviews the project, and provides an early walk-through of the kind of optimization that is possible. We illustrate how type transformations as well as local program transformations can combine, step by step, to convert a single- node program into a variety of distributed design points that offer the same semantics with different performance and deployment characteristics.</>
    ],
    conf: "ApPLIED 2023",
    links: <><Link href="pathname:///papers/joe-applied-2023.pdf">PDF</Link> / <Link href="https://arxiv.org/abs/2210.12605">arXiv</Link></>
  },
  {
    title: "Katara: Synthesizing CRDTs with Verified Lifting",
    pdf: "pathname:///papers/katara.pdf",
    thumb: require("./img/papers/katara.png"),
    authors: <>Shadaj Laddad, Conor Power, Mae Milano, Alvin Cheung, Joseph M. Hellerstein</>,
    description: [
      <>In this paper, we present Katara, a program synthesis-based system that takes sequential data type implementations and automatically synthesizes verified CRDT designs from them.</>,
      <>Katara is able to automatically synthesize CRDTs for a wide variety of scenarios, from reproducing classic CRDTs to synthesizing novel designs based on specifications in existing literature. Crucially, our synthesized CRDTs are fully, automatically verified, eliminating entire classes of common errors and reducing the process of producing a new CRDT from a painstaking paper proof of correctness to a lightweight specification.</>
    ],
    conf: "OOPSLA 2022",
    links: <><Link href="pathname:///papers/katara.pdf">PDF</Link> / <Link href="https://arxiv.org/abs/2205.12425">arXiv</Link> / <Link href="https://github.com/hydro-project/katara">GitHub</Link></>
  },
  {
    title: "Automatic Compartmentalization of Distributed Protocols",
    pdf: "pathname:///papers/auto-compartmentalization-src.pdf",
    thumb: require("./img/papers/auto-compartmentalization-src.png"),
    authors: <>David Chu</>,
    description: [
      <>Consensus protocols are inherently complex and difficult to reason about, and they often become bottlenecks in practice. This has driven the design of scalable protocol variants. Unfortunately, these variants are even more intricate and often error-ridden. The goal of our work is to stop inventing protocols, and instead systematize the scalability ideas from Compartmentalized Paxos so they can be applied automatically to a wide variety of complex protocols, including transactional concurrency control, BFT, etc.</>,
      <>Our vision of Automatic Compartmentalization proposes to increase throughput while preserving correctness and liveness, expanding the impact of compartmentalization to a broad range of programs</>
    ],
    conf: <>SOSP SRC 2021 · <span className={styles["award"]}>Student Research Competition Winner</span></>
  },
  {
    title: "Hydroflow: A Model and Runtime for Distributed Systems Programming",
    pdf: "pathname:///papers/hydroflow-thesis.pdf",
    thumb: require("./img/papers/hydroflow-thesis.png"),
    authors: <>Mingwei Samuel, Alvin Cheung, Joseph M. Hellerstein</>,
    description: [
      <>In this paper we present our ongoing work on Hydroflow, a new cloud programming model used to create constructively correct distributed systems. The model is a refinement and unification of the existing dataflow and reactive programming models.</>,
      <>Hydroflow is primarily a low-level compilation target for future declarative cloud programming languages, but developers can use it directly to precisely control program execution or fine-tune and debug compiled programs.</>
    ],
    conf: "UC Berkeley Technical Report",
    links: <><Link href="pathname:///papers/hydroflow-thesis.pdf">PDF</Link> / <Link href="https://github.com/hydro-project/hydro">GitHub</Link></>
  },
  {
    title: "New Directions in Cloud Programming",
    pdf: "pathname:///papers/new-directions.pdf",
    thumb: require("./img/papers/new-directions.png"),
    authors: <>Alvin Cheung, Natacha Crooks, Joseph M. Hellerstein, Mae Milano</>,
    description: [
      <>In this paper we lay out an agenda for a new generation of cloud programming research aimed at bringing research ideas to programmers in an evolutionary fashion. Key to our approach is a separation of distributed programs into a PACT of four facets: Program semantics, Availablity, Consistency and Targets of optimization.</>,
      <>Our agenda raises numerous research challenges across multiple areas including language design, query optimization, transactions, distributed consistency, compilers and program synthesis.</>
    ],
    conf: "CIDR 2021",
    links: <><Link href="pathname:///papers/new-directions.pdf">PDF</Link> / <Link href="https://arxiv.org/abs/2101.01159">arXiv</Link></>
  }
];

const linkIcon = (
  <svg xmlns="http://www.w3.org/2000/svg" fill="#ffffff" height="75" viewBox="0 0 24 24" width="75" style={{
    position: "absolute",
    left: "50%",
    top: "50%",
    transform: "translate(-50%, -50%)",
  }}>
    <path d="M0 0h24v24H0z" fill="none" />
    <path d="M3.9 12c0-1.71 1.39-3.1 3.1-3.1h4V7H7c-2.76 0-5 2.24-5 5s2.24 5 5 5h4v-1.9H7c-1.71 0-3.1-1.39-3.1-3.1zM8 13h8v-2H8v2zm9-6h-4v1.9h4c1.71 0 3.1 1.39 3.1 3.1s-1.39 3.1-3.1 3.1h-4V17h4c2.76 0 5-2.24 5-5s-2.24-5-5-5z" />
  </svg>
);

export default function Home() {
  return (
    <Layout
      description="Recent publications from the Hydro research group">
      <main>
        <Head>
          <title>Publications | Hydro</title>
          <meta property="og:title" content="Publications | Hydro" />
        </Head>
        <div style={{
          maxWidth: "calc(min(1100px, 100vw - 60px))",
          marginLeft: "auto",
          marginRight: "auto",
          marginTop: "30px",
          marginBottom: "30px"
        }}>
          <h1 className={styles["heading"]}>Latest Publications</h1>
          {papers.map((paper, i) => {
            return <div style={{
              marginTop: i > 0 ? "25px" : undefined,
            }} key={i}>
              <div className={styles["paperContainer"]}>
                <Link href={paper.pdf} className={styles["paper-thumb"]} style={{
                  display: "block",
                  marginRight: "20px",
                  position: "relative",
                  flexShrink: 0,
                }}>
                  <div className={styles["cardIcon"]}>{linkIcon}</div>
                  <Image
                    img={paper.thumb}
                    width={225}
                    placeholder="blur"
                    alt=""
                    style={{
                      position: "static",
                      background: "white",
                      width: "225px",
                      height: "auto",
                      display: "block",
                      borderRadius: "10px",
                      border: "1px solid #0a0a0a",
                      overflow: "hidden"
                    }}
                  />
                </Link>
                <div>
                  <b style={{
                    fontFamily: "'Inter', sans-serif"
                  }}>{paper.conf}</b>
                  <p style={{
                    margin: 0,
                    fontSize: "26px",
                    fontWeight: "600",
                    lineHeight: "125%",
                    marginBottom: "5px"
                  }}>
                    <Link href={paper.pdf} style={{
                      color: "inherit",
                      textDecoration: "none"
                    }}>{paper["title"]}</Link>
                  </p>
                  <p style={{ margin: 0, fontSize: "20px", fontWeight: 300, lineHeight: "130%" }}>{paper["authors"]}</p>
                  <p style={{ margin: 0, marginTop: "5px", fontSize: "16px", fontWeight: 300, lineHeight: "1.4" }}>{paper.description[0]} <span className={styles["paper-desc-extended"]}>{paper.description[1]}</span></p>
                  <p style={{ margin: 0, marginTop: "5px", fontSize: "20px" }}>{paper.links}</p>
                </div>
              </div>
            </div>;
          })}
        </div>
      </main>
    </Layout>
  );
}
