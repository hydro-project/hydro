#![expect(unused_mut, unused_variables, reason = "example code")]

use dfir_rs::dfir_syntax_inline;

fn main() {
    let mut flow = dfir_syntax_inline! {
        // DFIR syntax goes here
    };
}
