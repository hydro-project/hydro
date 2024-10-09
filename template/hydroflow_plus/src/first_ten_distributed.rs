use hydroflow_plus::*;
use stageleft::*;

pub struct P1 {}
pub struct P2 {}

pub fn first_ten_distributed<'a>(flow: &FlowBuilder<'a>) -> (Process<'a, P1>, Process<'a, P2>) {
    let process = flow.process::<P1>();
    let second_process = flow.process::<P2>();

    let numbers = process.source_iter(q!(0..10));
    numbers
        .send_bincode(&second_process)
        .for_each(q!(|n| println!("{}", n)));

    (process, second_process)
}

#[cfg(test)]
mod tests {
    use hydro_deploy::Deployment;
    use hydroflow_plus::deploy::{DeployCrateWrapper, TrybuildHost};
    use hydroflow_plus::futures::StreamExt;
    use tokio_stream::wrappers::UnboundedReceiverStream;

    #[tokio::test]
    async fn first_ten_distributed() {
        let mut deployment = Deployment::new();
        let localhost = deployment.Localhost();

        let flow = hydroflow_plus::FlowBuilder::new();
        let (p1, p2) = super::first_ten_distributed(&flow);

        let nodes = flow
            .with_default_optimize()
            .with_process(&p1, TrybuildHost::new(localhost.clone()))
            .with_process(&p2, TrybuildHost::new(localhost.clone()))
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
