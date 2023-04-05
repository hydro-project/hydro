use hydroflow::{
    hydroflow_syntax,
    util::cli::{ConnectedBidi, ConnectedMux, ConnectedSource},
};

#[tokio::main]
async fn main() {
    let mut ports = hydroflow::util::cli::init().await;
    let echo_recv = ports
        .remove("echo")
        .unwrap()
        .connect::<ConnectedMux<ConnectedBidi>>()
        .await
        .into_source();

    let df = hydroflow_syntax! {
        source_stream(echo_recv) ->
            map(|x| {
                let x = x.unwrap();
                (x.0, String::from_utf8(x.1.to_vec()).unwrap())
            }) ->
            for_each(|x| println!("echo {:?}", x));
    };

    hydroflow::util::cli::launch_flow(df).await;
}
