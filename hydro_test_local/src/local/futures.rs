use std::time::Duration;

use hydro_lang::deploy::SingleProcessGraph;
use hydro_lang::dfir_rs::scheduled::graph::Dfir;
use hydro_lang::*;
use stageleft::{Quoted, RuntimeData, q};
use tokio::sync::mpsc::UnboundedSender;

#[stageleft::entry]
pub fn unordered<'a>(
    flow: FlowBuilder<'a>,
    output: RuntimeData<&'a UnboundedSender<u32>>,
) -> impl Quoted<'a, Dfir<'a>> {
    let process = flow.process::<()>();

    process
        .source_iter(q!([2, 3, 1, 9, 6, 5, 4, 7, 8]))
        .map(q!(|x| async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            x
        }))
        .resolve_futures()
        .for_each(q!(|x| output.send(x).unwrap()));

    flow.compile_no_network::<SingleProcessGraph>()
}

#[stageleft::entry]
pub fn ordered<'a>(
    flow: FlowBuilder<'a>,
    output: RuntimeData<&'a UnboundedSender<u32>>,
) -> impl Quoted<'a, Dfir<'a>> {
    let process = flow.process::<()>();

    process
        .source_iter(q!([2, 3, 1, 9, 6, 5, 4, 7, 8]))
        .map(q!(|x| async move {
            // tokio::time::sleep works, import then just sleep does not, unsure why
            tokio::time::sleep(Duration::from_millis(10)).await;
            x
        }))
        .resolve_futures_ordered()
        .for_each(q!(|x| output.send(x).unwrap()));

    flow.compile_no_network::<SingleProcessGraph>()
}

#[cfg(stageleft_runtime)]
#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::time::Duration;

    use dfir_rs::util::collect_ready_async;

    #[tokio::test]
    async fn test_unordered() {
        let (out, mut out_recv) = dfir_rs::util::unbounded_channel();

        let mut flow = super::unordered!(&out);
        let handle = tokio::task::spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            assert_eq!(
                HashSet::from_iter(1..10),
                collect_ready_async::<HashSet<_>, _>(&mut out_recv).await
            );
        });

        tokio::time::timeout(Duration::from_secs(2), flow.run_async())
            .await
            .expect_err("Expected time out");

        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_ordered() {
        let (out, mut out_recv) = dfir_rs::util::unbounded_channel();

        let mut flow = super::ordered!(&out);
        let handle = tokio::task::spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            assert_eq!(
                Vec::from_iter([2, 3, 1, 9, 6, 5, 4, 7, 8]),
                collect_ready_async::<Vec<_>, _>(&mut out_recv).await
            );
        });

        tokio::time::timeout(Duration::from_secs(2), flow.run_async())
            .await
            .expect_err("Expected time out");

        handle.await.unwrap();
    }
}
