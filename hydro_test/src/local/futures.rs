use std::time::Duration;

use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::prelude::*;
use stageleft::q;

pub fn unordered<'a>(process: &Process<'a>) -> Stream<u32, Process<'a>, Unbounded, NoOrder> {
    process
        .source_iter(q!([2, 3, 1, 9, 6, 5, 4, 7, 8]))
        .map(q!(|x| async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            x
        }))
        .resolve_futures()
}

pub fn ordered<'a>(process: &Process<'a>) -> Stream<u32, Process<'a>, Unbounded> {
    process
        .source_iter(q!([2, 3, 1, 9, 6, 5, 4, 7, 8]))
        .map(q!(|x| async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            x
        }))
        .resolve_futures_ordered()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use futures::StreamExt;
    use hydro_deploy::Deployment;

    #[tokio::test]
    async fn test_unordered() {
        let mut deployment = Deployment::new();

        let builder = hydro_lang::builder::FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process();

        let out = super::unordered(&p1);
        let out_recv = out.send_bincode_external(&external);

        let built = builder.with_default_optimize();
        let nodes = built
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let out_recv = nodes.connect_source_bincode(out_recv).await;

        deployment.start().await.unwrap();

        let result: HashSet<u32> = out_recv
            .take(9)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect();
        assert_eq!(HashSet::from_iter(1..10), result);
    }

    #[tokio::test]
    async fn test_ordered() {
        let mut deployment = Deployment::new();

        let builder = hydro_lang::builder::FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process();

        let out = super::ordered(&p1);
        let out_recv = out.send_bincode_external(&external);

        let built = builder.with_default_optimize();
        let nodes = built
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let out_recv = nodes.connect_source_bincode(out_recv).await;

        deployment.start().await.unwrap();

        assert_eq!(
            out_recv.take(9).collect::<Vec<_>>().await,
            &[2, 3, 1, 9, 6, 5, 4, 7, 8]
        );
    }
}
