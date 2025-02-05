// Test harness for the various implementations of shopping carts.

use clap::{Parser, ValueEnum};
use dfir_lang::graph::{WriteConfig, WriteGraphType};
use driver::run_driver;

mod driver;
mod flows;
mod lattices;
mod structs;
mod test_data;
mod wrappers;

#[derive(Clone, ValueEnum, Debug)]
enum Role {
    Client,
    Server,
}

#[derive(Parser, Debug)]
struct Opts {
    #[clap(long)]
    opt: usize,
    #[clap(long)]
    graph: Option<WriteGraphType>,
    #[clap(flatten)]
    write_config: Option<WriteConfig>,
}

#[dfir_rs::main]
async fn main() {
    let opts = Opts::parse();

    // all the interesting logic is in the driver
    run_driver(opts).await;
}

#[test]
fn test() {
    use dfir_rs::util::{run_cargo_example, wait_for_process_output};

    fn escape_regex(input: &str) -> String {
        input
            .replace("(", "\\(")
            .replace(")", "\\)")
            .replace("{", "\\{")
            .replace("}", "\\}")
            .replace("[", "\\[")
            .replace("]", "\\]")
    }

    {
        let (_child, _, mut stdout) = run_cargo_example("shopping", "--opt 1");

        let mut output = String::new();
        for line in OPT1_OUTPUT.split("\n") {
            wait_for_process_output(&mut output, &mut stdout, &escape_regex(line));
        }
    }

    {
        let (_child, _, mut stdout) = run_cargo_example("shopping", "--opt 2");

        let mut output = String::new();
        for line in OPT2_OUTPUT.split("\n") {
            wait_for_process_output(&mut output, &mut stdout, &escape_regex(line));
        }
    }

    {
        let (_child, _, mut stdout) = run_cargo_example("shopping", "--opt 3");

        let mut output = String::new();
        for line in OPT3_OUTPUT.split("\n") {
            wait_for_process_output(&mut output, &mut stdout, &escape_regex(line));
        }
    }

    {
        let (_child, _, mut stdout) = run_cargo_example("shopping", "--opt 4");

        let mut output = String::new();
        for line in OPT4_OUTPUT.split("\n") {
            wait_for_process_output(&mut output, &mut stdout, &escape_regex(line));
        }
    }

    {
        let (_child, _, mut stdout) = run_cargo_example("shopping", "--opt 5");

        let mut output = String::new();
        for line in OPT5_OUTPUT.split("\n") {
            wait_for_process_output(&mut output, &mut stdout, &escape_regex(line));
        }
    }

    {
        let (_child, _, mut stdout) = run_cargo_example("shopping", "--opt 6");

        let mut output = String::new();
        for line in OPT6_OUTPUT.split("\n") {
            wait_for_process_output(&mut output, &mut stdout, &escape_regex(line));
        }
    }

    {
        let (_child, _, mut stdout) = run_cargo_example("shopping", "--opt 7");

        let mut output = String::new();
        for line in OPT7_OUTPUT.split("\n") {
            wait_for_process_output(&mut output, &mut stdout, &escape_regex(line));
        }
    }
}

#[cfg(test)]
const OPT1_OUTPUT: &str = r#"
((2, Basic), [LineItem { name: "apple", qty: 1 }, LineItem { name: "apple", qty: -1 }, LineItem { name: "", qty: 0 }])
((1, Basic), [LineItem { name: "apple", qty: 1 }, LineItem { name: "banana", qty: 6 }, LineItem { name: "", qty: 0 }])
((100, Prime), [LineItem { name: "potato", qty: 1 }, LineItem { name: "ferrari", qty: 1 }, LineItem { name: "", qty: 0 }])
"#;

#[cfg(test)]
const OPT2_OUTPUT: &str = r#"
((2, Basic), BoundedPrefix { vec: [ClLineItem { client: 2, li: LineItem { name: "apple", qty: 1 } }, ClLineItem { client: 2, li: LineItem { name: "apple", qty: -1 } }, Checkout { client: 2 }], len: Some(3) })
((1, Basic), BoundedPrefix { vec: [ClLineItem { client: 1, li: LineItem { name: "apple", qty: 1 } }, ClLineItem { client: 1, li: LineItem { name: "banana", qty: 6 } }, Checkout { client: 1 }], len: Some(3) })
((100, Prime), BoundedPrefix { vec: [ClLineItem { client: 100, li: LineItem { name: "potato", qty: 1 } }, ClLineItem { client: 100, li: LineItem { name: "ferrari", qty: 1 } }, Checkout { client: 100 }], len: Some(3) })
"#;

