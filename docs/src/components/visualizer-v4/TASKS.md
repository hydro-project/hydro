TASKS:
I'd like you to work through the following tasks:
1. Examine the ELKBridge and ReactFlowBridge. Make sure they are DRY and stateless. Make sure they do nothing but format translations, and no interesting business logic. Any business logic should be handled inside `core`, likely in `VisualizationState`.
2. The VisualizationState object has a "deprecated" API and an `adapter.ts` file. It's time to clean that up and have all the callers use the "official" API, and remove the deprecated API, adapters, and fix all the callsites.

REMAINING TASKS FOR LATER:
- Review VisState API encapsulation
- DRY, clean up, check encapsulation of any index structure modifications
- write tests that check/maintain the statelessness of FlowGraph and the bridges.
- Build Rust/TS loader for big files
- Search in treeview
- Graph Filtering/Focus
- Centralize any stray constants
- Consolidate validation functions in VisState.js
- clean up config.js and constants.js
- Put all relevant styling constant into a dockable config widget

### Remaining Issues to Fix 🔧
- **Test Optimization**: 18 tests are skipped - could be optimized to run if needed
- **Performance Tuning**: Further optimization opportunities for large dataset handling

FIXS:
- 🔄 PARTIALLY COMPLETE: remove "legacy API" and "compatibility methods" from VisState
- Edges are shifted north of nodes. Perhaps due to padding for the node labels?
- Fix remaining hyperEdge preservation during container expansion (4 failing tests)
- change naming: "aggregate" -> "hyperEdge"
- make sure that padded container dimensions are the only dimensions visible to the outside, and that the API for getting containers is small and doesn't support multiple ways of getting containers and/or their dimensions