import DefaultAdmonitionTypes from "@theme-original/Admonition/Types";
import styles from "@docusaurus/theme-classic/src/theme/Admonition/Layout/styles.module.css";
import { ThemeClassNames } from "@docusaurus/theme-common";

import clsx from "clsx";

function LearnAdmonition(props) {
  const { children } = props;
  return (
    <div
      className={clsx(
        ThemeClassNames.common.admonition,
        ThemeClassNames.common.admonitionType("tip"),
        styles.admonition,
        "alert alert--success",
      )}
      style={{
        "--ifm-alert-background-color": "rgb(206 189 255 / 26%)",
        "--ifm-alert-background-color-highlight": "rgb(35 0 164 / 15%)",
        "--ifm-alert-border-left-width": "0px",
        "--ifm-list-left-padding": "1.75rem",
      }}
    >
      <h3
        style={{
          fontFamily: "inherit",
          marginBottom: "10px",
          borderTop: "none",
          paddingTop: "0px",
        }}
      >
        You will learn
      </h3>
      <div
        className={styles.admonitionContent}
        style={{
          fontSize: "1.05em",
        }}
      >
        {children}
      </div>
    </div>
  );
}

const AdmonitionTypes = {
  ...DefaultAdmonitionTypes,

  learn: LearnAdmonition,
};

export default AdmonitionTypes;
