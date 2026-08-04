// @ts-check
const fs = require('fs');
const path = require('path');

/**
 * Serves the raw `.md`/`.mdx` doc sources at `/raw/docs/...`, mirroring the
 * `docs/` content directory. Used by the "Copy as Markdown" button.
 *
 * - In dev, the content directory is served directly by webpack-dev-server
 *   (always fresh, no copying).
 * - In production builds, sources are copied into the build output.
 */

/** URL prefix under which raw sources are served. */
const RAW_URL_PREFIX = '/raw/docs';

/** Recursively copy only .md/.mdx files, preserving directory structure. */
function copyMarkdownSources(srcDir, destDir) {
  for (const entry of fs.readdirSync(srcDir, {withFileTypes: true})) {
    const srcPath = path.join(srcDir, entry.name);
    const destPath = path.join(destDir, entry.name);
    if (entry.isDirectory()) {
      copyMarkdownSources(srcPath, destPath);
    } else if (/\.mdx?$/.test(entry.name)) {
      fs.mkdirSync(path.dirname(destPath), {recursive: true});
      fs.copyFileSync(srcPath, destPath);
    }
  }
}

/** @returns {import('@docusaurus/types').Plugin} */
module.exports = function rawDocsPlugin(context) {
  const docsDir = path.join(context.siteDir, 'docs');
  return {
    name: 'raw-docs-plugin',

    configureWebpack() {
      return {
        devServer: {
          static: [{directory: docsDir, publicPath: RAW_URL_PREFIX}],
        },
      };
    },

    postBuild({outDir}) {
      copyMarkdownSources(docsDir, path.join(outDir, RAW_URL_PREFIX));
    },
  };
};
