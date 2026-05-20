//! Test 2: Store &T reference in a local variable within one tick call.
//! Simulates: fold produces value, downstream borrows it via a local binding.
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    let mut fold_state: i64 = 0;

    let mut tick = async move |_: &mut ()| -> bool {
        // Stratum 0: mutate
        fold_state += 1;

        // "singleton slot" - local to this call
        let singleton_ref: &i64 = &fold_state;

        // Stratum 1: consume the reference
        {
            println!("via ref: {}", singleton_ref);
        }
        true
    };

    tick(&mut ()).await;
    tick(&mut ()).await;
}
