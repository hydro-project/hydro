fn main() {
    let (_send, recv) = dfir_rs::util::unbounded_channel::<i32>();
    let mut flow = dfir_rs::dfir_syntax_inline! {
        x = source_stream(recv);
        loop {
            x -> batch() -> for_each(|_x: i32| {});
        };
    };
}
