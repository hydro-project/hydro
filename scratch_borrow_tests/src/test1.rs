//! Test 1: async move || with direct borrow of captured state across plain blocks.
//! Question: Can stratum 1 borrow fold_state that stratum 0 mutated?
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    let mut fold_state: i64 = 0;
    let input_buf: Vec<i64> = vec![1, 2, 3];

    let mut tick = async move |_: &mut ()| -> bool {
        // Stratum 0: mutate fold_state
        {
            for item in input_buf.iter() {
                fold_state += item;
            }
        }
        // Stratum 1: borrow fold_state immutably
        {
            let r: &i64 = &fold_state;
            println!("fold_state = {}", r);
        }
        true
    };

    tick(&mut ()).await;
    tick(&mut ()).await;
    println!("done");
}
