//! Test 33: The no_std version of the Slab — backed by a fixed array.
//! Same interface, no heap allocation at all.
//! RESULT: ✅ PASS

use std::mem::MaybeUninit;
use std::ptr;

/// Fixed-capacity slab for no_std. Stores N elements of type T.
/// Between ticks, len is 0 (no live values). The backing array persists.
/// 
/// Unlike the heap Slab, this one CAN be generic over T because for no_std
/// we know the type at compile time (the proc macro generates it).
/// But the key question is: can it hold T = &'tick Something?
///
/// Answer: NO if it's captured. YES if it's tick-local.
/// So for no_std, the fixed array must be tick-local (stack-allocated each tick).
/// The "slab" concept doesn't help for no_std — fixed arrays are already free to create.
///
/// This test confirms: for no_std, just use tick-local arrays directly.
/// The Slab abstraction is only useful for std (heap allocation reuse).

#[tokio::main]
async fn main() {
    let mut fold_state: i64 = 0;
    let mut persist_buf: [i64; 16] = [0; 16];
    let mut persist_len: usize = 0;

    let mut tick = async move |input: &[i64]| -> bool {
        // no_std: tick-local fixed arrays. Zero cost. Can hold refs.
        let mut stream_hoff: [MaybeUninit<i64>; 16] = [const { MaybeUninit::uninit() }; 16];
        let mut stream_len: usize = 0;
        let mut ref_hoff: [Option<&i64>; 16] = [None; 16];

        // Stratum 0
        {
            for &val in input {
                fold_state += val;
                persist_buf[persist_len] = val;
                persist_len += 1;
                stream_hoff[stream_len] = MaybeUninit::new(val);
                stream_len += 1;
            }
            for i in 0..persist_len {
                ref_hoff[i] = Some(&persist_buf[i]);
            }
        }

        let fold_ref = &fold_state;

        // Stratum 1
        {
            print!("stream (fold={}): ", fold_ref);
            for i in 0..stream_len {
                let val = unsafe { stream_hoff[i].assume_init() };
                print!("{} ", val);
            }
            println!();
            print!("refs: ");
            for slot in ref_hoff.iter().flatten() {
                print!("{} ", slot);
            }
            println!();
        }

        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[4]).await;
}
