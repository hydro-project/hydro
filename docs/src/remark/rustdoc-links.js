// @ts-check
// Remark plugin that resolves `rust:` pseudo-links to rustdoc pages.
//
// Usage in Markdown/MDX:
//
//   [`Stream`](rust:hydro_lang::live_collections::Stream)
//   [`Stream::fold`](rust:hydro_lang::live_collections::Stream::fold)
//   [`source_iter`](rust:hydro_lang::location::Location::source_iter)
//   [the `live_collections` module](rust:hydro_lang::live_collections)
//   [`TotalOrder`](rust:enum@hydro_lang::live_collections::stream::TotalOrder)
//   [`setup!`](rust:hydro_lang::setup!)
//
// Paths are resolved against the compiled rustdoc HTML in `static/rustdoc`
// (populated by `cargo doc` — see `build_docs.bash`). The item kind
// (struct/enum/trait/fn/...) is discovered automatically by looking at the
// generated files, so you never write `struct.Foo.html` paths by hand, and
// every link (including `Type::method` anchors) is validated against the
// rustdoc output at build time. Paths must spell out the full module path of
// the page being linked; rustdoc redirect pages (generated for re-exports)
// are followed to their destination. In the rare case of a same-name
// collision within one module, disambiguate rustdoc-style with a `kind@`
// prefix (struct@, enum@, trait@, fn@, macro@, mod@, ...) or a trailing
// `()` / `!`.
//
// If `static/rustdoc` does not exist (the common case in local dev), links
// degrade to rustdoc search URLs (`.../index.html?search=Name`) with a
// warning. In production builds (`NODE_ENV=production`), a missing rustdoc
// directory or an unresolvable link fails the build.

const fs = require("fs");
const path = require("path");

const RUSTDOC_ROOT = path.resolve(__dirname, "..", "..", "static", "rustdoc");
const URL_PREFIX = "pathname:///rustdoc/";
const SCHEME = "rust:";

/** File-name prefixes rustdoc uses for item pages, e.g. `struct.Stream.html`. */
const ITEM_FILE_KINDS = [
  "struct",
  "enum",
  "trait",
  "traitalias",
  "union",
  "fn",
  "macro",
  "attr",
  "derive",
  "constant",
  "static",
  "type",
  "primitive",
];

// `kind@` prefix disambiguators -> allowed file kinds. Mirrors rustdoc's own
// intra-doc-link disambiguators:
// https://github.com/rust-lang/rust/blob/e7408fbec/src/librustdoc/passes/collect_intra_doc_links.rs#L1755-L1776
// (`field@`/`variant@` are omitted: those are anchors on a parent page here,
// found automatically from the `Type::name` form.)
const KIND_ALIASES = {
  struct: ["struct"],
  enum: ["enum"],
  trait: ["trait"],
  union: ["union"],
  module: ["mod"],
  mod: ["mod"],
  const: ["constant"],
  constant: ["constant"],
  static: ["static"],
  function: ["fn"],
  fn: ["fn"],
  method: ["fn"],
  derive: ["derive"],
  // Namespace disambiguators.
  type: ["struct", "enum", "trait", "traitalias", "union", "type", "primitive", "mod"],
  value: ["fn", "constant", "static"],
  macro: ["macro", "attr", "derive"],
  prim: ["primitive"],
  primitive: ["primitive"],
  tyalias: ["type"],
  typealias: ["type"],
};

// Trailing suffix disambiguators, also mirroring rustdoc (`foo!()` etc.).
const SUFFIX_KINDS = [
  ["!()", ["macro"]],
  ["!{}", ["macro"]],
  ["![]", ["macro"]],
  ["()", ["fn"]],
  ["!", ["macro"]],
];

/** Anchor prefixes rustdoc uses for associated items on an item page. */
const ANCHOR_KINDS = [
  "method",
  "tymethod",
  "associatedtype",
  "associatedconstant",
  "variant",
  "structfield",
];

const IDENT_RE = /^[A-Za-z_][A-Za-z0-9_]*$/;

class RustdocLinkError extends Error {}

// ---------------------------------------------------------------------------
// Filesystem helpers (cached; rustdoc output is immutable during a build)
// ---------------------------------------------------------------------------

/** @type {Map<string, string[] | null>} dir -> entries (null if not a dir) */
const dirCache = new Map();
/** @type {Map<string, string>} html file -> contents */
const htmlCache = new Map();
/** @type {Map<string, Set<string>>} html file -> set of `id="..."` anchors */
const anchorCache = new Map();

function listDir(dir) {
  let entries = dirCache.get(dir);
  if (entries === undefined) {
    try {
      entries = fs.readdirSync(dir);
    } catch {
      entries = null;
    }
    dirCache.set(dir, entries);
  }
  return entries;
}

function isDir(dir) {
  return listDir(dir) !== null;
}

