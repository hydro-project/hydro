//! Test 20: Multiple downstream consumers borrowing the same singleton.
//! Simulates tee on a singleton reference.
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    let mut fold_state: i64 = 0;

    let mut tick = async move |input: &[i64]| -> bool {
        // Stratum 0
        for &v in input { fold_state += v; }

        // Multiple borrows of the same singleton
        let ref1: &i64 = &fold_state;
        let ref2: &i64 = &fold_state;

        // Stratum 1a: consumer A
        {
            println!("consumer A: fold = {}", ref1);
        }
        // Stratum 1b: consumer B (same stratum, different subgraph)
        {
            println!("consumer B: fold = {}", ref2);
        }

        true
    };

    tick(&[1, 2, 3]).await;
    tick(&[10]).await;
}
