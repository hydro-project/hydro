use dfir_rs::dfir_syntax;

pub fn main() {
    // Create our channel input
    let (input_example, example_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut flow = dfir_syntax! {
         source_stream(example_recv)
        -> filter_map(|n: usize| {
            let n2 = n * n;
            if n2 > 10 {
                Some(n2)
            }
            else {
                None
            }
        })
        -> flat_map(|n| n..=n+1)
        -> for_each(|n| println!("Ahoy, {}", n));
    };

    println!("A");
    input_example.send(1).unwrap();
    input_example.send(0).unwrap();
    input_example.send(2).unwrap();
    input_example.send(3).unwrap();
    input_example.send(4).unwrap();
    input_example.send(5).unwrap();
    flow.run_available_sync();

    println!("B");
    input_example.send(6).unwrap();
    input_example.send(7).unwrap();
    input_example.send(8).unwrap();
    input_example.send(9).unwrap();
    flow.run_available_sync();
}
