# coord-criterion branch: outstanding issues

This branch is shelved pending a static broadcast PR. Rebase on top of that PR
before resuming work here.

## 1. Dynamic broadcast to Cluster is unsound (blocking)

The analysis classifies `Network` from a different location into a `Cluster` as
`Source` (discharged) in `classify()`, and the propagation layer doesn't model
membership dynamics. This produces false CONV/SEQ labels for programs that are
actually weaker:

- **NoOrder/SetInclusion goal → should be SELF, not CONV.** A late-joining
  Cluster member sees a subset of what an always-present member sees. That's
  future-monotone per-member (set inclusion holds locally), but members don't
  converge to the same set.

- **Prefix/TotalOrder goal → should be INCON.** A late joiner misses a prefix,
  so its observation is not a prefix of the full sequence.

**Fix:** A separate PR will introduce a static broadcast mechanism (e.g.,
`StaticCluster` location type or `static_broadcast` vs `dynamic_broadcast` at
the network layer). Once that lands, rebase this branch and:

- For static broadcast to a Cluster: keep current `Source` classification
  (all members present for entire execution, same data).
- For dynamic broadcast to a Cluster: classify as SELF under SetInclusion,
  INCON under Prefix.
- Remove the `label_fixed_membership` / `consistency_fixed` dual-label
  mechanism, which was a workaround for this issue.

## 2. `goal_overrides` keyed by source spans (minor)

Sink identifiers use `"name@file:line:col"` format (e.g.,
`"sendexternal@src/plumbing.rs:73:20"`). These break on any refactor that moves
the sink call. Consider user-provided labels as a more stable alternative.

## 3. `viz/render.rs` refactor — is it used? (minor)

`build_hydro_graph_structure` was extracted as a public function but doesn't
appear to be called by the coordination analysis. Confirm whether this is for
the future viz overlay mentioned in COORDINATION.md or if it's dead code.

## 4. Remaining review items not yet discussed

These were identified in the initial review but we didn't get to them in
discussion. They may be fine as-is or may need attention:

- `SinkResult` and `CoordinationReport` are `pub` — confirm these need to be
  public API (CoordinationReport is returned by `check_coordination()` so yes,
  but SinkResult fields could potentially be `pub(crate)`).

- `HYDRO_CHECK_COORDINATION` env var in `builder.rs::finalize()` prints to
  stderr unconditionally when set. Fine for development, but consider whether
  this should use a structured logging mechanism before merging.

- The inline codegen fixes (`dfir_lang/src/graph/mod.rs`, `meta_graph.rs`) are
  bug fixes to the inline path unrelated to coordination. Could be split into
  a separate small PR to land independently, but not blocking.
