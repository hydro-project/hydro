use hydro_deploy::Deployment;

#[tokio::main]
async fn main() {
    let mut deployment = Deployment::new();

    let flow = hydro_lang::FlowBuilder::new();
    let p1 = flow.process();
    let p2 = flow.process();
    hydro_template::first_ten_distributed::first_ten_distributed(&p1, &p2);

    let _nodes = flow
        .with_process(&p1, deployment.Localhost())
        .with_process(&p2, deployment.Localhost())
        .deploy(&mut deployment);

    deployment.run_ctrl_c().await.unwrap();
}
