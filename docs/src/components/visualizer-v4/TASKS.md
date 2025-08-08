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
- Here's what we get when I do a re-grouping of the paxos graph by backtrace. Do you see how the big purple container is too large? 

One possible issue is that we only track 2 choices of dimensions for a container
- fully expanded
- fully collapsed

Maybe we need to consider what happens to nodes higher in the hierarchy with collapse of nodes low in the hierarchy?
