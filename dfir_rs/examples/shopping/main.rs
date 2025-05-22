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
    use example_test::run_current_example;

    fn escape_regex(input: &str) -> String {
        input
            .replace("(", "\\(")
            .replace(")", "\\)")
            .replace("{", "\\{")
            .replace("}", "\\}")
            .replace("[", "\\[")
            .replace("]", "\\]")
    }

    let opts_outputs = [
        ("--opt=1", OPT1_OUTPUT),
        ("--opt=2", OPT2_OUTPUT),
        ("--opt=3", OPT3_OUTPUT),
        ("--opt=4", OPT4_OUTPUT),
        ("--opt=5", OPT5_OUTPUT),
        ("--opt=6", OPT6_OUTPUT),
        ("--opt=7", OPT7_OUTPUT),
    ];

    for (opt, expected_output) in opts_outputs {
        let mut proc = run_current_example!([opt]);
        for line in expected_output.split('\n') {
            if !line.trim().is_empty() {
                proc.wait_for_output(&escape_regex(line));
            }
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
