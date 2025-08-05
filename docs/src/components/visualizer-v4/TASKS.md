TASKS:
- see if we can simplify the hyperedge lift/ground logic wrt collapse state at the external end of the hyperedge!
- Review VisState API encapsulation
- DRY, clean up, check encapsulation of any index structure modifications
- write tests that check/maintain the statelessness of FlowGraph and the bridges.
- Why are there .js and .js.map files in this folder?
- Build Rust/TS loader for big files

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
✅ ELK nested container recursion fix - critical fix for node positioning
    - Root cause: updateContainerFromELK() was calling updateNodeFromELK() for ALL children
    - Fix: Added proper container vs node detection in recursive processing
    - Now correctly processes: containers → updateContainerFromELK(), leaf nodes → updateNodeFromELK()
    - Result: All nodes now get proper positions from ELK instead of defaulting to (0,0)
✅ InfoPanel hierarchy tree synchronization with visualization state
    - Replaced local React collapsedContainers state with VisualizationState single source of truth
    - InfoPanel now reads collapse state via visualizationState.getContainerCollapsed()
    - handleToggleContainer() calls visualizationState.collapseContainer()/expandContainer()
    - Two-way sync: InfoPanel tree reflects current state AND controls visualization
    - Matches Visualizer functionality for unified container control interface
✅ Container label display fixes for proper hierarchy names
    - Fixed ReactFlowBridge to use container.data?.label || container.label || container.id
    - Fixed InfoPanel hierarchy tree to use proper labels from container data
    - Container names now show actual function names instead of "bt_x" internal IDs
    - Added collapsedContainers dependency to InfoPanel useMemo for proper tree updates
✅ Fixed method name mismatches in container expand/collapse
    - Fixed ContainerCollapseExpand.ts to call getParentContainer() instead of getNodeContainer()
    - Fixed FlowGraph.tsx to use allManualPositions getter instead of getAllManualPositions()
    - Updated ContainerHierarchyView interface to use getParentContainer for consistency
✅ VisState missing method implementations
    - Added missing _addEdgeToNodeMapping() for edge-to-node relationship tracking
    - Added missing _removeEdgeFromNodeMapping() for cleanup
    - Added missing _updateExpandedContainers() for visibility management
    - Fixed Object.assign prototype conflicts (allManualPositions getter)
    - All JSON parsing and state management now works correctly

FIXS:
- collapse all after initialization
- node count on collapsed containers