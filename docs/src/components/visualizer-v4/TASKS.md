TASKS:
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

FIXS:
- change naming: "aggregate" -> "hyperEdge"
- make sure that padded container dimensions are the only dimensions visible to the outside, and that the API for getting containers is small and doesn't support multiple ways of getting containers and/or their dimensions
- _handleContainerExpansion: is it the inverse of _handleContainerCollapse?