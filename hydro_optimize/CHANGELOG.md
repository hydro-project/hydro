

## v0.13.0 (2025-07-31)

<csr-id-edab6c2b94b08ff2204411409e840fcc43112609/>
<csr-id-22a7d0d9a057356a4a01ccdad6febc8aa229d325/>
<csr-id-fb016b017a42df95286cb2236ef987ea68be9ea7/>

### Chore

 - <csr-id-edab6c2b94b08ff2204411409e840fcc43112609/> clean up dependencies
   Using `hydro_optimize` as a regular dependency in `hydro_test` results
   in leaking many dependencies including `hydro_deploy`, so this moves it
   to a dev-dependency

### New Features

 - <csr-id-0e6403cc21c89ae397828a84b4908204c9f4dba5/> improve logging for profiling
 - <csr-id-173d9c0f1cc955f2a478195bcf3de0b055284008/> Use partitioning analysis results to partition
   Test/insta changes stem from changed implementation of
   broadcast_bincode, will change again once #1949 is implemented.
   Also added missing cases for Persists hidden behind CrossProduct,
   Difference, AntiJoin, Join, and Scan for decoupler.
 - <csr-id-dfaf51776923485e73a52b01c36ebb06d2cc6957/> Remove commercial ilp
 - <csr-id-3b013ac3520a984cd5faf882b78f1c2dfd4f12eb/> Partitioning analysis
   Integrating with the partitioner next
 - <csr-id-d44b225f697f8d7ccdf80b0d5dc2afdea1955324/> capture stack traces for each IR node
   Because Hydro is staged, the stack traces capture the structure of the
   program, which is helpful for profiling / visualization.
 - <csr-id-c4b9590552d7e05b352a0d4a215f1bbf274ccc61/> add `scan` operator
 - <csr-id-99a8f1dfdde087578f25c19e502ad13e1d98a394/> Decoupling analysis
   A Gurobi license is required to run code that uses `hydro_optimize` (for ILP over decoupling decisions)

### Bug Fixes

 - <csr-id-739b622f7a6ee3e58289863519419bd925c1e788/> don't snapshot-test backtraces
   Backtraces aren't stable across Unix / Windows. Just have a separate
   test for them.
   
   Also defers resolution of backtraces until we actually need them to
   improve performance.

### Refactor

 - <csr-id-22a7d0d9a057356a4a01ccdad6febc8aa229d325/> separate externals from other location kinds, clean up network operators
   First, we remove externals from `LocationId`, to ensure that a
   `LocationId` only is used for locations where we can concretely place
   compiled logic. This simplifes a lot of pattern matching where we wanted
   to disallow externals.
   
   Keys on network inputs / outputs (`from_key` and `to_key`) are only
   relevant to external networking. We extract the logic to instantiate
   external network collections, so that the core logic does not need to
   deal with keys.

