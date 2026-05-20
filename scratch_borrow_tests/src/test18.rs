//! Test 18: Direct iteration of &T from captured buffer (no intermediate ref handoff).
//! This is the simplest model: persist_buf is captured, downstream just borrows it.
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    let mut persist_buf: [i64; 16] = [0; 16];
    let mut persist_len: usize = 0;
    let mut fold_state: i64 = 0;

    let mut tick = async move |input: &[i64]| -> bool {
        // Stratum 0: accumulate
        {
            for &val in input {
                persist_buf[persist_len] = val;
                persist_len += 1;
                fold_state += val;
            }
        }

        // Stratum 1: directly borrow captured persist_buf (no handoff needed!)
        {
            let fold_ref = &fold_state;
            let slice = &persist_buf[..persist_len];
            for item in slice {
                println!("item={}, fold={}", item, fold_ref);
            }
        }

        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[4]).await;
}