#[cfg(test)]
const OPT3_OUTPUT: &str = r#"
((2, Basic), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 2, li: LineItem { name: "apple", qty: 1 } }, 1: ClLineItem { client: 2, li: LineItem { name: "apple", qty: -1 } }}, len: Some(3) })
((1, Basic), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 1, li: LineItem { name: "apple", qty: 1 } }, 1: ClLineItem { client: 1, li: LineItem { name: "banana", qty: 6 } }}, len: Some(3) })
((100, Prime), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 100, li: LineItem { name: "potato", qty: 1 } }, 1: ClLineItem { client: 100, li: LineItem { name: "ferrari", qty: 1 } }}, len: Some(3) })
"#;

#[cfg(test)]
const OPT4_OUTPUT: &str = r#"
((1, Basic), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 1, li: LineItem { name: "apple", qty: 1 } }, 1: ClLineItem { client: 1, li: LineItem { name: "banana", qty: 6 } }}, len: Some(3) })
((2, Basic), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 2, li: LineItem { name: "apple", qty: 1 } }, 1: ClLineItem { client: 2, li: LineItem { name: "apple", qty: -1 } }}, len: Some(3) })
((100, Prime), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 100, li: LineItem { name: "potato", qty: 1 } }, 1: ClLineItem { client: 100, li: LineItem { name: "ferrari", qty: 1 } }}, len: Some(3) })
"#;

#[cfg(test)]
const OPT5_OUTPUT: &str = r#"
((100, Prime), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 100, li: LineItem { name: "potato", qty: 1 } }, 1: ClLineItem { client: 100, li: LineItem { name: "ferrari", qty: 1 } }}, len: Some(3) })
((1, Basic), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 1, li: LineItem { name: "apple", qty: 1 } }, 1: ClLineItem { client: 1, li: LineItem { name: "banana", qty: 6 } }}, len: Some(3) })
((2, Basic), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 2, li: LineItem { name: "apple", qty: 1 } }, 1: ClLineItem { client: 2, li: LineItem { name: "apple", qty: -1 } }}, len: Some(3) })
"#;

#[cfg(test)]
const OPT6_OUTPUT: &str = r#"
((100, Prime), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 100, li: LineItem { name: "potato", qty: 1 } }, 1: ClLineItem { client: 100, li: LineItem { name: "ferrari", qty: 1 } }}, len: Some(3) })
((1, Basic), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 1, li: LineItem { name: "apple", qty: 1 } }, 1: ClLineItem { client: 1, li: LineItem { name: "banana", qty: 6 } }}, len: Some(3) })
((2, Basic), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 2, li: LineItem { name: "apple", qty: 1 } }, 1: ClLineItem { client: 2, li: LineItem { name: "apple", qty: -1 } }}, len: Some(3) })
"#;

#[cfg(test)]
const OPT7_OUTPUT: &str = r#"
((2, Basic), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 2, li: LineItem { name: "apple", qty: 1 } }, 1: ClLineItem { client: 2, li: LineItem { name: "apple", qty: -1 } }}, len: Some(3) })
((1, Basic), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 1, li: LineItem { name: "apple", qty: 1 } }, 1: ClLineItem { client: 1, li: LineItem { name: "banana", qty: 6 } }}, len: Some(3) })
((100, Prime), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 100, li: LineItem { name: "potato", qty: 1 } }, 1: ClLineItem { client: 100, li: LineItem { name: "ferrari", qty: 1 } }}, len: Some(3) })
((2, Basic), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 2, li: LineItem { name: "apple", qty: 1 } }, 1: ClLineItem { client: 2, li: LineItem { name: "apple", qty: -1 } }}, len: Some(3) })
((2, Basic), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 2, li: LineItem { name: "apple", qty: 1 } }, 1: ClLineItem { client: 2, li: LineItem { name: "apple", qty: -1 } }}, len: Some(3) })
((1, Basic), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 1, li: LineItem { name: "apple", qty: 1 } }, 1: ClLineItem { client: 1, li: LineItem { name: "banana", qty: 6 } }}, len: Some(3) })
((1, Basic), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 1, li: LineItem { name: "apple", qty: 1 } }, 1: ClLineItem { client: 1, li: LineItem { name: "banana", qty: 6 } }}, len: Some(3) })
((100, Prime), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 100, li: LineItem { name: "potato", qty: 1 } }, 1: ClLineItem { client: 100, li: LineItem { name: "ferrari", qty: 1 } }}, len: Some(3) })
((100, Prime), SealedSetOfIndexedValues { set: {0: ClLineItem { client: 100, li: LineItem { name: "potato", qty: 1 } }, 1: ClLineItem { client: 100, li: LineItem { name: "ferrari", qty: 1 } }}, len: Some(3) })
"#;
