//! Test 13: Stream of references through a TICK-LOCAL Vec handoff.
//! persist() collects items into a captured Vec, then downstream iterates &T.
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    // Cross-tick: persisted collection
    let mut persist_buf: Vec<i64> = Vec::new();

    let mut tick = async move |input: &[i64]| -> bool {
        // Stratum 0: persist (append to cross-tick buffer)
        {
            persist_buf.extend_from_slice(input);
        }

        // Tick-local handoff carrying references to persist_buf
        let ref_handoff: Vec<&i64> = persist_buf.iter().collect();

        // Stratum 1: process references
        {
            for r in ref_handoff.iter() {
                println!("ref item: {}", r);
            }
        }

        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[4, 5]).await;
}
