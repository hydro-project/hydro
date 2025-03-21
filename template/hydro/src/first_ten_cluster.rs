use hydro_lang::*;

pub struct Leader {}
pub struct Worker {}

pub fn first_ten_cluster<'a>(leader: &Process<'a, Leader>, workers: &Cluster<'a, Worker>) {
    leader
        .source_iter(q!(0..10)) // : Stream<i32, Process<Leader>, ...>
        .round_robin_bincode(workers) // : Stream<i32, Cluster<Worker>, ...>
        .map(q!(|n| n * 2)) // : Stream<i32, Cluster<Worker>, ...>
        .inspect(q!(|n| println!("{}", n))) // : Stream<i32, Cluster<Worker>, ...>
        .send_bincode_anonymous(leader) // : Stream<i32, Process<Leader>, ...>
        .for_each(q!(|n| println!("{}", n)));
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use hydro_deploy::Deployment;
    use hydro_lang::deploy::DeployCrateWrapper;
    use tokio_stream::wrappers::UnboundedReceiverStream;

    #[tokio::test]
    async fn first_ten_cluster() {
        let mut deployment = Deployment::new();
        let localhost = deployment.Localhost();

        let flow = hydro_lang::FlowBuilder::new();
        let leader = flow.process();
        let workers = flow.cluster();
        super::first_ten_cluster(&leader, &workers);

        let nodes = flow
            .with_process(&leader, localhost.clone())
            .with_cluster(&workers, vec![localhost.clone(); 4])
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let leader_stdout = nodes.get_process(&leader).stdout().await;

        deployment.start().await.unwrap();

        let mut out = UnboundedReceiverStream::new(leader_stdout)
            .take(10)
            .collect::<Vec<_>>()
            .await;
        out.sort();

        let mut expected = vec!["0", "2", "4", "6", "8", "10", "12", "14", "16", "18"];
        expected.sort();

        assert_eq!(out, expected);
    }
}