### Refactor (BREAKING)

 - <csr-id-fb016b017a42df95286cb2236ef987ea68be9ea7/> invert external sources and clean up locations in IR
   First, instead of creating external sources by invoking
   `external.source_bincode_external(&p)`, we switch the API to
   `p.source_bincode_external(&external)` for symmetry with `source_iter`
   and `source_stream`.
   
   The other, much larger change is to clean up how the IR handles external
   inputs and outputs and keeps track of locations. First, we introduce
   `HydroNode::ExternalInput` and `HydroLeaf::SendExternal` as specialized
   nodes for these, so that we no longer create dummy sources / sinks.
   
   Then, we eliminate places where we have multiple sources of truth for
   where the output of an IR node is located, by instead referring to the
   metadata. Because it is easy in optimizer rewrites to corrupt this
   metadata, we also add a flag to `transform_bottom_up` that lets
   developers enable a metadata validity check. We disable it in most
   transformations for performance, but enable it in the decoupling
   rewrites since it manipulates locations in complex ways.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 12 commits contributed to the release over the course of 12 calendar days.
 - 11 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 11 unique issues were worked on: [#1859](https://github.com/hydro-project/hydro/issues/1859), [#1930](https://github.com/hydro-project/hydro/issues/1930), [#1934](https://github.com/hydro-project/hydro/issues/1934), [#1935](https://github.com/hydro-project/hydro/issues/1935), [#1937](https://github.com/hydro-project/hydro/issues/1937), [#1940](https://github.com/hydro-project/hydro/issues/1940), [#1947](https://github.com/hydro-project/hydro/issues/1947), [#1952](https://github.com/hydro-project/hydro/issues/1952), [#1955](https://github.com/hydro-project/hydro/issues/1955), [#1958](https://github.com/hydro-project/hydro/issues/1958), [#1962](https://github.com/hydro-project/hydro/issues/1962)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1859](https://github.com/hydro-project/hydro/issues/1859)**
    - Decoupling analysis ([`99a8f1d`](https://github.com/hydro-project/hydro/commit/99a8f1dfdde087578f25c19e502ad13e1d98a394))
 * **[#1930](https://github.com/hydro-project/hydro/issues/1930)**
    - Add `scan` operator ([`c4b9590`](https://github.com/hydro-project/hydro/commit/c4b9590552d7e05b352a0d4a215f1bbf274ccc61))
 * **[#1934](https://github.com/hydro-project/hydro/issues/1934)**
    - Clean up dependencies ([`edab6c2`](https://github.com/hydro-project/hydro/commit/edab6c2b94b08ff2204411409e840fcc43112609))
 * **[#1935](https://github.com/hydro-project/hydro/issues/1935)**
    - Partitioning analysis ([`3b013ac`](https://github.com/hydro-project/hydro/commit/3b013ac3520a984cd5faf882b78f1c2dfd4f12eb))
 * **[#1937](https://github.com/hydro-project/hydro/issues/1937)**
    - Capture stack traces for each IR node ([`d44b225`](https://github.com/hydro-project/hydro/commit/d44b225f697f8d7ccdf80b0d5dc2afdea1955324))
 * **[#1940](https://github.com/hydro-project/hydro/issues/1940)**
    - Remove commercial ilp ([`dfaf517`](https://github.com/hydro-project/hydro/commit/dfaf51776923485e73a52b01c36ebb06d2cc6957))
 * **[#1947](https://github.com/hydro-project/hydro/issues/1947)**
    - Don't snapshot-test backtraces ([`739b622`](https://github.com/hydro-project/hydro/commit/739b622f7a6ee3e58289863519419bd925c1e788))
 * **[#1952](https://github.com/hydro-project/hydro/issues/1952)**
    - Use partitioning analysis results to partition ([`173d9c0`](https://github.com/hydro-project/hydro/commit/173d9c0f1cc955f2a478195bcf3de0b055284008))
 * **[#1955](https://github.com/hydro-project/hydro/issues/1955)**
    - Improve logging for profiling ([`0e6403c`](https://github.com/hydro-project/hydro/commit/0e6403cc21c89ae397828a84b4908204c9f4dba5))
 * **[#1958](https://github.com/hydro-project/hydro/issues/1958)**
    - Invert external sources and clean up locations in IR ([`fb016b0`](https://github.com/hydro-project/hydro/commit/fb016b017a42df95286cb2236ef987ea68be9ea7))
 * **[#1962](https://github.com/hydro-project/hydro/issues/1962)**
    - Separate externals from other location kinds, clean up network operators ([`22a7d0d`](https://github.com/hydro-project/hydro/commit/22a7d0d9a057356a4a01ccdad6febc8aa229d325))
 * **Uncategorized**
    - Release dfir_lang v0.14.0, dfir_macro v0.14.0, hydro_deploy_integration v0.14.0, lattices_macro v0.5.10, variadics_macro v0.6.1, dfir_rs v0.14.0, hydro_deploy v0.14.0, hydro_lang v0.14.0, hydro_optimize v0.13.0, hydro_std v0.14.0, safety bump 6 crates ([`0683595`](https://github.com/hydro-project/hydro/commit/06835950c12884d661100c13f73ad23a98bfad9f))
</details>

