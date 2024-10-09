use std::io::Write;

#[hydroflow::main]
async fn main() {
    let _ = hydroflow::util::deploy::init::<()>().await;
    println!("hello!");

    std::io::stdout().flush().unwrap();

    panic!();
}
