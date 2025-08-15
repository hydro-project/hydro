#github-pull-request_copilot-coding-agent

I'd like you to work through the following tasks:
1. Examine the ELKBridge and ReactFlowBridge. Make sure they are DRY and stateless. Make sure they do nothing but format translations, and no interesting business logic. Any business logic should be handled inside `core`, likely in `VisualizationState`.
2. The VisualizationState object has a "deprecated" API and an `adapter.ts` file. It's time to clean that up and have all the callers use the "official" API, and remove the deprecated API, adapters, and fix all the callsites.

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