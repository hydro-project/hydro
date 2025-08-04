TASKS:
- see if we can simplify the hyperedge lift/ground logic wrt collapse state at the external end of the hyperedge!
- Review VisState API encapsulation
- DRY, clean up, check encapsulation of any index structure modifications
- write tests that check/maintain the statelessness of FlowGraph and the bridges.
- write tests that check/maintain the statelessness of FlowGraph and the bridges.

COMPLETED:
✅ Layout change menu functionality - all ELK algorithms supported (MRTree default)
✅ Collapsed container dimensions fix - properly uses SIZES constants
✅ JSON parsing cleanup - removed duplicate/unused JSONLoader and EnhancedJSONLoader, unified on core/JSONParser.ts
✅ Container drag functionality fix - removed forced re-renders that broke ReactFlow drag state
✅ Clean file restoration - restored vis.js from git history, applied only necessary changes
✅ Synchronous resetAll approach - clean reinitialization with fresh FlowGraph/ELK/ReactFlow instances
✅ Architectural fix: Manual positions moved to VisualizationState for clean resets and proper encapsulation
✅ ReactFlow nested container drag fix - removed extent: 'parent' that caused coordinate system corruption
    - Discovered by comparing with working Visualizer implementation
    - The Visualizer team had identified and fixed this exact issue
    - Location grouping worked (no parentId → no extent) vs Backtrace grouping failed (parentId → extent: 'parent')
    - Phase 1 Fix: Remove extent: 'parent' from ReactFlowBridge.ts, keep parentId for visual hierarchy
    - Phase 2 Fix: Add elk.hierarchyHandling: 'INCLUDE_CHILDREN' to maintain proper nesting in ELK layout
✅ Layout switching functionality - fixed effect dependencies to prevent interference with initial render
✅ ELK layout spacing improvements - ported Visualizer's proven spacing configuration
    - NODE_TO_NODE_NORMAL: 20→75 (better node separation)
    - CONTAINER_PADDING: 15→60 (proper breathing room)
    - COMPONENT_TO_COMPONENT: 30→60 (better container separation)
    - Added comprehensive ELK spacing options matching Visualizer

FIXS:
- collapse all after initialization
- node count on collapsed containers