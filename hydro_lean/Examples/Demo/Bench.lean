import Examples.Demo.BenchDemo

/-! `lake exe bench [nClients ckFreq durationMs]` — the benchmark (`Examples/Demo/BenchDemo.lean`). -/

def main (args : List String) : IO UInt32 := do
  BenchDemo.exitFlushing (← BenchDemo.benchMain args)
