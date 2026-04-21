use std::collections::HashSet;

use dfir_rs::dfir_syntax;
use dfir_rs::util::collect_ready_async;
use multiplatform_test::multiplatform_test;
use tokio::time::{Duration, sleep};

#[multiplatform_test(dfir, env_tracing)]
async fn single_batch_test() {
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<u32>();

    let mut df = dfir_syntax! {
        source_iter(0..10)
        -> map(|x| async move {
            println!("Producing {}", x);
            sleep(Duration::from_millis(100)).await;
            println!("Produced {}", x);
            x
        })
        -> resolve_futures_blocking()
        -> for_each(|x| result_send.send(x).unwrap());
    };

    df.run_tick().await;
    assert_eq!(
        HashSet::from_iter([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
        collect_ready_async::<HashSet<_>, _>(&mut result_recv).await
    );
}