function htmlOf(file) {
  let html = htmlCache.get(file);
  if (html === undefined) {
    try {
      html = fs.readFileSync(file, "utf8");
    } catch {
      html = "";
    }
    htmlCache.set(file, html);
  }
  return html;
}

/**
 * Rustdoc generates `<meta http-equiv="refresh">` stub pages for some
 * re-exports; follow them to the canonical page so emitted URLs are stable.
 */
function followRedirect(file) {
  for (let hops = 0; hops < 5; hops++) {
    const match = htmlOf(file)
      .slice(0, 1024)
      .match(/http-equiv="refresh"[^>]*content="\d+;\s*URL=([^"]+)"/i);
    if (!match) return file;
    const target = path.resolve(path.dirname(file), decodeURIComponent(match[1]));
    if (!fs.existsSync(target)) return file;
    file = target;
  }
  return file;
}

function anchorsOf(file) {
  let anchors = anchorCache.get(file);
  if (anchors === undefined) {
    anchors = new Set();
    for (const match of htmlOf(file).matchAll(/\bid="([^"]+)"/g)) {
      anchors.add(match[1]);
    }
    anchorCache.set(file, anchors);
  }
  return anchors;
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

/**
 * Find item pages named `<kind>.<name>.html` in `dir`.
 * @returns {string[]} matching absolute file paths
 */
function itemFilesIn(dir, name, allowedKinds) {
  const entries = listDir(dir) || [];
  const kinds = allowedKinds || ITEM_FILE_KINDS;
  return entries
    .filter((entry) => {
      if (!entry.endsWith(`.${name}.html`)) return false;
      const kind = entry.slice(0, entry.length - `.${name}.html`.length);
      return kinds.includes(kind);
    })
    .map((entry) => path.join(dir, entry));
}

/**
 * Find the anchor for associated item `name` on the page `file`.
 * @returns {string | null}
 */
function findAnchor(file, name) {
  const anchors = anchorsOf(file);
  for (const kind of ANCHOR_KINDS) {
    if (anchors.has(`${kind}.${name}`)) return `${kind}.${name}`;
  }
  return null;
}

/**
 * Interpret `segments` as an exact module path within `dir`, where the
 * trailing segments may be a module, an item, or an item plus an associated
 * item (`Type::method`).
 *
 * @returns {{file: string, anchor?: string}[]}
 */
function resolveIn(dir, segments, allowedKinds) {
  const results = [];
  if (segments.length === 0) {
    if (
      (!allowedKinds || allowedKinds.includes("mod")) &&
      (listDir(dir) || []).includes("index.html")
    ) {
      results.push({ file: path.join(dir, "index.html") });
    }
    return results;
  }
  const [seg, ...rest] = segments;
  // Descend into a submodule directory.
  if (rest.length > 0 || !allowedKinds || allowedKinds.includes("mod")) {
    if (isDir(path.join(dir, seg))) {
      results.push(...resolveIn(path.join(dir, seg), rest, allowedKinds));
    }
  }
  if (rest.length === 0) {
    // An item page in this directory.
    for (const file of itemFilesIn(dir, seg, allowedKinds)) {
      results.push({ file: followRedirect(file) });
    }
  } else if (rest.length === 1) {
    // An item page plus an associated item (`Type::method`). A kind
    // disambiguator refers to the container here (e.g. `trait@...::method`).
    for (const file of itemFilesIn(dir, seg, allowedKinds)) {
      const target = followRedirect(file);
      const anchor = findAnchor(target, rest[0]);
      if (anchor) results.push({ file: target, anchor });
    }
  }
  return results;
}

function toUrl(result, rustdocRoot) {
  const rel = path.relative(rustdocRoot, result.file).split(path.sep).join("/");
  return URL_PREFIX + rel + (result.anchor ? `#${result.anchor}` : "");
}

/**
 * Resolve a `rust:` link spec to a URL under `pathname:///rustdoc/`.
 * Throws {@link RustdocLinkError} if the spec is malformed, unresolvable, or
 * ambiguous.
 *
 * @param {string} spec e.g. `hydro_lang::live_collections::Stream::fold`
 * @param {string} rustdocRoot
 * @returns {string}
 */
