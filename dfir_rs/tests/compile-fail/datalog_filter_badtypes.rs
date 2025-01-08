use dfir_rs::datalog;

#[derive(Clone, PartialEq, Eq, Hash)]
struct Uncomparable;

fn main() {
    let mut df = datalog!(r#"
        .input in1 `source_iter(0..10) -> map(|_| (Uncomparable {},))`
        .output out `null::<(u32,)>()`
        out(123) :- in1(a), (a > a)
    "#);
    df.run_available();
}
