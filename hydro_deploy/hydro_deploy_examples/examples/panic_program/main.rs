use std::io::Write;

#[dfir_rs::main]
async fn main() {
    let _ = dfir_rs::util::deploy::init::<()>().await;
    println!("hello!");

    std::io::stdout().flush().unwrap();

    panic!();
}
