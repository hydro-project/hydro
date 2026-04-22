use dfir_rs::dfir_syntax;

fn main() {
    let (output_send, mut output_recv) = dfir_rs::util::unbounded_channel::<char>();
    let mut flow = dfir_syntax! {
        source_iter("Hello World".chars()) -> map(|c| c.to_ascii_uppercase())
            -> for_each(|c| output_send.send(c).unwrap());
    };
    flow.run_available_sync();

    let output = &*dfir_rs::util::collect_ready::<String, _>(&mut output_recv);
    assert_eq!(output, "HELLO WORLD");
}
