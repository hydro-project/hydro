use dfir_rs::datalog;

fn main() {
    let mut df = datalog!(r#"
        .input in1 `source_iter(0..10) -> map(|x| (x, x))`
        .input in2 `source_iter(0..10) -> map(|_| ("string",))`
        .output out `null::<(u32,)>()`
        out(a) :- in1(a, b), in2
    "#);
    df.run_available();
}
