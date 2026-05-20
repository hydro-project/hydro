//! Test 37: bumpalo with async { }.await subgraph blocks.
//! Does the 'bump lifetime survive across await points?

use bumpalo::Bump;

#[tokio::main]
async fn main() {
    let mut fold_state: i64 = 0;
    let mut persist_buf: Vec<i64> = Vec::new();
    let mut bump = Bump::new();

    let mut tick = async move |input: &[i64]| -> bool {
        bump.reset();

        let mut stream_hoff: bumpalo::collections::Vec<i64> = bumpalo::vec![in &bump; ];
        let mut ref_hoff: bumpalo::collections::Vec<&i64> = bumpalo::vec![in &bump; ];

        // Stratum 0 (async block)
        async {
            for &val in input {
                fold_state += val;
                persist_buf.push(val);
                stream_hoff.push(val);
            }
            for item in persist_buf.iter() {
                ref_hoff.push(item);
            }
        }.await;

        let fold_ref: &i64 = &fold_state;

        // Stratum 1 (async block)
        async {
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
        }.await;

        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[10]).await;
}
