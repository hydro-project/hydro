/**
 * Map a doc's `metadata.source` (e.g. "@site/docs/hydro/learn/quickstart.mdx")
 * to the URL where raw-docs-plugin serves the raw file
 * (e.g. "/raw/docs/hydro/learn/quickstart.mdx").
 *
 * Returns null for sources outside the docs content directory.
 */
export function rawSourceUrl(source: string): string | null {
  const match = source.match(/^@site\/(docs\/.+\.mdx?)$/);
  return match ? `/raw/${match[1]}` : null;
}
