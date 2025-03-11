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

const PersonCard = (props) => {
  return (
    <Link
      href={props.url}
      style={{
        color: "inherit",
        textDecoration: "inherit",
        margin: "15px",
      }}
    >
      <div className={styles["personContainer"]}>
        <Image
          className={styles["personImage"]}
          img={props.img}
          style={{
            width: "250px",
            height: "250px",
          }}
        />
        <div style={{ marginLeft: "15px" }}>
          <p
            style={{
              display: "block",
              fontSize: "1.1em",
              fontWeight: 700,
              marginTop: "5px",
              marginBottom: 0,
            }}
          >
            {props.name}
          </p>
          <i
            style={{
              display: "block",
              fontSize: "1.1em",
              fontWeight: 400,
              marginTop: 0,
              marginBottom: "5px",
            }}
          >
            {props.role}
          </i>
        </div>
      </div>
    </Link>
  );
};

export default function Home() {
  return (
    <Layout description="Members of the Hydro research group">
      <main>
        <div
          style={{
            maxWidth: "calc(min(1100px, 100vw - 60px))",
            marginLeft: "auto",
            marginRight: "auto",
            marginTop: "30px",
            marginBottom: "30px",
          }}
        >
          <h1
            style={{
              fontSize: "4rem",
              textAlign: "center",
            }}
          >
            About Us
          </h1>
          <div
            style={{
              maxWidth: "1000px",
              marginLeft: "auto",
              marginRight: "auto",
            }}
          >
            <div className={styles["subtitle"]}>Faculty</div>
            <div className={styles["personGroup"]}>
              <PersonCard
                name={"Alvin Cheung"}
                role={"Faculty"}
                url={"https://people.eecs.berkeley.edu/~akcheung"}
                img={akcheung}
              ></PersonCard>
              <PersonCard
                name={"Natacha Crooks"}
                role={"Faculty"}
                url={"https://nacrooks.github.io"}
                img={natacha}
              ></PersonCard>
              <PersonCard
                name={"Joe Hellerstein"}
                role={"Faculty"}
                url={"https://dsf.berkeley.edu/jmh"}
                img={jmh}
              ></PersonCard>
              <PersonCard
                name={"Mae Milano"}
                role={"Faculty"}
                url={"http://www.languagesforsyste.ms"}
                img={mae}
              ></PersonCard>
            </div>

            <div className={styles["subtitle"]}>
              Graduate Students & Research Engineers
            </div>
            <div className={styles["personGroup"]}>
              <PersonCard
                name={"David Chu"}
                role={"PhD Student"}
                url={
                  "https://github.com/davidchuyaya/portfolio/blob/master/README.md"
                }
                img={david}
              ></PersonCard>
              <PersonCard
                name={"Chris Douglas"}
                role={"PhD Student"}
                url={"https://www.linkedin.com/in/chris-douglas-73333a1"}
                img={hydroTurtle}
              ></PersonCard>
              <PersonCard
                name={"Shadaj Laddad"}
                role={"PhD Student"}
                url={"https://www.shadaj.me"}
                img={shadaj}
              ></PersonCard>
              <PersonCard
                name={"Conor Power"}
                role={"PhD Student"}
                url={"https://www.linkedin.com/in/conorpower23"}
                img={conor}
              ></PersonCard>
              <PersonCard
                name={"Mingwei Samuel"}
                role={"Chief Research Engineer"}
                url={"https://github.com/MingweiSamuel"}
                img={mingwei}
              ></PersonCard>
              <PersonCard
                name={"Lucky Katahanas"}
                role={"Research Engineer"}
                url={"https://www.linkedin.com/in/luckykatahanas/"}
                img={lucky}
              ></PersonCard>
            </div>

            <div className={styles["subtitle"]}>Alumni</div>
            <div className={styles["personGroup"]}>
              <PersonCard
                name={"Tiemo Bang"}
                role={"Postdoc"}
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
