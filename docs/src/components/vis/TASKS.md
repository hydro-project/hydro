TASKS:
- unit test to ensure that changes in ELK layout are reflected in VisState
- unit test to ensure that changes at the UI (collapse/expand) are reflected in VisState
- unit test to ensure that VisState is self-consistent wrt collapses:
   - a container has 1 or 0 visible representations (expanded, collapsed, or both hidden)
   - a visible item must have visible parents
- Add the controls from `visualizer`
- remove any custom logic for rendering and always use ReactFlow built-ins
- remove any custom logic for layout and always use ELK
- unit tests for layout, rendering, and the conversion between
- DRY, clean up, check encapsulation of any index structure modifications


FIXS:
- expand container visual logic
- collapse all after initialization
- re-init after changing group by