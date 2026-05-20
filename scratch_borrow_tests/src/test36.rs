//! Test 36: bumpalo as the tick-scoped allocator.
//! Bump is captured across ticks. Each tick, reset() and allocate fresh Vecs.
//! Question: does the 'bump lifetime work with async move ||?

use bumpalo::Bump;

#[tokio::main]
async fn main() {
    let mut fold_state: i64 = 0;
    let mut persist_buf: Vec<i64> = Vec::new();

    // Captured across ticks — backing memory reused
    let mut bump = Bump::new();

    let mut tick = async move |input: &[i64]| -> bool {
        // Reset arena — O(1), all previous allocations invalidated
        bump.reset();

        // Allocate tick-local Vecs from the bump
        let mut stream_hoff = bumpalo::vec![in &bump; ];
        let mut ref_hoff: bumpalo::collections::Vec<&i64> = bumpalo::vec![in &bump; ];

        // Stratum 0
        {
            for &val in input {
                fold_state += val;
                persist_buf.push(val);
                stream_hoff.push(val);
            }
            for item in persist_buf.iter() {
                ref_hoff.push(item);
            }
        }

        // Singleton ref
        let fold_ref: &i64 = &fold_state;

        // Stratum 1
        {
            print!("stream (fold={}): ", fold_ref);
            for val in stream_hoff.drain(..) {
                print!("{} ", val);
            }
            println!();
            print!("refs: ");
            for r in ref_hoff.iter() {
                print!("{} ", r);
            }
            println!();
        }

        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[10]).await;
}
