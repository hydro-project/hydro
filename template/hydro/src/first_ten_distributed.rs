use hydro_lang::*;

pub struct P1 {}
pub struct P2 {}

pub fn first_ten_distributed<'a>(p1: &Process<'a, P1>, p2: &Process<'a, P2>) {
    p1.source_iter(q!(0..10)) // : Stream<i32, Process<P1>, ...>
        .send_bincode(p2) // : Stream<i32, Process<P2>, ...>
        .for_each(q!(|n| println!("{}", n)));
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use hydro_deploy::Deployment;
    use hydro_lang::deploy::DeployCrateWrapper;
    use tokio_stream::wrappers::UnboundedReceiverStream;

    #[tokio::test]
    async fn first_ten_distributed() {
        let mut deployment = Deployment::new();
        let localhost = deployment.Localhost();

        let flow = hydro_lang::FlowBuilder::new();
        let p1 = flow.process();
        let p2 = flow.process();
        super::first_ten_distributed(&p1, &p2);

        let nodes = flow
            .with_process(&p1, localhost.clone())
            .with_process(&p2, localhost.clone())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let second_process_stdout = nodes.get_process(&p2).stdout().await;

        deployment.start().await.unwrap();

        assert_eq!(
            UnboundedReceiverStream::new(second_process_stdout)
                .take(10)
                .collect::<Vec<_>>()
                .await,
            vec!["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"]
        );
    }
}
