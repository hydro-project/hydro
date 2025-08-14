TASKS:
- DRY, clean up, check encapsulation of any index structure modifications
- write tests that check/maintain the statelessness of FlowGraph and the bridges.
- Search/Graph Filtering/Focus: in treeview?
- Centralize any stray constants
- Put all relevant styling constant into a dockable config widget

FIXS:
- I want a comprehensive fuzz test of the entire visualizer and all its controls.
    - I'd like you to start this by combining the "DISCONNECTED EDGES BUG HUNTER" with what's in fuzz_test.ts, continuing to use `paxos-flipped.json
    - Here are moves we'd like to randomly take
        - expand node
        - contract node
        - expand/contract via HierarchyTree
        - expandAll
        - contractAll
        - pick a new hierarchy as done via infoPanel
        - change layouts as done via menu
        - fit to viewport as done via menu
        - autofit on vs off as done via menu
- remove redundant tests?
- Tests should not regex on error messages, they should look for error IDs. Is there a TS best practice for this?
- remaining deprecated APIs in VisState? setGraph for example?