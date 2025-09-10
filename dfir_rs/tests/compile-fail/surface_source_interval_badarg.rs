use dfir_rs::dfir_syntax;

#[dfir_rs::main]
async fn main() {
    let mut df = dfir_syntax! {
        // Should be a `Duration`.
        source_interval(5) -> for_each(std::mem::drop);
    };
    df.run().await;
}
