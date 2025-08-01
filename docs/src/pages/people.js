import React from "react";
import Link from "@docusaurus/Link";
import Layout from "@theme/Layout";
import Image from "@theme/IdealImage";

import akcheung from "./people-img/akcheung.jpeg";
import conor from "./people-img/conor.jpeg";
import david from "./people-img/david.jpeg";
import jmh from "./people-img/jmh.jpeg";
import mae from "./people-img/mae.png";
import mingwei from "./people-img/mingwei.jpeg";
import natacha from "./people-img/natacha.jpeg";
import shadaj from "./people-img/shadaj.png";
import lucky from "./people-img/lucky.jpeg";
import rohit from "./people-img/rohit.jpg";
import hydroTurtle from "../../static/img/hydro-turtle.png";

import styles from "./people.module.css";
import Head from "@docusaurus/Head";

const PersonCard = (props) => {
  return (
    <Link
      href={props.url}
      className={styles["personCard"]}
    >
      <div className={styles["personContainer"]}>
        <Image
          className={styles["personImage"]}
          img={props.img}
          style={{
            width: "100%",
          }}
        />
        <div style={{ marginLeft: "15px", marginRight: "15px" }}>
          <p
            style={{
              display: "block",
              fontSize: "1.1em",
              lineHeight: "1.25em",
              fontWeight: 700,
              marginTop: "10px",
              marginBottom: "5px",
            }}
          >
            {props.name}
          </p>
          <p
            style={{
              display: "block",
              fontSize: "0.98em",
              fontWeight: 500,
              marginTop: 0,
              marginBottom: "5px",
              lineHeight: "1.25em",
            }}
          >
            {props.role}
          </p>
        </div>
      </div>
    </Link>
  );
};

export default function Home() {
  return (
    <Layout description="Members of the Hydro research group">
      <Head>
        <title>Hydro Team | Hydro</title>
        <meta property="og:title" content="Hydro Team | Hydro" />
      </Head>
      <main>
        <div className={styles["container"]}>
          <h1 className={styles["title"]}>Hydro Team</h1>
          <p className={styles["blurb"]}>The Hydro framework began as a research project at UC Berkeley led by Joe Hellerstein, continuing a long line of work applying insights from database research to distributed systems. In 2025, several graduates of the Hydro group joined AWS to continue its development in a production environment. The core team, which leads the technical direction for Hydro, consists of researchers and engineers at UC Berkeley and AWS. We also collaborate with leading researchers at UC Berkeley and Princeton University.</p>
          <div>
            <div className={styles["subtitle"]}>Core Team</div>
            <div className={styles["personGroup"]}>
              <PersonCard
                name={"Joe Hellerstein"}
                role={"Professor, UC Berkeley"}
                url={"https://dsf.berkeley.edu/jmh"}
                img={jmh}
              ></PersonCard>
              <PersonCard
                name={"David Chu"}
                role={"PhD Student, UC Berkeley"}
                url={
                  "https://github.com/davidchuyaya/portfolio/blob/master/README.md"
                }
                img={david}
              ></PersonCard>
              <PersonCard
                name={"Lucky Katahanas"}
                role={"Engineer, AWS"}
                url={"https://www.linkedin.com/in/luckykatahanas/"}
                img={lucky}
              ></PersonCard>
              <PersonCard
                name={"Shadaj Laddad"}
                role={"Scientist, AWS"}
                url={"https://www.shadaj.me"}
                img={shadaj}
              ></PersonCard>
              <PersonCard
                name={"Conor Power"}
                role={"Scientist, UC Berkeley"}
                url={"https://www.linkedin.com/in/conorpower23"}
                img={conor}
              ></PersonCard>
              <PersonCard
                name={"Mingwei Samuel"}
                role={"Engineer, AWS"}
                url={"https://github.com/MingweiSamuel"}
                img={mingwei}
              ></PersonCard>
            </div>

            <div className={styles["subtitle"]}>
              Academic Collaborators
            </div>
            <div className={styles["personGroup"]}>
              <PersonCard
                name={"Alvin Cheung"}
                role={"Professor, UC Berkeley"}
                url={"https://people.eecs.berkeley.edu/~akcheung"}
                img={akcheung}
              ></PersonCard>
              <PersonCard
                name={"Natacha Crooks"}
                role={"Professor, UC Berkeley"}
                url={"https://nacrooks.github.io"}
                img={natacha}
              ></PersonCard>
              
              <PersonCard
                name={"Mae Milano"}
                role={"Professor, Princeton University"}
                url={"https://www.languagesforsyste.ms"}
                img={mae}
              ></PersonCard>
              <PersonCard
                name={"Chris Douglas"}
                role={"PhD Student, UC Berkeley"}
                url={"https://www.linkedin.com/in/chris-douglas-73333a1"}
                img={hydroTurtle}
              ></PersonCard>
              
            </div>

            <div className={styles["subtitle"]}>Alumni</div>
            <div className={styles["personGroup"]}>
              <PersonCard
                name={"Tiemo Bang"}
                role={"Postdoc, UC Berkeley"}
                url={
                  "https://scholar.google.com/citations?user=HDK0KRYAAAAJ&hl=en"
                }
                img={hydroTurtle}
              ></PersonCard>
              <PersonCard
                name={"Rohit Kulshreshtha"}
                role={"Research Engineer"}
                url={"https://www.linkedin.com/in/rohitkul/"}
                img={rohit}
              ></PersonCard>
              <PersonCard
                name={"Justin Jaffray"}
                role={"Research Engineer"}
                url={"https://www.linkedin.com/in/justinjaffray/"}
                img={hydroTurtle}
              ></PersonCard>
            </div>
          </div>
        </div>
      </main>
    </Layout>
  );
}
