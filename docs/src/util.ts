/// Grabs the specified lines `[lineStart, lineEnd]` from the string.
///
/// Lines are one-indexed (start with `1`). Both `lineStart` and `lineEnd` are inclusive.
export function getLines(str: string, lineStart: number, lineEnd?: number): string {
    let lines = str.split('\n').slice(lineStart - 1, lineEnd || lineStart);
    const leadingWhitespace = Math.min(...lines.filter(line => 0 !== line.length).map(line => line.search(/\S/)).map(Number));
    if (0 < leadingWhitespace) {
        lines = lines.map(line => line.slice(leadingWhitespace));
    }
    return lines.join('\n');
}

/// Extract the output from the stdout snapshots created by `surface_examples.rs`.
///
/// This hides the graph output. Use `extractMermaid` to extract the graph output.
///
/// If `short` is false (default), will include code to show the `cargo run` console call.
/// If `short` is true, returns only the stdout output.
export function extractOutput(output: string, short = false): string {
    const outputLines = output.replace(/\n$/, '').split('\n');
    // Delete the first four lines, which are the snapshot front matter.
    outputLines.splice(0, 4);
    // Mermaid graph starts with double-percent signs.
    if (outputLines[0].startsWith('%%')) {
        // Continues until double newline (a blank line).
        const count = outputLines.findIndex(line => 0 === line.length);
        // Hide mermaid output.
        outputLines.splice(0, count + 1, '<graph output>');
    }
    const stdOut = outputLines.join('\n');
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
    const outputLines = output.split('\n');
     // Delete the first four lines, which are the snapshot front matter.
    outputLines.splice(0, 4);
    // Mermaid graph starts with double-percent signs.
    if (!outputLines[0].startsWith('%%')) {
        console.error('Snapshot output may be missing mermaid graph.');
    }
    // Continues until double newline (a blank line).
    const count = outputLines.findIndex(line => 0 === line.length);
    outputLines.length = count;
    return outputLines.join('\n');
}