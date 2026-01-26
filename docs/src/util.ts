/// Grabs the specified lines from the string.
///
/// Can specify a line number range `lineStart, lineEnd`, a specific line number `lineNumber` (no
/// second arg), or a section name.
///
/// Lines are one-indexed (start with `1`). Both `lineStart` and `lineEnd` are inclusive.
///
/// Sections are marked in the code with a start tag `//[mysection]//` and and end tag
/// `//[/mysection]//`. The rest of these tag lines must be whitespace, and these tag lines are not
/// included in the output. However they are included for line number _counting_.
///
/// An optional `expected` parameter can be provided as the last argument. If provided, the function
/// will assert that the extracted content (after trimming whitespace) matches the expected string
/// (also trimmed). This allows inlining code snippets in docs while ensuring they stay in sync
/// with the source file.
export function getLines(
  str: string,
  sectionName: string,
  expected?: string,
): string;
export function getLines(
  str: string,
  lineStart: number,
  lineEnd?: number,
  expected?: string,
): string;
export function getLines(
  str: string,
  lineStartOrSectionName: number | string,
  lineEndOrExpected?: number | string,
  expected?: string,
): string {
  // `//[section]//` or `//[/section]//` (rest of line must be whitespace).
  const SECTION_REGEX = /^\s*\/\/\[(\/?)(\S+)\]\/\/\s*$/;
  let lines;
  let lineEnd: number | undefined;
  let expectedContent: string | undefined;

  if ("string" === typeof lineStartOrSectionName) {
    // Section name mode: getLines(str, sectionName, expected?)
    expectedContent = lineEndOrExpected as string | undefined;
    let inSection = false;
    lines = str.split("\n").filter((line) => {
      const match = SECTION_REGEX.exec(line);
      if (null == match) {
        return inSection;
      }
      const [_, end, name] = match;
      if (name == lineStartOrSectionName) {
        inSection = 0 === end.length;
      }
      return false;
    });
  } else {
    // Line number mode: getLines(str, lineStart, lineEnd?, expected?)
    if ("string" === typeof lineEndOrExpected) {
      // getLines(str, lineStart, expected) - no lineEnd provided
      expectedContent = lineEndOrExpected;
      lineEnd = undefined;
    } else {
      lineEnd = lineEndOrExpected;
      expectedContent = expected;
    }
    lines = str
      .split("\n")
      .slice(lineStartOrSectionName - 1, lineEnd || lineStartOrSectionName) // Select lines before removing section lines.
      .filter((line) => !SECTION_REGEX.test(line));
  }
  const leadingWhitespace = Math.min(
    ...lines
      .filter((line) => 0 !== line.length)
      .map((line) => line.search(/\S/))
      .map(Number),
  );
  if (0 < leadingWhitespace) {
    lines = lines.map((line) => line.slice(leadingWhitespace));
  }
  const result = lines.join("\n");

  // If expected content is provided, assert it matches
  if (expectedContent !== undefined) {
    // Normalize by stripping leading whitespace from each line (MDX can mess with indentation)
    const stripLeadingWhitespace = (s: string) =>
      s
        .trim()
        .split("\n")
        .map((line) => line.trimStart())
        .join("");
    const normalizedResult = stripLeadingWhitespace(result);
    const normalizedExpected = stripLeadingWhitespace(expectedContent);
    if (normalizedResult !== normalizedExpected) {
      console.error("getLines assertion failed!");
      console.error("Expected:\n", normalizedExpected);
      console.error("Got:\n", normalizedResult);
      throw new Error(
        `getLines content mismatch. The inlined code snippet does not match the source file. ` +
          `Update the expected string or check if the source file has changed.`,
      );
    }
  }

  return result;
}

/// Adds `// highlight-next-line` annotations to the specified lines.
export function highlightLines(code: string, lines: number[]): string {
  return code
    .split("\n")
    .map((line, i) => {
      if (lines.includes(i + 1)) {
        return `// highlight-next-line\n${line}`;
      }
      return line;
    })
    .join("\n");
}

/// Extract the output from the stdout snapshots created by `surface_examples.rs`.
///
/// This hides the graph output. Use `extractMermaid` to extract the graph output.
///
/// If `short` is false (default), will include code to show the `cargo run` console call.
/// If `short` is true, returns only the stdout output.
export function extractOutput(output: string, short = false): string {
  const outputLines = output.replace(/\n$/, "").split("\n");
  // Delete the first four lines, which are the snapshot front matter.
  outputLines.splice(0, 4);
  // Mermaid graph starts with double-percent signs.
  if (outputLines[0].startsWith("%%")) {
    // Continues until double newline (a blank line).
    const count = outputLines.findIndex((line) => 0 === line.length);
    // Hide mermaid output.
    outputLines.splice(0, count + 1, "<graph output>");
  }
  const stdOut = outputLines.join("\n");
  if (short) {
    return stdOut;
  }
  return `#shell-command-next-line
cargo run
<build output>
${stdOut}`;
}

/// Extract the mermaid graph logged to stdout from the snapshots created by `surface_examples.rs`.
export function extractMermaid(output: string): string {
  const outputLines = output.split("\n");
  // Delete the first four lines, which are the snapshot front matter.
  outputLines.splice(0, 4);
  // Mermaid graph starts with double-percent signs.
  if (!outputLines[0].startsWith("%%")) {
    console.error("Snapshot output may be missing mermaid graph.");
  }
  // Continues until double newline (a blank line).
  const count = outputLines.findIndex((line) => 0 === line.length);
  outputLines.length = count;
  return outputLines.join("\n");
}
