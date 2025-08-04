TASKS:
- see if we can simplify the hyperedge lift/ground logic wrt collapse state at the external end of the hyperedge!
- Review VisState API encapsulation
- DRY, clean up, check encapsulation of any index structure modifications

COMPLETED:
✅ Layout change menu functionality - all ELK algorithms supported (MRTree default)
✅ Collapsed container dimensions fix - properly uses SIZES constants
✅ JSON parsing cleanup - removed duplicate/unused JSONLoader and EnhancedJSONLoader, unified on core/JSONParser.ts

FIXS:
- collapse all after initialization
- re-init after changing group by
- node count on collapsed containers