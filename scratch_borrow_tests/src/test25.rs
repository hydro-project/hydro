//! Test 25: No handoff at all — producer writes to captured buffer,
//! consumer borrows a slice of it. The "handoff" is just knowing the range.
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    let mut persist_buf: [i64; 16] = [0; 16];
    let mut persist_len: usize = 0;

    let mut tick = async move |input: &[i64]| -> bool {
        // Stratum 0: write into captured buffer
        let start = persist_len;
        {
            for &val in input {
                persist_buf[persist_len] = val;
                persist_len += 1;
            }
        }
        let end = persist_len;

        // Stratum 1: borrow the slice that was just written
        // (plus all historical data)
        {
            let new_items: &[i64] = &persist_buf[start..end];
            let all_items: &[i64] = &persist_buf[..end];
            println!("new: {:?}", new_items);
            println!("all: {:?}", all_items);
        }

        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[4, 5]).await;
}
