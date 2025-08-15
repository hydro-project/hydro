TASKS:
- DRY, clean up, check encapsulation of any index structure modifications
- write tests that check/maintain the statelessness of FlowGraph and the bridges.
- Search/Graph Filtering/Focus: in treeview?
- Centralize any stray constants
- Put all relevant styling constant into a dockable config widget

FIXS:
- clean up smartCollapse and validateHyperEdgeLifting
- remove redundant tests?
- Tests should not regex on error messages, they should look for error IDs. Is there a TS best practice for this?
- remaining deprecated APIs in VisState? setGraph for example?