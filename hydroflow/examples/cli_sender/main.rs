use hydroflow::{hydroflow_syntax, util::serialize_to_bytes};

#[tokio::main]
async fn main() {
    let mut ports = hydroflow::util::cli::init().await;
    let foo_send = ports.remove("foo").unwrap().1.unwrap();

    let mut df = hydroflow_syntax! {
        foo = map(serialize_to_bytes::<String>) -> dest_sink(foo_send);

        repeat_iter([
            "Hello".to_string(),
            "World".to_string(),
        ]) -> foo;
    };

    println!("hello from the sender!");

    df.run_async().await;
}
