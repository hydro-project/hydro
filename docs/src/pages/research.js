import React from 'react';
import Link from '@docusaurus/Link';
import Layout from '@theme/Layout';
import Image from '@theme/IdealImage';

import styles from './research.module.css';

const papers = [
  {
    title: "Bigger, not Badder: Safely Scaling BFT Protocols",
    pdf: "pathname:///papers/david-papoc-2024.pdf",
    thumb: require("./img/papers/david-papoc-2024.png"),
    authors: <>David Chu, Chris Liu, Natacha Crooks, Joe Hellerstein, & Heidi Howard</>,
    description: [
      <>Byzantine Fault Tolerant (BFT) protocols provide powerful guarantees in the presence of arbitrary machine failures, yet they do not scale. The process of creating new, scalable BFT protocols requires expert analysis and is often error-prone. Recent work suggests that localized, rule-driven rewrites can be mechanically applied to scale existing (non-BFT) protocols, including Paxos. We modify these rewrites--- decoupling and partitioning---so they can be safely applied to BFT protocols, and apply these rewrites to the critical path of PBFT, improving its throughput by 5x. We prove the correctness of the modified rewrites on any BFT protocol by formally modeling the arbitrary logic of a Byzantine node. We define the Borgesian simulator, a theoretical node that simulates a Byzantine node through randomness, and show that in any BFT protocol, the messages that a Borgesian simulator can generate before and after optimization is the same. Our initial results point the way towards an automatic optimizer for BFT protocols.</>
    ],
    conf: "PaPoC 2024",
    links: <><Link href="pathname:///papers/david-papoc-2024.pdf">PDF</Link> / <Link href="https://github.com/rithvikp/autocomp">GitHub</Link></>
  },
  {
    title: "Optimizing Distributed Protocols with Query Rewrites",
    pdf: "pathname:///papers/david-sigmod-2024.pdf",
    thumb: require("./img/papers/david-sigmod-2024.png"),
    authors: <>David Chu, Rithvik Panchapakesan, Shadaj Laddad, Lucky Katahanas, Chris Liu, Kaushik Shivakumar, Natacha Crooks, Joe Hellerstein, & Heidi Howard</>,
    description: [
      <>Distributed protocols such as 2PC and Paxos lie at the core of many systems in the cloud, but standard implementations do not scale. New scalable distributed protocols are developed through careful analysis and rewrites, but this process is ad hoc and error-prone. This paper presents an approach for scaling any distributed protocol by applying rule-driven rewrites, borrowing from query optimization. Distributed protocol rewrites entail a new burden: reasoning about spatiotemporal correctness. We leverage order-insensitivity and data dependency analysis to systematically identify correct coordination-free scaling opportunities. We apply this analysis to create preconditions and mechanisms for coordination-free decoupling and partitioning, two fundamental vertical and horizontal scaling techniques. Manual rule-driven applications of decoupling and partitioning improve the throughput of 2PC by 5x and Paxos by 3x, and match state-of-the-art throughput in recent work. These results point the way toward automated optimizers for distributed protocols based on correct-by-construction rewrite rules.</>
    ],
    conf: "SIGMOD 2024",
    links: <><Link href="pathname:///papers/david-sigmod-2024.pdf">PDF</Link> / <Link href="https://arxiv.org/abs/2404.01593">Tech Report</Link> / <Link href="https://github.com/rithvikp/autocomp">GitHub</Link></>
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
    links: <><Link href="pathname:///papers/hydroflow-thesis.pdf">PDF</Link> / <Link href="https://github.com/hydro-project/hydroflow">GitHub</Link></>
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
    <path d="M0 0h24v24H0z" fill="none"/>
    <path d="M3.9 12c0-1.71 1.39-3.1 3.1-3.1h4V7H7c-2.76 0-5 2.24-5 5s2.24 5 5 5h4v-1.9H7c-1.71 0-3.1-1.39-3.1-3.1zM8 13h8v-2H8v2zm9-6h-4v1.9h4c1.71 0 3.1 1.39 3.1 3.1s-1.39 3.1-3.1 3.1h-4V17h4c2.76 0 5-2.24 5-5s-2.24-5-5-5z"/>
  </svg>
);

export default function Home() {
  return (
    <Layout
      description="Recent publications from the Hydro research group">
      <main>
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
                  <p style={{ margin: 0, fontSize: "26px", fontWeight: "600" }}>
                    <Link href={paper.pdf} style={{
                      color: "inherit",
                      textDecoration: "none"
                    }}>{paper["title"]}</Link>
                  </p>
                  <p style={{ margin: 0, fontSize: "20px", fontWeight: 300, lineHeight: "130%" }}>{paper["authors"]}</p>
                  <p style={{ margin: 0, marginTop: "5px", fontSize: "16px", fontWeight: 300, lineHeight: "1.4" }}>{paper.description[0]} <span className="paper-desc-extended">{paper.description[1]}</span></p>
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
