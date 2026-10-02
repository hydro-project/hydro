import HydroV2.HydroRel

/-!
# HydroV2 · the generated relational op laws

This file is one command: everything it adds to the environment —
`HRel.law_<op>` per operation, the bundle `HRel.Laws`, the per-op
accessors, and the op-head dispatch registry — is derived from the
`HydroSem` class declaration by `hydro_rel_laws` (`HydroRel.lean`).
Adding an operation to the signature and rebuilding regenerates the
laws; every `HRel.Laws` instance then fails to compile until it
supplies the new obligation.
-/

namespace HydroV2

hydro_rel_laws

end HydroV2
