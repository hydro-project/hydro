fn main() {
    let mut df = hydroflow::hydroflow_syntax! {
        source_iter(0..10) -> next_loop() -> null();
    };
    df.run_available();
}
