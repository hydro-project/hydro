# Known Issues

## Sim Runtime Cannot Drive Multi-Cluster E2E Dataflows

**Status:** Partially resolved; one blocker remains

The distributed KVS (`kvs_core`) cannot be end-to-end tested in the Hydro simulator. All component-level logic is tested in isolation (47 sim/unit tests passing), and full E2E coverage uses Docker integration tests.

### Resolved: Cluster membership race condition (was issue #2)

`round_robin` previously panicked with division-by-zero when the sim scheduled command processing before cluster membership was populated (`data.0 % members.len()` with `members.len() == 0`). Fixed by replacing the unchecked `map` with `filter_map` that returns `None` when members is empty.

Commands arriving before membership is populated are now silently dropped instead of crashing. This is correct behavior — the sim will retry on the next tick when membership is available.

### Remaining blocker: `sliced!` blocks panic on empty `forward_ref` streams

`use(stream, nondet!())` inside `sliced!` requires the stream to have at least one item or a previously released value. On the first tick, streams from `forward_ref` cycles (the write-phase put channel and the rebalancing channel) are empty, causing:

```
thread panicked at hydro_lang/src/sim/runtime.rs:535:
No input and no last released item to re-release
```

This is a SIGABRT (not a hang) that kills the entire test process. It affects `kvs_core` because it uses two `forward_ref` cycles:
1. Write-phase commands: router reads existing VC, then sends ClockedPut back to nodes
2. Rebalancing: nodes redistribute keys when membership changes

Both cycles start with empty streams that trigger the panic.

### What would fix this

The sim runtime's `NonDet` decision points should handle the case where a `use(...)` stream has no data and no previous state. Options:

- **Allow empty `use(...)` to produce an empty batch** instead of panicking (treat "no input and no last released item" as "yield nothing this tick")
- **Initialize `forward_ref` streams with an empty sentinel** so there's always a "last released item" to re-release

### Workaround

Test components in isolation using `Process`-level sim tests (no cluster communication needed), and use Docker integration tests for full E2E coverage. The extracted pure helper functions (`build_node_response`, `rendezvous_targets`, `classify_merged_response`, `build_dominating_clocked_put`, `split_client_command`) can be unit tested without the sim runtime at all.