function resolveRustdocLink(spec, rustdocRoot) {
  let rest = spec;
  let allowedKinds = null;
  const at = rest.indexOf("@");
  if (at !== -1) {
    const kind = rest.slice(0, at);
    rest = rest.slice(at + 1);
    allowedKinds = KIND_ALIASES[kind];
    if (!allowedKinds) {
      throw new RustdocLinkError(
        `unknown disambiguator \`${kind}@\` (expected one of: ${Object.keys(KIND_ALIASES).join(", ")})`
      );
    }
  }
  for (const [suffix, kinds] of SUFFIX_KINDS) {
    if (rest.endsWith(suffix)) {
      rest = rest.slice(0, rest.length - suffix.length);
      if (allowedKinds && !allowedKinds.some((k) => kinds.includes(k))) {
        throw new RustdocLinkError(
          `unmatched disambiguator prefix and suffix \`${suffix}\` in \`${spec}\``
        );
      }
      allowedKinds = kinds;
      break;
    }
  }
  const segments = rest.split("::");
  // All segments are Rust identifiers — crate names included: rustdoc output
  // directories use the crate name (always underscores), not the Cargo
  // package name, just like paths in Rust source.
  if (!segments.every((s) => IDENT_RE.test(s))) {
    throw new RustdocLinkError(
      `malformed path \`${rest}\` (expected \`crate::path::Item\` with Rust identifiers)`
    );
  }
  const [crate, ...itemPath] = segments;
  const crateDir = path.join(rustdocRoot, crate);
  if (!isDir(crateDir)) {
    throw new RustdocLinkError(
      `crate \`${crate}\` not found in compiled rustdoc (no directory ${crateDir})`
    );
  }

  const results = resolveIn(crateDir, itemPath, allowedKinds);
  if (results.length === 0) {
    throw new RustdocLinkError(
      `\`${spec}\` does not resolve to any documented item under ${crateDir}. ` +
        `Check the spelling and module path, or rebuild rustdoc if the item ` +
        `was recently added or moved.`
    );
  }
  // Multiple pages can share a module path and name (e.g. `fn.foo.html` and
  // `macro.foo.html`, or a submodule named like an item). Redirect-following
  // may collapse them to the same page; otherwise it's ambiguous.
  const urls = new Set(results.map((r) => toUrl(r, rustdocRoot)));
  if (urls.size > 1) {
    throw new RustdocLinkError(
      `\`${spec}\` is ambiguous; it matches:\n` +
        [...urls].map((u) => `  - ${u}`).join("\n") +
        `\nDisambiguate with a prefix (e.g. \`struct@...\`) or suffix (\`()\`, \`!\`).`
    );
  }
  return urls.values().next().value;
}

/** Dummy URL used when rustdoc has not been compiled (dev mode). */
function dummyUrl(spec) {
  const rest = spec.slice(spec.indexOf("@") + 1);
  const segments = rest.replace(/[!()[\]{}]+$/, "").split("::").filter(Boolean);
  const crate = segments[0] || "hydro_lang";
  const name = segments[segments.length - 1] || "";
  return segments.length > 1
    ? `${URL_PREFIX}${crate}/index.html?search=${encodeURIComponent(name)}`
    : `${URL_PREFIX}${crate}/index.html`;
}

// ---------------------------------------------------------------------------
// Remark plugin
// ---------------------------------------------------------------------------

let warnedMissingRustdoc = false;

/**
 * @param {{rustdocRoot?: string, strict?: boolean}} [options]
 */
function remarkRustdocLinks(options = {}) {
  const rustdocRoot = options.rustdocRoot || RUSTDOC_ROOT;
  const strict =
    options.strict !== undefined
      ? options.strict
      : process.env.NODE_ENV === "production";

  return (tree, file) => {
    const haveRustdoc = isDir(rustdocRoot);
    if (!haveRustdoc && strict) {
      throw new Error(
        `[rustdoc-links] compiled rustdoc not found at ${rustdocRoot}. ` +
          `Production builds require it to resolve \`rust:\` links; run ` +
          `\`cargo doc --no-deps --all-features\` and copy/symlink \`target/doc\` there ` +
          `(see build_docs.bash).`
      );
    }

    const visit = (node) => {
      const url = node && typeof node.url === "string" ? node.url : null;
      if (
        (node.type === "link" || node.type === "definition") &&
        url &&
        url.startsWith(SCHEME)
      ) {
        const spec = url.slice(SCHEME.length);
        if (!haveRustdoc) {
          if (!warnedMissingRustdoc) {
            warnedMissingRustdoc = true;
            console.warn(
              `[rustdoc-links] ${rustdocRoot} not found; ` +
                `\`rust:\` links will point to rustdoc search as a fallback. ` +
                `To resolve them properly, run \`cargo doc --no-deps --all-features\` ` +
                `and symlink \`target/doc\` to \`docs/static/rustdoc\`.`
            );
          }
          node.url = dummyUrl(spec);
        } else {
          try {
            node.url = resolveRustdocLink(spec, rustdocRoot);
          } catch (err) {
            if (!(err instanceof RustdocLinkError)) throw err;
            const message = `[rustdoc-links] in ${file.path || "unknown file"}: ${err.message}`;
            if (strict) {
              throw new Error(message);
            }
            console.warn(message);
            node.url = dummyUrl(spec);
          }
        }
      }
      if (node.children) {
        for (const child of node.children) visit(child);
      }
    };
    visit(tree);
  };
}

module.exports = remarkRustdocLinks;
module.exports.resolveRustdocLink = resolveRustdocLink;
module.exports.RustdocLinkError = RustdocLinkError;
module.exports.RUSTDOC_ROOT = RUSTDOC_ROOT;
