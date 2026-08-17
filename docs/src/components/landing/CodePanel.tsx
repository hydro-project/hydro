/**
 * CodePanel: a syntax-highlighted code panel for the landing page with
 * support for
 *   - semantic "paints" (color-coding tokens to match diagram locations
 *     and network channels),
 *   - dynamic line highlighting (synced to the pinned graph animation),
 *   - static line highlighting,
 *   - an inline compiler-style error with a red squiggle.
 */

import React from "react";
import { Highlight, themes } from "prism-react-renderer";
import { useColorMode } from "@docusaurus/theme-common";

import styles from "./landing.module.css";

export interface PaintRule {
  /** Substring to colorize (matched within individual tokens). */
  match: string;
  color?: string;
  /** Restrict the rule to a single (1-based) line. */
  line?: number;
  bold?: boolean;
  squiggle?: boolean;
}

export interface ErrorSpec {
  line: number;
  match: string;
  title: string;
  notes: string[];
}

/**
 * Split a token's text so painted/squiggled substrings get their own spans.
 * `rules` is a list of { match, color?, squiggle? }.
 */
function renderTokenContent(
  content: string,
  rules: PaintRule[],
  keyBase: string,
): React.ReactNode {
  const applicable = rules.filter((r) => content.includes(r.match));
  if (applicable.length === 0) return content;

  // Apply the first matching rule (rules rarely overlap in practice).
  const rule = applicable[0];
  const rest = applicable.slice(1);
  const parts = content.split(rule.match);
  const out: React.ReactNode[] = [];
  parts.forEach((part, i) => {
    if (i > 0) {
      out.push(
        <span
          key={`${keyBase}-m${i}`}
          className={rule.squiggle ? styles.errSquiggle : undefined}
          style={
            rule.color
              ? { color: rule.color, fontWeight: rule.bold ? 800 : 600 }
              : undefined
          }
        >
          {rule.match}
        </span>,
      );
    }
    if (part) out.push(renderTokenContent(part, rest, `${keyBase}-p${i}`));
  });
  return out;
}

export default function CodePanel({
  title,
  accentColor,
  code,
  language = "rust",
  paints = [],
  activeLines = [],
  staticLines = [],
  flashLines = [],
  flashKey = 0,
  error = null,
}: {
  title?: string;
  accentColor?: string;
  code: string;
  language?: string;
  paints?: PaintRule[];
  activeLines?: number[];
  staticLines?: number[];
  flashLines?: number[];
  flashKey?: number;
  error?: ErrorSpec | null;
}) {
  const { colorMode } = useColorMode();
  const theme = colorMode === "dark" ? themes.vsDark : themes.github;

  return (
    <div
      className={`${styles.codePanel} ${
        accentColor ? styles.codePanelAccented : ""
      }`}
      style={accentColor ? { borderLeftColor: accentColor } : undefined}
    >
      {title && <div className={styles.codeTitle}>{title}</div>}
      <Highlight theme={theme} code={code.trim()} language={language}>
        {({ tokens, getLineProps, getTokenProps }) => (
          <pre className={styles.codePre}>
            {tokens.map((line, i) => {
              const lineNo = i + 1;
              const lineRules: PaintRule[] = [
                ...paints.filter((p) => !p.line || p.line === lineNo),
                ...(error && error.line === lineNo
                  ? [{ match: error.match, squiggle: true }]
                  : []),
              ];
              const lineProps = getLineProps({ line });
              const isFlash = flashLines.includes(lineNo);
              const highlightClass = isFlash
                ? styles.codeLineFlash
                : activeLines.includes(lineNo)
                  ? styles.codeLineActive
                  : staticLines.includes(lineNo)
                    ? styles.codeLineStatic
                    : "";
              return (
                <React.Fragment
                  key={isFlash ? `${i}-flash-${flashKey}` : `${i}`}
                >
                  <div
                    {...lineProps}
                    className={`${lineProps.className} ${styles.codeLine} ${highlightClass}`}
                  >
                    {line.map((token, j) => {
                      const tokenProps = getTokenProps({ token });
                      // The default themes paint function calls red, which
                      // is distracting; render them in the plain foreground
                      // color instead.
                      const style = token.types.includes("function")
                        ? { ...tokenProps.style, color: "inherit" }
                        : tokenProps.style;
                      return (
                        <span
                          key={j}
                          className={tokenProps.className}
                          style={style}
                        >
                          {renderTokenContent(
                            token.content,
                            lineRules,
                            `t${i}-${j}`,
                          )}
                        </span>
                      );
                    })}
                  </div>
                  {error && error.line === lineNo && (
                    <div className={styles.errCallout}>
                      <span className={styles.errCalloutTitle}>
                        error: {error.title}
                      </span>
                      {error.notes.map((note, k) => (
                        <div key={k} className={styles.errCalloutBody}>
                          {"   = "}
                          {note}
                        </div>
                      ))}
                    </div>
                  )}
                </React.Fragment>
              );
            })}
          </pre>
        )}
      </Highlight>
    </div>
  );
}
