TASKS:
- Review VisState API encapsulation
- DRY, clean up, check encapsulation of any index structure modifications
- write tests that check/maintain the statelessness of FlowGraph and the bridges.
- Build Rust/TS loader for big files
- Search in treeview
- Graph Filtering/Focus
- Centralize any stray constants
- Put all relevant styling constant into a dockable config widget

FIXS:
- ~~paxos-flipped init~~ ✅ **FIXED** - Added comprehensive dimension explosion prevention with auto-collapse logic
- Edges are shifted north of nodes. Perhaps due to padding for the node labels?
