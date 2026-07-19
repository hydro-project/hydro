# KVS Agent Instructions

## Architecture Documentation

Read [`ARCHITECTURE.md`](./ARCHITECTURE.md) before modifying this crate. It
describes the current topology, request flow, consistency model, reusable
combinators, deployment boundary, assumptions, and limitations.

Keep `ARCHITECTURE.md` synchronized with the implementation. Any code change
that affects the crate's architecture or behavior must include the
corresponding documentation update in the same revision. This includes changes
to:

- protocol types or client-visible semantics;
- router, storage, sidecar, or deployment responsibilities;
- request routing, replication, quorum collection, or state management;
- consistency guarantees, membership assumptions, or failure behavior;
- persistence, recovery, networking, or external I/O;
- testing strategy or source-file responsibilities.

For every code change, explicitly review `ARCHITECTURE.md` even when no update
appears necessary. Document the behavior that exists in the code, distinguish
guarantees from assumptions, and do not describe planned behavior as already
implemented.
