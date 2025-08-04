TASKS:
- Get rid of hyperedge mentions everywhere except in VisState.js and its tests.
- fix redundant VisState.js and VisualizationState.js
- unit test to ensure that changes in ELK layout are reflected in VisState
- unit test to ensure that changes at the UI (collapse/expand) are reflected in VisState
- Review VisState API encapsulation
- DRY, clean up, check encapsulation of any index structure modifications


FIXS:
- expand container visual logic
- collapse all after initialization
- re-init after changing group by