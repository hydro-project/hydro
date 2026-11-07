/-!
# Durations

The signature's clock readings and timer periods are naturals in **microseconds** — the
one unit `time`'s readings, `timeout`'s duration and `source_interval`'s delay and
period share. The Rust spells its durations through `Duration::from_secs`/`from_millis`
(`PaxosConfig`'s timeouts are seconds, `paxos_bench`'s intervals milliseconds); the
programs mirror those calls with these, so the unit conversion is in the program text
where the Rust's is, and a shell's clock is a microsecond clock.
-/

namespace Hydro.Duration

/-- Rust `Duration::from_secs(s)`, in microseconds. -/
def from_secs (s : Nat) : Nat := s * 1000000

/-- Rust `Duration::from_millis(ms)`, in microseconds. -/
def from_millis (ms : Nat) : Nat := ms * 1000

end Hydro.Duration
