import BenchDemo

/-! `lake exe bench [nClients ckFreq durationMs]` — the benchmark (`BenchDemo.lean`). -/

def main (args : List String) : IO UInt32 := do
  BenchDemo.exitFlushing (← BenchDemo.benchMain args)
