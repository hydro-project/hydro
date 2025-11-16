use hydro_lang::{location::{Location, NoTick}, prelude::*};

/// A simple counter that increments infinitely using a cyclic flow.
/// Returns a stream of counter values.
pub fn counter<'a, L: Location<'a> + NoTick>(
    tick: &Tick<L>,
) -> Singleton<usize, Tick<L>, Bounded> {
    // Create a cycle to hold the current counter value
    let (counter_cycle, counter_value) = tick.cycle_with_initial(tick.singleton(q!(0)));

    // Increment the counter
    let next_value = counter_value
        .map(q!(|count| count + 1));

    // Complete the cycle with the incremented value for the next tick
    counter_cycle.complete_next_tick(next_value.clone());

    // Return the stream of counter values (convert singleton to stream)
    next_value
}
